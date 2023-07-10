pub mod bin;
pub mod fs;
pub mod options;
pub mod server;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use self::{bin::BinaryResource, fs::FileSystemResource, server::NivaResourceServer};
use crate::utils::path::UniPath;
use options::ResourceOptions;

#[async_trait]
pub trait NivaResource {
    async fn exists(self: Arc<Self>, key: &str) -> bool;

    async fn read(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>>;

    async fn read_all(self: Arc<Self>, key: &str) -> Result<Vec<u8>> {
        let bytes = self.read(key, 0, 0).await?;
        Ok(bytes)
    }

    async fn read_string(self: Arc<Self>, key: &str) -> Result<String> {
        let content = self.read_all(key).await?;
        Ok(std::str::from_utf8(&content)?.to_string())
    }
}

pub type NivaResourceRef = Arc<dyn NivaResource + Sync + Send>;
pub type NivaResourceServerRef = Arc<NivaResourceServer>;

pub struct NivaResourceManager {
    workspace: PathBuf,
    resources: HashMap<String, (NivaResourceRef, NivaResourceServerRef)>,
}

impl NivaResourceManager {
    pub fn get(&self, name: &str) -> Result<(NivaResourceRef, NivaResourceServerRef)> {
        Ok(self
            .resources
            .get(name)
            .ok_or(anyhow!("Failed to find resource {}", name))?
            .clone())
    }

    pub async fn register(&mut self, name: &String, resource_path: &String) -> Result<()> {
        if resource_path.starts_with("$INNER:") {
            let resource_name = name.trim_start_matches("$INNER:");
            let resource = Arc::new(BinaryResource::from_inner(resource_name)?);
            let _resource = resource.clone();
            self.resources.insert(
                name.to_string(),
                (resource, NivaResourceServer::new(_resource).await?),
            );
        } else {
            let resource_full_path = self
                .workspace
                .join(UniPath::new(resource_path).to_path_buf());

            let metadata = async_fs::metadata(&resource_full_path).await?;
            if metadata.is_file() {
                let resource = Arc::new(BinaryResource::from_file(&resource_full_path).await?);
                let _resource = resource.clone();
                self.resources.insert(
                    name.to_string(),
                    (resource, NivaResourceServer::new(_resource).await?),
                );
            } else if metadata.is_dir() {
                let resource = Arc::new(FileSystemResource::new(&resource_full_path)?);
                let _resource = resource.clone();
                self.resources.insert(
                    name.to_string(),
                    (resource, NivaResourceServer::new(_resource).await?),
                );
            } else {
                return Err(anyhow!(
                    "Resource load failed `{}`",
                    resource_full_path.to_string_lossy()
                ));
            }
        }
        Ok(())
    }

    pub async fn new(workspace: &Path, options: &ResourceOptions) -> Result<NivaResourceManager> {
        let mut manager = Self {
            workspace: workspace.to_path_buf(),
            resources: HashMap::new(),
        };
        for (name, resource_path) in &options.0 {
            manager.register(name, resource_path).await?;
        }
        Ok(manager)
    }
}
