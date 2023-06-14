use super::NivaResource;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{collections::HashMap, io::Read, path::Path, sync::Arc};

pub struct BinaryResource {
    index: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
}

#[async_trait]
impl NivaResource for BinaryResource {
    fn exists(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    fn read(&self, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
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

    async fn exists_async(&self, key: &str) -> bool {
        self.exists(key)
    }

    async fn read_async(&self, key: &str, start: usize, len: usize) -> Result<Vec<u8>> {
        self.read(key, start, len)
    }
}

impl BinaryResource {
    pub fn new(buffer: &[u8]) -> Result<BinaryResource> {
        let mut parts = buffer.splitn(2, |b| *b == b'\n');
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

        Ok(Self { index, data })
    }

    pub fn from_file(path: &Path) -> Result<BinaryResource> {
        let content = std::fs::read(path)?;
        BinaryResource::new(&content)
    }

    pub fn from_inner(resource_name: &str) -> Result<BinaryResource> {
        #[cfg(target_os = "windows")]
        use crate::utils::win::load_resource;

        #[cfg(target_os = "macos")]
        use crate::utils::mac::load_resource;

        let resource_bytes = load_resource(resource_name)?;
        let binary_resource = BinaryResource::new(&resource_bytes)?;

        Ok(binary_resource)
    }
}
