use clap::Args;
use fs_err as fs;

use crate::{
    auth_cookie::get_auth_cookie,
    options::Global,
    roblox_api::{get_preferred_client, RobloxCredentials},
};

#[derive(Debug, Args)]
pub struct DownloadImageOptions {
    /// The path to the image to upload.
    pub asset_id: u64,

    /// The resulting path for the image asset
    #[clap(long, short)]
    pub output: String,
}

pub async fn download_image(
    global: Global,
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
