use std::{
    borrow::Cow,
    fmt::{self, Write},
};

use rbxcloud::rbx::assets::{AssetCreator, AssetGroupCreator, AssetUserCreator};
use reqwest::{
    header::{HeaderValue, COOKIE},
    Client, Request, Response, StatusCode,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::auth_cookie::get_csrf_token;

#[derive(Debug, Clone)]
pub struct ImageUploadData<'a> {
    pub image_data: Cow<'a, [u8]>,
    pub name: &'a str,
    pub description: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub asset_id: u64,
    pub backing_asset_id: u64,
}

/// Internal representation of what the asset upload endpoint returns, before
/// we've handled any errors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawUploadResponse {
    success: bool,
    message: Option<String>,
    asset_id: Option<u64>,
    backing_asset_id: Option<u64>,
}

pub struct RobloxApiClient {
    pub creator: Option<AssetCreator>,

    csrf_token: Option<HeaderValue>,
    credentials: RobloxCredentials,
    client: Client,
}

#[derive(Debug)]
pub struct RobloxCredentials {
    pub token: Option<SecretString>,
    pub api_key: Option<SecretString>,
    pub user_id: Option<u64>,
    pub group_id: Option<u64>,
}

impl fmt::Debug for RobloxApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "RobloxApiClient")
    }
}

impl RobloxApiClient {
    pub fn new(credentials: RobloxCredentials) -> Result<Self, RobloxApiError> {
        if credentials.api_key.is_none() && credentials.token.is_none() {
            return Err(RobloxApiError::MissingAuth);
        }

        let csrf_token = if let Some(token) = &credentials.token {
            match get_csrf_token(&token) {
                Ok(value) => Some(value),
                Err(err) => {
                    log::error!("Was unable to fetch CSRF token: {}", err.to_string());
                    None
                }
            }
        } else {
            None
        };

        let creator = match (
            &credentials.api_key,
            credentials.group_id,
            credentials.user_id,
        ) {
            (_, Some(id), None) => Some(AssetCreator::Group(AssetGroupCreator {
                group_id: id.to_string(),
            })),

            (api_key, None, Some(id)) => {
                if api_key.is_none() {
                    log::warn!("{}", "A user ID was specified, but no API key was specified.

Tarmac will attempt to upload to the currently logged-in user or to the user associated with the token given in --auth.

If you mean to use the Open Cloud API, make sure to provide an API key!
")
                }

                Some(AssetCreator::User(AssetUserCreator {
                    user_id: id.to_string(),
                }))
            }

            (Some(_), None, None) => return Err(RobloxApiError::ApiKeyNeedsCreatorId),

            (_, Some(_), Some(_)) => return Err(RobloxApiError::AmbiguousCreatorType),

            (None, None, None) => None,
        };

        Ok(Self {
            csrf_token,
            creator,
            credentials,
            client: Client::new(),
        })
    }

    pub fn download_image(&mut self, id: u64) -> Result<Vec<u8>, RobloxApiError> {
        let url = format!("https://roblox.com/asset?id={}", id);

        let mut response =
            self.execute_with_csrf_retry(|client| Ok(client.get(&url).build()?))?;

        let mut buffer = Vec::new();
        response.copy_to(&mut buffer)?;

        Ok(buffer)
    }

    /// Upload an image, retrying if the asset endpoint determines that the
    /// asset's name is inappropriate. The asset's name will be replaced with a
    /// generic known-good string.
    pub fn upload_image_with_moderation_retry(
        &mut self,
        data: ImageUploadData,
    ) -> Result<UploadResponse, RobloxApiError> {
        let response = self.upload_image_raw(&data)?;

        // Some other errors will be reported inside the response, even
        // though we received a successful HTTP response.
        if response.success {
            let asset_id = response.asset_id.unwrap();
            let backing_asset_id = response.backing_asset_id.unwrap();

            Ok(UploadResponse {
                asset_id,
                backing_asset_id,
            })
        } else {
            let message = response.message.unwrap();

            // There are no status codes for this API, so we pattern match
            // on the returned error message.
            //
            // If the error message text mentions something being
            // inappropriate, we assume the title was problematic and
            // attempt to re-upload.
            if message.contains("inappropriate") {
                log::warn!(
                    "Image name '{}' was moderated, retrying with different name...",
                    data.name
                );

                let new_data = ImageUploadData {
                    name: "image",
                    ..data
                };

                self.upload_image(new_data)
            } else {
                Err(RobloxApiError::ApiError { message })
            }
        }
    }

    /// Upload an image, returning an error if anything goes wrong.
    pub fn upload_image(
        &mut self,
        data: ImageUploadData,
    ) -> Result<UploadResponse, RobloxApiError> {
        let response = self.upload_image_raw(&data)?;

        // Some other errors will be reported inside the response, even
        // though we received a successful HTTP response.
        if response.success {
            let asset_id = response.asset_id.unwrap();
            let backing_asset_id = response.backing_asset_id.unwrap();

            Ok(UploadResponse {
                asset_id,
                backing_asset_id,
            })
        } else {
            let message = response.message.unwrap();

            Err(RobloxApiError::ApiError { message })
        }
    }

    /// Upload an image, returning the raw response returned by the endpoint,
    /// which may have further failures to handle.
    fn upload_image_raw(
        &mut self,
        data: &ImageUploadData,
    ) -> Result<RawUploadResponse, RobloxApiError> {
        let mut url = "https://data.roblox.com/data/upload/json?assetTypeId=13".to_owned();

        if let Some(AssetCreator::Group(AssetGroupCreator { group_id })) = &self.creator {
            write!(url, "&groupId={}", group_id).unwrap();
        }

        let mut response = self.execute_with_csrf_retry(|client| {
            Ok(client
                .post(&url)
                .query(&[("name", data.name), ("description", data.description)])
                .body(data.image_data.clone().into_owned())
                .build()?)
        })?;

        let body = response.text()?;

        // Some errors will be reported through HTTP status codes, handled here.
        if response.status().is_success() {
            match serde_json::from_str(&body) {
                Ok(response) => Ok(response),
                Err(source) => Err(RobloxApiError::BadResponseJson { body, source }),
            }
        } else {
            Err(RobloxApiError::ResponseError {
                status: response.status(),
                body,
            })
        }
    }

    /// Execute a request generated by the given function, retrying if the
    /// endpoint requests that the user refreshes their CSRF token.
    fn execute_with_csrf_retry<F>(&mut self, make_request: F) -> Result<Response, RobloxApiError>
    where
        F: Fn(&Client) -> Result<Request, RobloxApiError>,
    {
        let mut request = make_request(&self.client)?;
        self.attach_headers(&mut request);

        let response = self.client.execute(request)?;

        match response.status() {
            StatusCode::FORBIDDEN => {
                if let Some(csrf) = response.headers().get("X-CSRF-Token") {
                    log::debug!("Retrying request with X-CSRF-Token...");

                    self.csrf_token = Some(csrf.clone());

                    let mut new_request = make_request(&self.client)?;
                    self.attach_headers(&mut new_request);

                    Ok(self.client.execute(new_request)?)
                } else {
                    // If the response did not return a CSRF token for us to
                    // retry with, this request was likely forbidden for other
                    // reasons.

                    Ok(response)
                }
            }
            _ => Ok(response),
        }
    }

    /// Attach required headers to a request object before sending it to a
    /// Roblox API, like authentication and CSRF protection.
    fn attach_headers(&self, request: &mut Request) {
        if let Some(auth_token) = &self.credentials.token {
            let cookie_value = format!(".ROBLOSECURITY={}", auth_token.expose_secret());

            request.headers_mut().insert(
                COOKIE,
                HeaderValue::from_bytes(cookie_value.as_bytes()).unwrap(),
            );
        }

        if let Some(csrf) = &self.csrf_token {
            request.headers_mut().insert("X-CSRF-Token", csrf.clone());
        }
    }
}

#[derive(Debug, Error)]
pub enum RobloxApiError {
    #[error("Roblox API HTTP error")]
    Http {
        #[from]
        source: reqwest::Error,
    },

    #[error("Roblox API error: {message}")]
    ApiError { message: String },

    #[error("Roblox API returned success, but had malformed JSON response: {body}")]
    BadResponseJson {
        body: String,
        source: serde_json::Error,
    },

    #[error("Roblox API returned HTTP {status} with body: {body}")]
    ResponseError { status: StatusCode, body: String },

    #[error("Request for CSRF token did not return an X-CSRF-Token header.")]
    MissingCsrfToken,

    #[error("Failed to retrieve asset ID from Roblox cloud")]
    AssetGetFailed,

    #[error("Either a group or a user ID must be specified when using an API key")]
    ApiKeyNeedsCreatorId,

    #[error("Tarmac is unable to locate an authentication method")]
    MissingAuth,

    #[error("Group ID and user ID cannot both be specified")]
    AmbiguousCreatorType,
}
