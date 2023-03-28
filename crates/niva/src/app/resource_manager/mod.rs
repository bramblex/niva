
#[cfg(target = "windows")]
mod win_utils;

mod image_utils;

use anyhow::{Ok, Result};
use tao::window::Icon;
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
};

pub trait ResourceManager: std::fmt::Debug + Send + Sync {
    fn exists(&self, path: String) -> bool;
    fn load(&self, path: String) -> Result<Vec<u8>>;
    fn extract(&self, from: String, to: &Path) -> Result<()>;

    fn load_icon(&self, path: String) -> Result<Icon> {
        let data = self.load(path.clone())?;
        if path.ends_with("png") {
            image_utils::png_to_icon(&data)
        // } else if path.ends_with("jpg") {
        //     image_utils::jpg_to_icon(&data)
        // } else if path.ends_with("jpeg") {
        //     image_utils::jpg_to_icon(&data)
        } else {
            Err(anyhow::anyhow!("Unsupported icon format."))
        }
    }
}

#[derive(Debug)]
pub struct FileSystemResource {
    root_dir: PathBuf,
}

impl FileSystemResource {
    pub fn new(root_dir: &Path) -> Result<FileSystemResource> {
        root_dir
            .exists()
            .then(|| root_dir.is_dir())
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?;
        Ok(FileSystemResource { root_dir: root_dir.to_path_buf() })
    }
}

impl ResourceManager for FileSystemResource {
    fn exists(&self, path: String) -> bool {
        let path = self.root_dir.join(path);
        path.exists() && path.is_file()
    }

    fn load(&self, path: String) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.root_dir.join(path))?)
    }

    fn extract(&self, from: String, to: &Path) -> Result<()> {
        fs_extra::file::copy(
            self.root_dir.join(from),
            to,
            &fs_extra::file::CopyOptions::new(),
        )?;
        Ok(())
    }
}

pub struct AppResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
}

impl std::fmt::Debug for AppResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSAppResourceManager")
            .field("indexes", &self.indexes)
            .field("data", &"Vec<u8>")
            .finish()
    }
}

impl AppResourceManager {
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<AppResourceManager> {
        let resources_dir = std::env::current_exe()?
            .parent()
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?
            .join("../Resources/");
        let indexes_data = std::fs::read(resources_dir.join("RESOURCE_INDEXES"))?;
        println!("indexes_data: {:?}", indexes_data.len());
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        println!("indexes: {:?}", indexes);
        let compressed_data = std::fs::read(resources_dir.join("RESOURCE_DATA"))?;
        println!("compressed_data: {:?}", compressed_data.len());
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        println!("data: {:?}", data.len());
        Ok(AppResourceManager { indexes, data })
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Result<AppResourceManager> {
        use win_utils::load_resource;

        println!("new resource.");
        let indexes_data = load_resource("RESOURCE_INDEXES")?;
        println!("indexes_data: {:?}", indexes_data.len());
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        println!("indexes: {:?}", indexes);
        let compressed_data = load_resource("RESOURCE_DATA")?;
        println!("compressed_data: {:?}", compressed_data.len());
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        println!("data: {:?}", data.len());
        Ok(AppResourceManager { indexes, data })
    }
}

impl ResourceManager for AppResourceManager {
    fn exists(&self, path: String) -> bool {
        self.indexes.contains_key(&path)
    }

    fn load(&self, path: String) -> Result<Vec<u8>> {
        let (offset, length) = *self
            .indexes
            .get(&path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        Ok(self.data[offset..(offset + length)].to_vec())
    }

    fn extract(&self, from: String, to: &Path) -> Result<()> {
        let content = self.load(from)?;
        std::fs::write(to, content)?;
        Ok(())
    }
}
