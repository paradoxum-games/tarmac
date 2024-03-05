use anyhow::bail;
use clap::Args;
use fs_err as fs;

use image::{codecs::png::PngEncoder, imageops::resize, DynamicImage, GenericImageView};
use log::{debug, info};

use std::{borrow::Cow, path::PathBuf};

use crate::{
    alpha_bleed::alpha_bleed,
    auth_cookie::get_auth_cookie,
    options::Global,
    roblox_api::{get_preferred_client, ImageUploadData, RobloxCredentials},
};

#[derive(Debug, Args)]
pub struct UploadImageOptions {
    /// The path to the image to upload.
    pub path: PathBuf,

    /// The name to give to the resulting Decal asset.
    #[clap(long)]
    pub name: String,

    /// The description to give to the resulting Decal asset.
    #[clap(long, default_value = "Uploaded by Tarmac.")]
    pub description: String,

    /// The ID of the user to upload to. This option only has effect when using
    /// an API key. Please note that you may only specify a group ID or a user ID.
    #[clap(
        long,
        conflicts_with("group_id"),
        requires("api_key"),
        conflicts_with("auth")
    )]
    pub user_id: Option<u64>,

    /// The ID of the group to upload to. This option only has an effect when
    /// using an API key. Please note that you may only specify a group ID or a user ID.
    #[clap(
        long,
        conflicts_with("user_id"),
        requires("api_key"),
        conflicts_with("auth")
    )]
    pub group_id: Option<u64>,

    #[clap(long, value_parser(clap::builder::ValueParser::new(parse_resize_var)))]
    pub resize: Option<(u32, u32)>,
}

fn parse_resize_var(env: &str) -> anyhow::Result<(u32, u32)> {
    if let Some((width, height)) = env
        .split_once('x')
        .map(|(w, h)| (w.parse::<u32>(), h.parse::<u32>()))
    {
        Ok((width?, height?))
    } else {
        bail!("invalid dimensions passed - please pass your dimensions in the WxH format (e.g. 100x100, 200x200, etc)")
    }
}

pub async fn upload_image(global: Global, options: UploadImageOptions) -> anyhow::Result<()> {
    let image_data = fs::read(options.path)?;

    let mut img = match options.resize {
        Some((width, height)) => {
            let img = image::load_from_memory(&image_data)?;
            debug!(
                "read image with dimensions {:?}, resizing to {:?}",
                img.dimensions(),
                (width, height)
            );
            let img = resize(&img, width, height, image::imageops::FilterType::Gaussian);
            DynamicImage::ImageRgba8(img)
        }
        None => image::load_from_memory(&image_data)?,
    };

    alpha_bleed(&mut img);

    let (width, height) = img.dimensions();

    let mut encoded_image: Vec<u8> = Vec::new();
    PngEncoder::new(&mut encoded_image).encode(&img.to_bytes(), width, height, img.color())?;

    let client = get_preferred_client(RobloxCredentials {
        token: global.auth.or_else(get_auth_cookie),
        api_key: global.api_key,
        user_id: options.user_id,
        group_id: options.group_id,
    })?;

    let upload_data = ImageUploadData {
        image_data: Cow::Owned(encoded_image.to_vec()),
        name: options.name,
        description: options.description,
    };

    let response = client.upload_image(upload_data).await?;

    info!("Image uploaded successfully!");
    info!("Asset ID: rbxassetid://{}", response.backing_asset_id);
    info!(
        "Visit https://create.roblox.com/store/asset/{} to see it",
        response.backing_asset_id
    );

    Ok(())
}
