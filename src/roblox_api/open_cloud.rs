use anyhow::{bail, Result};
use async_trait::async_trait;
use std::marker::PhantomData;
use std::time::Duration;

use rbxcloud::rbx::{
    assets::{
        AssetCreation, AssetCreationContext, AssetCreator, AssetGroupCreator, AssetType,
        AssetUserCreator,
    },
    error::Error as RbxCloudError,
    CreateAssetWithContents, GetAsset, RbxAssets, RbxCloud,
};
use reqwest::StatusCode;
use secrecy::ExposeSecret;

use super::{ImageUploadData, RobloxApiClient, RobloxApiError, RobloxCredentials, UploadResponse};

pub struct OpenCloudClient<'a> {
    credentials: RobloxCredentials,
    creator: AssetCreator,
    assets: RbxAssets,
    _marker: PhantomData<&'a ()>,
}

#[async_trait]
impl<'a> RobloxApiClient<'a> for OpenCloudClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self> {
        let creator = match (credentials.group_id, credentials.user_id) {
            (Some(id), None) => AssetCreator::Group(AssetGroupCreator {
                group_id: id.to_string(),
            }),
            (None, Some(id)) => AssetCreator::User(AssetUserCreator {
                user_id: id.to_string(),
            }),
            _ => unreachable!(),
        };

        let Some(api_key) = credentials.api_key.as_ref() else {
            bail!(RobloxApiError::MissingAuth);
        };

        let assets = RbxCloud::new(api_key.expose_secret()).assets();

        Ok(Self {
            creator,
            assets,
            credentials,
            _marker: PhantomData::default(),
        })
    }

    // this was a bad idea, sorry
    // async fn upload_image_with_moderation_retry(
    //     &self,
    //     data: ImageUploadData<'a>,
    // ) -> Result<UploadResponse> {
    //     match self.upload_image(data.clone()).await {
    //         Err(RobloxApiError::ResponseError { status, body })
    //             if status == 400 && body.contains("moderated") =>
    //         {
    //             log::warn!(
    //                 "Image name '{}' was moderated, retrying with different name...",
    //                 data.name
    //             );
    //             self.upload_image(ImageUploadData {
    //                 name: "image".to_string(),
    //                 ..data.to_owned()
    //             })
    //             .await
    //         }

    //         result => result,
    //     }
    // }

    async fn upload_image(&self, data: ImageUploadData<'a>) -> Result<UploadResponse> {
        self.upload_image_inner(data).await
    }

    fn download_image(&self, id: u64) -> Result<Vec<u8>> {
        todo!();
        // LegacyClient::new(self.credentials.clone())?.download_image(id)
    }
}

impl<'a> OpenCloudClient<'a> {
    async fn upload_image_inner(&self, data: ImageUploadData<'a>) -> Result<UploadResponse> {
        let asset_info = CreateAssetWithContents {
            asset: AssetCreation {
                asset_type: AssetType::DecalPng,
                display_name: data.name.to_string(),
                description: data.description.to_string(),
                creation_context: AssetCreationContext {
                    creator: self.creator.clone(),
                    expected_price: None,
                },
            },
            contents: &data.image_data,
        };

        let response = self.assets.create_with_contents(&asset_info).await?;

        let Some(operation_id) = response.path else {
            bail!(RobloxApiError::MissingOperationPath);
        };

        let Some(operation_id) = operation_id.strip_prefix("operations/") else {
            bail!(RobloxApiError::MissingOperationPath);
        };

        let operation_id = operation_id.to_string();

        const MAX_RETRIES: u32 = 5;
        const INITIAL_SLEEP_DURATION: Duration = Duration::from_millis(50);
        const BACKOFF: u32 = 2;

        let mut retry_count = 0;
        let operation = GetAsset { operation_id };
        let asset_id = async {
            loop {
                let res = self.assets.get(&operation).await?;
                let Some(response) = res.response else {
                    if retry_count > MAX_RETRIES {
                        return Err(RobloxApiError::AssetGetFailed);
                    }

                    retry_count += 1;
                    std::thread::sleep(INITIAL_SLEEP_DURATION * retry_count.pow(BACKOFF));
                    continue;
                };

                let Ok(asset_id) = response.asset_id.parse::<u64>() else {
                    return Err(RobloxApiError::AssetGetFailed);
                };

                return Ok(asset_id);
            }
        }
        .await?;

        Ok(UploadResponse {
            asset_id,
            backing_asset_id: asset_id,
        })
    }
}

impl From<RbxCloudError> for RobloxApiError {
    fn from(value: RbxCloudError) -> Self {
        match value {
            RbxCloudError::HttpStatusError { code, msg } => RobloxApiError::ResponseError {
                status: StatusCode::from_u16(code).unwrap_or_default(),
                body: msg,
            },
            _ => RobloxApiError::RbxCloud(value),
        }
    }
}
