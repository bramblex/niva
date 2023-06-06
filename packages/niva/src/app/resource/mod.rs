pub mod bin;
pub mod fs;
pub mod options;
pub mod server;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use objc::runtime::NO;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tao::{event::Event, event_loop::ControlFlow, window::Icon};
use url::Url;

use self::{bin::BinaryResource, fs::FileSystemResource};
use crate::utils::path::UniPath;

use super::{
    event::{NivaEvent, NivaEventLoop, NivaWindowTarget},
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

    pub fn run(
        &mut self,
        event: &Event<NivaEvent>,
        target: &NivaWindowTarget,
        control_flow: &mut ControlFlow,
    ) -> Result<()> {
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

    pub fn transform_resource_url(&self, origin_url: &str) -> Result<Url> {
        if let Some((resource_name, resource_path)) = Self::parse_resource_url(origin_url)? {
            let resource = self.get(&resource_name)?;
            return Ok(resource.base_url().join(&resource_path)?);
        };
        Ok(Url::parse(origin_url)?)
    }

    pub async fn load_by_resource_url(&self, origin_url: &str) -> Result<Vec<u8>> {
        let (resource_name, resource_path) =
            Self::parse_resource_url(origin_url)?.ok_or(anyhow!("Unexpected Resource uri."))?;
        self.load(&resource_name, &resource_path).await
    }

    pub async fn load(&self, resource_name: &str, resource_path: &str) -> Result<Vec<u8>> {
        let resource = self.get(resource_name)?;
        resource.read_all(resource_path).await
    }

    fn parse_resource_url(origin_url: &str) -> Result<Option<(String, String)>> {
        let base_url = Url::parse("http://base.resource.niva/")?;
        let target_url = base_url.join(origin_url)?;

        let host = target_url
            .host()
            .ok_or(anyhow!("Unexpected url."))?
            .to_string();

        let params = host.splitn(2, '.').collect::<Vec<&str>>();
        if params.len() == 2 && params[1] == "resource.niva" {
            let resource_key = target_url.path().to_string().strip_prefix("/").ok_or(anyhow!(""))?.to_string();
            return Ok(Some((
                params[0].to_string(),
                resource_key,
            )));
        }
        Ok(None)
    }
}
