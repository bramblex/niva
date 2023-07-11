use crate::utils::path::UniPath;

use super::{NivaResource, server::{NivaResourceServerRef, NivaResourceServer}};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use smol::io::{AsyncReadExt, AsyncSeekExt};

use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct FileSystemResource {
    root_path: PathBuf,
    server: NivaResourceServerRef,
}

#[async_trait]
impl NivaResource for FileSystemResource {

    fn base_url(self: Arc<Self>) -> String {
        format!("http://localhost:{}", self.server.port)
    }

    async fn exists(self: Arc<Self>, key: &str) -> bool {
        let metadata_result = async_fs::metadata(key).await;
        match metadata_result {
            Ok(metadata) => metadata.is_file(),
            _ => false,
        }
    }

    async fn read(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
        let path = self.key_to_path(key)?;
        let mut file = async_fs::OpenOptions::new().read(true).open(path).await?;
        file.seek(SeekFrom::Start(start as u64)).await?;

        let mut buffer: Vec<u8>;
        if len == 0 {
            buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;
        } else {
            buffer = vec![0; len];
            file.read_exact(&mut buffer).await?;
        }

        Ok(buffer)
    }
}

impl FileSystemResource {
    pub async fn new(root_path: &Path) -> Result<Arc<FileSystemResource>> {
        let resource = Arc::new(Self {
            root_path: root_path.to_path_buf(),
            server: NivaResourceServer::new().await?,
        });

        let server = resource.server.clone();
        smol::spawn(server.run(resource.clone())).detach();

        Ok(resource)
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
