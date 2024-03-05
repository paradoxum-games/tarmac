mod asset_list;
mod create_cache_map;
mod download_image;
mod sync;
mod upload_image;

pub use asset_list::*;
use clap::Subcommand;
pub use create_cache_map::*;
pub use download_image::*;
pub use sync::*;
pub use upload_image::*;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Upload a single image to the Roblox cloud. Prints the asset ID of the
    /// resulting Image asset to stdout.
    UploadImage(UploadImageOptions),

    /// Sync your Tarmac project, uploading any assets that have changed.
    Sync(SyncOptions),

    /// Downloads any packed spritesheets, then generates a file mapping asset
    /// IDs to file paths. This command only works when logged into Roblox
    /// Studio or when a .ROBLOSECURITY token is passed via --auth.
    CreateCacheMap(CreateCacheMapOptions),

    /// Creates a file that lists all assets required by the project.
    AssetList(AssetListOptions),

    /// Downloads a single image from the Roblox cloud.
    DownloadImage(DownloadImageOptions),
}
