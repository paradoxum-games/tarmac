use fs_err as fs;

use image::{codecs::png::PngEncoder, imageops::resize, DynamicImage, GenericImageView};
use log::{debug, info};

use std::borrow::Cow;

use crate::{
    alpha_bleed::alpha_bleed,
    auth_cookie::get_auth_cookie,
    options::{GlobalOptions, UploadImageOptions},
    roblox_api::{get_preferred_client, ImageUploadData, RobloxCredentials},
};

pub async fn upload_image(
    global: GlobalOptions,
    options: UploadImageOptions,
) -> anyhow::Result<()> {
    let image_data = fs::read(options.path).expect("couldn't read input file");

    let mut img = match options.resize {
        Some((width, height)) => {
            let img = image::load_from_memory(&image_data).expect("couldn't load image");
            debug!("read image with dimensions {:?}, resizing to {:?}", img.dimensions(), (width, height));
            let img = resize(&img, width, height, image::imageops::FilterType::Gaussian);
            DynamicImage::ImageRgba8(img)
        },
        None => {
            image::load_from_memory(&image_data).expect("couldn't load image")
        }
    };

    alpha_bleed(&mut img);

    let (width, height) = img.dimensions();

    let mut encoded_image: Vec<u8> = Vec::new();
    PngEncoder::new(&mut encoded_image)
        .encode(&img.to_bytes(), width, height, img.color())
        .unwrap();

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
    info!("Visit https://create.roblox.com/store/asset/{} to see it", response.backing_asset_id);

    Ok(())
}
