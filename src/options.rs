use clap::{Args, Parser, Subcommand};
use secrecy::SecretString;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Options {
    #[command(flatten)]
    pub global: GlobalOptions,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub struct GlobalOptions {
    /// The authentication cookie for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the cookie from the Roblox Studio installation on
    /// the system.
    #[clap(long, global(true), conflicts_with("api_key"))]
    pub auth: Option<SecretString>,

    /// The Open Cloud API key for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the API key stored in the environment variable
    /// 'TARMAC_API_KEY'.
    #[clap(
        long,
        global(true),
        env("TARMAC_API_KEY"),
        hide_env_values(true),
        conflicts_with("auth")
    )]
    pub api_key: Option<SecretString>,

    /// Sets verbosity level. Can be specified multiple times to increase the verbosity
    /// of this program.
    #[clap(long = "verbose", short, global(true), action(clap::ArgAction::Count))]
    pub verbosity: u8,
}

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
}

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
}

#[derive(Debug, Args)]
pub struct SyncOptions {
    /// Where Tarmac should sync the project.
    ///
    /// Options:
    ///
    /// - roblox: Upload to Roblox.com
    ///
    /// - none: Do not upload. Tarmac will exit with an error if there are any
    ///   unsynced assets.
    ///
    /// - debug: Copy to local debug directory for debugging output
    ///
    /// - local: Copy to locally installed Roblox content folder.
    #[clap(long, require_equals = true, value_enum, num_args = 0..=1, default_value_t = SyncTarget::Roblox)]
    pub target: SyncTarget,

    /// When provided, Tarmac will upload again at most the given number of times
    /// when it encounters rate limitation errors.
    #[clap(long)]
    pub retry: Option<usize>,

    /// The number of seconds to wait between each re-upload attempts.
    #[clap(long, default_value = "60")]
    pub retry_delay: u64,

    /// The path to a Tarmac config, or a folder containing a Tarmac project.
    pub config_path: Option<PathBuf>,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum SyncTarget {
    Roblox,
    None,
    Debug,
    Local,
}

#[derive(Debug, Args)]
pub struct CreateCacheMapOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a directory to put any downloaded packed images.
    #[clap(long = "cache-dir")]
    pub cache_dir: PathBuf,

    /// A path to a file to contain the cache mapping.
    #[clap(long = "index-file")]
    pub index_file: PathBuf,
}

#[derive(Debug, Args)]
pub struct AssetListOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a file to put the asset list.
    #[clap(long = "output")]
    pub output: PathBuf,
}
