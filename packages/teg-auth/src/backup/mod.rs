// use std::time::Duration;
// use std::sync::Arc;
use std::path::{PathBuf, Path};
// use std::collections::HashMap;
// use lazy_static::lazy_static;

// use sha2::{ Sha256, Digest };
use serde::{ Serialize, Deserialize };
use itertools::{ Itertools };
// use chrono::prelude::*;

use anyhow::{
    anyhow,
    Result,
    Context as _,
};

// use futures::stream::{
//     self,
//     StreamExt,
//     TryStreamExt,
// };
// use futures::future::FutureExt;

// use async_std::prelude::*;
// use async_std::fs::File;
// use async_std::io::BufReader;

mod schedule_backups;
pub use schedule_backups::schedule_backups;

mod backup;
pub use backup::backup;

mod restore;
pub use restore::*;

#[cfg(test)]
mod tests;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BackupRow {
    // MetaData(MetaData),
    Collection(Collection),
    TreeEntry(TreeEntry),
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// struct MetaData {
//     Sha256: Vec<u8>,
// }

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, std::hash::Hash)]
pub struct Collection {
    r#type: Vec<u8>,
    name: Vec<u8>,
}

pub type TreeEntry = Vec<Vec<u8>>;

pub fn get_backup_files(
    backups_dir: &Path,
) -> Result<impl std::iter::Iterator<Item = PathBuf>> {
    let dir = std::fs::read_dir(&backups_dir)
    .with_context(|| format!("Unable to open backups directory ({:?})", backups_dir))?;

    let files: Vec<Option<PathBuf>> = dir
        .map(|entry| -> Result<Option<PathBuf>> {
            let entry = entry
                .with_context(|| "Error reading file in backup directory")?;

            let path = entry.path();
            let file_name = path.file_name()
                .and_then(|name| name.to_str());

            if let Some(file_name) = file_name {
                if path.is_dir() || !file_name.ends_with(".bck") {
                    Ok(None)
                } else {
                    Ok(Some(path))
                }
            } else {
                Ok(None)
            }
        })
        .try_collect()?;

    let files = files
        .into_iter()
        .filter_map(|option| option)
        .sorted();

    Ok(files)
}

pub async fn get_latest_backup(
    backups_dir: &Path,
) -> Result<PathBuf> {
    let mut files = get_backup_files(&backups_dir)?;

    files.next().ok_or(anyhow!("No Backups Found"))
}
