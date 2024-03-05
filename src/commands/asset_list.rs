use std::collections::BTreeSet;
use std::env;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Args;
use fs_err as fs;

use anyhow::Result;

use crate::data::Manifest;
use crate::options::Global;

#[derive(Debug, Args)]
pub struct AssetListOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a file to put the asset list.
    #[clap(long = "output")]
    pub output: PathBuf,
}

pub async fn asset_list(_: Global, options: AssetListOptions) -> Result<()> {
    let project_path = match options.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let mut asset_list = BTreeSet::new();
    for input_manifest in manifest.inputs.values() {
        if let Some(id) = input_manifest.id {
            asset_list.insert(id);
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.output)?);
    for id in asset_list {
        writeln!(file, "{}", id)?;
    }
    file.flush()?;

    Ok(())
}
