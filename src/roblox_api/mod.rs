mod open_cloud;

use std::borrow::Cow;

use async_trait::async_trait;
use anyhow::{bail, Result};
use rbxcloud::rbx::error::Error as RbxCloudError;
use reqwest::StatusCode;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use self::open_cloud::OpenCloudClient;

#[derive(Debug, Clone)]
pub struct ImageUploadData<'a> {
    pub image_data: Cow<'a, [u8]>,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub asset_id: u64,
    pub backing_asset_id: u64,
}

#[derive(Clone, Debug)]
pub struct RobloxCredentials {
    pub token: Option<SecretString>,
    pub api_key: Option<SecretString>,
    pub user_id: Option<u64>,
    pub group_id: Option<u64>,
}

#[async_trait]
pub trait RobloxApiClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self>
    where
        Self: Sized;

    // this was a bad idea, sorry
    // async fn upload_image_with_moderation_retry(
    //     &self,
    //     data: ImageUploadData<'a>,
    // ) -> Result<UploadResponse>;

    async fn upload_image(
        &self,
        data: ImageUploadData<'a>,
    ) -> Result<UploadResponse>;

    fn download_image(&self, id: u64) -> Result<Vec<u8>>;
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

    #[error("Operation path is missing")]
    MissingOperationPath,

    #[error("Operation path is malformed")]
    MalformedOperationPath,

    #[error("Open Cloud API error")]
    RbxCloud(RbxCloudError),

    #[error("Failed to parse asset ID from asset get response")]
    MalformedAssetId(#[from] std::num::ParseIntError),
}

pub fn get_preferred_client<'a>(
    credentials: RobloxCredentials,
) -> Result<Box<dyn RobloxApiClient<'static>>> {
    match &credentials {
        RobloxCredentials {
            token: None,
            api_key: None,
            ..
        } => bail!(RobloxApiError::MissingAuth),

        RobloxCredentials {
            group_id: Some(_),
            user_id: Some(_),
            ..
        } => bail!(RobloxApiError::AmbiguousCreatorType),

        RobloxCredentials {
            api_key: Some(_), ..
        } => Ok(Box::new(OpenCloudClient::new(credentials)?)),

        RobloxCredentials {
            token: Some(_),
            user_id,
            ..
        } => {
            if user_id.is_some() {
                log::warn!("A user ID was specified, but no API key was specified.

Tarmac will attempt to upload to the user currently logged into Roblox Studio, or to the user associated with the token given in --auth.

If you mean to use the Open Cloud API, make sure to provide an API key!")
            };

            todo!();
            // Ok(Box::new(LegacyClient::new(credentials)?))
        }
    }
}
