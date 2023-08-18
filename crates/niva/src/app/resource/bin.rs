use super::{
    server::{NivaResourceServer, NivaResourceServerRef},
    NivaResource,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use url::Url;
use std::{collections::HashMap, io::Read, path::Path, sync::Arc};

pub struct BinaryResource {
    index: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
    server: NivaResourceServerRef,
}

#[async_trait]
impl NivaResource for BinaryResource {
    async fn exists(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    async fn read(&self, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
        let (offset, size) = self
            .index
            .get(key)
            .ok_or(anyhow!("Cannot find file in resource, `{}`", key))?;

        let len = if len == 0 { *size } else { len };

        let start = std::cmp::min(offset + start, offset + size);
        let len = std::cmp::min(size - start, len);
        let data = &self.data[start..len];

        Ok(data.to_vec())
    }

    fn base_url(&self) -> Url {
        Url::parse(&format!("http://localhost:{}", self.server.port)).unwrap()
    }
}

impl BinaryResource {
    pub async fn new(buffer: &[u8]) -> Result<Arc<BinaryResource>> {
        let mut parts = buffer.splitn(2, |b| *b == b'\0');
        let index_bytes = parts
            .next()
            .ok_or(anyhow!("Unexpected binary resource format."))?;
        let data_bytes = parts
            .next()
            .ok_or(anyhow!("Unexpected binary resource format."))?;

        let index = serde_json::from_slice::<HashMap<String, (usize, usize)>>(index_bytes)?;
        let mut data_decoder = flate2::read::DeflateDecoder::new(data_bytes);

        let mut data = Vec::new();
        data_decoder.read_to_end(&mut data)?;

        let resource = Arc::new(Self {
            index,
            data,
            server: NivaResourceServer::new().await?,
        });

        let server = resource.server.clone();
        smol::spawn(server.run(resource.clone())).detach();

        Ok(resource)
    }

    pub async fn from_file(path: &Path) -> Result<Arc<BinaryResource>> {
        let content = async_fs::read(path).await?;
        BinaryResource::new(&content).await
    }

    pub async fn from_inner(resource_name: &str) -> Result<Arc<BinaryResource>> {
        #[cfg(target_os = "windows")]
        use crate::utils::win::load_resource;

        #[cfg(target_os = "macos")]
        use crate::utils::mac::load_resource;

        let resource_bytes = load_resource(resource_name)?;
        BinaryResource::new(&resource_bytes).await
    }
}
