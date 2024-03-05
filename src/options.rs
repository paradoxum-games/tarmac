use crate::commands::Command;
use clap::Parser;
use secrecy::SecretString;

#[derive(Debug, Parser)]
#[clap(about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Options {
    #[command(flatten)]
    pub global: Global,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub struct Global {
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
