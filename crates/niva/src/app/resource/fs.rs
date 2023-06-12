use crate::utils::path::UniPath;

use super::Resource;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct FileSystemResource {
    root_path: PathBuf,
}

#[async_trait]
impl Resource for FileSystemResource {
    fn exists(self: Arc<Self>, key: &str) -> bool {
        let path = self.key_to_path(key);
        if let Ok(path) = path {
            path.exists() && path.is_file()
        } else {
            false
        }
    }

    fn read(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
        let path = self.key_to_path(key)?;
        let mut file = OpenOptions::new().read(true).open(path)?;
        file.seek(SeekFrom::Start(start as u64))?;
        let mut buffer = vec![0; len];
        file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    async fn exists_async(self: Arc<Self>, key: &str) -> bool {
        self.exists(key)
    }

    async fn read_async(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
        todo!()
    }
}

impl FileSystemResource {
    pub fn new(root_path: &Path) -> Result<FileSystemResource> {
        Ok(Self {
            root_path: root_path.to_path_buf(),
        })
    }

    pub fn key_to_path(&self, key: &str) -> Result<PathBuf> {
        let path = UniPath::new(key);
        if path.has_upward() {
            Err(anyhow!("Unsupported upward path"))
        } else {
            Ok(self.root_path.join(path.to_path_buf()))
        }
    }
}
