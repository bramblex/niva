pub mod bin;
pub mod fs;
pub mod options;
pub mod server;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tao::{event::Event, event_loop::ControlFlow};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex}, any::Any,
};
use url::Url;

use self::{bin::BinaryResource, fs::FileSystemResource};
use crate::utils::path::UniPath;
use options::ResourceOptions;

use super::{
    event::{NivaEventLoop, NivaEvent, NivaWindowTarget},
    launch_info::NivaLaunchInfo,
    NivaAppRef,
};

#[async_trait]
pub trait NivaResource {
    fn base_url(&self) -> Url;

    async fn exists(&self, key: &str) -> bool;

    async fn read(&self, key: &str, start: usize, len: usize) -> Result<Vec<u8>>;

    async fn read_all(&self, key: &str) -> Result<Vec<u8>> {
        let bytes = self.read(key, 0, 0).await?;
        Ok(bytes)
    }

    async fn read_string(&self, key: &str) -> Result<String> {
        let content = self.read_all(key).await?;
        Ok(std::str::from_utf8(&content)?.to_string())
    }
}

pub type NivaResourceRef = Arc<dyn NivaResource + Sync + Send>;

pub struct NivaResourceManager {
    app: Option<NivaAppRef>,
    workspace: PathBuf,
    resources: HashMap<String, NivaResourceRef>,
}

impl NivaResourceManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> Result<Arc<Mutex<NivaResourceManager>>> {
        let manager = Self {
            app: None,
            workspace: launch_info.workspace.clone(),
            resources: HashMap::new(),
        };
        Ok(Arc::new(Mutex::new(manager)))
    }

    pub async fn init(&mut self, app: &NivaAppRef) -> Result<()> {
        self.app = Some(app.clone());
        let options = &app.launch_info.options.resource;
        for (name, resource_path) in &options.0 {
            self.register(name, resource_path).await?;
        }
        Ok(())
    }

    pub fn start(&mut self, event_loop: &NivaEventLoop) -> Result<()> {
        Ok(())
    }

    pub fn run(&mut self, event: &Event<NivaEvent>, target: &NivaWindowTarget, control_flow: &mut ControlFlow) -> Result<()> {
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<NivaResourceRef> {
        Ok(self
            .resources
            .get(name)
            .ok_or(anyhow!("Failed to find resource {}", name))?
            .clone())
    }

    pub async fn register(&mut self, name: &str, resource_path: &str) -> Result<()> {
        if resource_path.starts_with("$INNER:") {
            let resource_name = name.trim_start_matches("$INNER:");
            let resource = BinaryResource::from_inner(resource_name).await?;
            self.resources.insert(name.to_string(), resource);
        } else {
            let resource_full_path = self
                .workspace
                .join(UniPath::new(resource_path).to_path_buf());

            let metadata = async_fs::metadata(&resource_full_path).await?;
            if metadata.is_file() {
                let resource = BinaryResource::from_file(&resource_full_path).await?;
                self.resources.insert(name.to_string(), resource);
            } else if metadata.is_dir() {
                let resource = FileSystemResource::new(&resource_full_path).await?;
                self.resources.insert(name.to_string(), resource);
            } else {
                return Err(anyhow!(
                    "Resource load failed `{}`",
                    resource_full_path.to_string_lossy()
                ));
            }
        }
        Ok(())
    }

    pub fn transfer_url(&self, origin_url: &str) -> Result<Url> {
        let base_url = self.get("base")?.base_url();
        let mut target_url = base_url.join(origin_url)?;
        if let Some(url::Host::Domain(host)) = target_url.host() {
            let params = host.splitn(2, '.').collect::<Vec<&str>>();
            if params.len() == 2 && params[1] == "resource.niva" {
                let resource = self.get(params[0])?;
                let base_url = resource.base_url();
                target_url = base_url.join(target_url.path())?;
            }
        }
        Ok(target_url)
    }
}
