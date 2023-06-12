pub mod bin;
pub mod fs;
pub mod options;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{collections::HashMap, path::Path, sync::Arc};

use options::ResourceOptions;

use crate::utils::path::UniPath;

use self::{bin::BinaryResource, fs::FileSystemResource};

#[async_trait]
pub trait Resource {
    fn exists(self: Arc<Self>, key: &str) -> bool;

    fn read(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>>;

    async fn exists_async(self: Arc<Self>, key: &str) -> bool;

    async fn read_async(self: Arc<Self>, key: &str, start: usize, len: usize) -> Result<Vec<u8>>;

    async fn read_all_async(self: Arc<Self>, key: &str) -> Result<Vec<u8>> {
        let bytes = self.read_async(key, 0, 0).await?;
        Ok(bytes)
    }

    async fn read_string_async(self: Arc<Self>, key: &str) -> Result<String> {
        let content = self.read_all_async(key).await?;
        Ok(std::str::from_utf8(&content)?.to_string())
    }
}

pub type ResourceRef = Arc<dyn Resource>;

pub struct ResourceManager {
    resources: HashMap<String, ResourceRef>,
}

impl ResourceManager {
    pub fn get(&self, name: &String) -> Result<ResourceRef> {
        Ok(self
            .resources
            .get(name)
            .ok_or(anyhow!("Failed to find resource {}", name))?
            .clone())
    }

    pub fn new(workspace: &Path, options: &ResourceOptions) -> Result<ResourceManager> {
        let mut resources: HashMap<String, ResourceRef> = HashMap::new();
        for (name, resource_path) in &options.0 {
            if resource_path.starts_with("$INNER:") {
                let resource_name = name.trim_start_matches("$INNER:");
                let resource = Arc::new(BinaryResource::from_inner(resource_name)?);
                resources.insert(name.to_string(), resource);
            } else {
                let resource_full_path =
                    workspace.join(UniPath::new(resource_path).to_path_buf());

                if resource_full_path.is_file() {
                    let resource = Arc::new(BinaryResource::from_file(&resource_full_path)?);
                    resources.insert(name.to_string(), resource);
                } else if resource_full_path.is_dir() {
                    let resource = Arc::new(FileSystemResource::new(&resource_full_path)?);
                    resources.insert(name.to_string(), resource);
                } else {
                    return Err(anyhow!(
                        "Resource load failed `{}`",
                        resource_full_path.to_string_lossy()
                    ));
                }
            }
        }
        Ok(Self { resources })
    }
}
