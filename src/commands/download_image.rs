use fs_err as fs;

use image::{codecs::png::PngEncoder, imageops::resize, DynamicImage, GenericImageView};
use log::{debug, info};

use std::borrow::Cow;

use crate::{
    alpha_bleed::alpha_bleed,
    auth_cookie::get_auth_cookie,
    options::{GlobalOptions, DownloadImageOptions},
    roblox_api::{get_preferred_client, ImageUploadData, RobloxCredentials},
};

pub async fn download_image(
    global: GlobalOptions,
    options: DownloadImageOptions,
) -> anyhow::Result<()> {
    let client = get_preferred_client(RobloxCredentials {
        token: global.auth.or_else(get_auth_cookie),
        api_key: None,
        user_id: None,
        group_id: None,
    })?;

    let response = client.download_image(options.asset_id).await?;
    fs::write(options.output, response)?;
    
    Ok(())
}
