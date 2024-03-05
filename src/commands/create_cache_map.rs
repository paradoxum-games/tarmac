use std::collections::BTreeMap;
use std::env;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;
use fs_err as fs;
use resolve_path::PathResolveExt;

use crate::asset_name::AssetName;
use crate::auth_cookie::get_auth_cookie;
use crate::data::Manifest;
use crate::options::Global;
use crate::roblox_api::{get_preferred_client, RobloxCredentials};

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

pub async fn create_cache_map(global: Global, options: CreateCacheMapOptions) -> Result<()> {
    let api_client = get_preferred_client(RobloxCredentials {
        token: global.auth.or_else(get_auth_cookie),
        api_key: None,
        user_id: None,
        group_id: None,
    })?;

    let project_path = match options.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let index_file = options.index_file.try_resolve()?;

    let Some(index_dir) = index_file.parent() else {
        bail!("missing parent directory for index file - this should never happen");
    };

    fs::create_dir_all(index_dir)?;

    fs::create_dir_all(&options.cache_dir)?;

    let mut uploaded_inputs: BTreeMap<u64, Vec<&AssetName>> = BTreeMap::new();
    for (name, input_manifest) in &manifest.inputs {
        if let Some(id) = input_manifest.id {
            let paths = uploaded_inputs.entry(id).or_default();
            paths.push(name);
        }
    }

    let mut index: BTreeMap<u64, String> = BTreeMap::new();
    for (id, contributing_assets) in uploaded_inputs {
        if contributing_assets.len() == 1 {
            index.insert(id, contributing_assets[0].to_string());
        } else {
            let contents = api_client.download_image(id).await?;
            let path = options.cache_dir.join(id.to_string());
            fs::write(&path, contents)?;

            index.insert(id, path.display().to_string());
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.index_file)?);
    serde_json::to_writer_pretty(&mut file, &index)?;
    file.flush()?;

    Ok(())
}
