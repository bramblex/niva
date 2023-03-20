use anyhow::{Ok, Result};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(target_os = "windows")]
use self::win_resource::load_resource;

mod utils;
#[cfg(target_os = "windows")]
mod win_resource;

pub trait ResourceManager: std::fmt::Debug + Send + Sync {
    fn exists(&self, path: String) -> bool;
    fn read(&self, path: String) -> Result<Vec<u8>>;
    fn extract(&self, from: String, to: &Path) -> Result<()>;
}

pub type ResourceManagerRef = Arc<dyn ResourceManager>;

pub fn create(resource_dir: Option<PathBuf>) -> Result<ResourceManagerRef> {
    match resource_dir {
        Some(dir) => Ok(Arc::new(FileSystemResource::new(dir)?)),
        None => {
            #[cfg(target_os = "macos")]
            {
                Ok(Arc::new(MacOSAppResourceManager::new()?))
            }
            #[cfg(target_os = "windows")]
            {
                Ok(Arc::new(WindowsExecutableResourceManager::new()?))
            }
        }
    }
}

#[derive(Debug)]
struct FileSystemResource {
    root_dir: PathBuf,
}

impl FileSystemResource {
    #[allow(dead_code)]
    pub fn new(root_dir: PathBuf) -> Result<FileSystemResource> {
        root_dir
            .exists()
            .then(|| root_dir.is_dir())
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?;
        Ok(FileSystemResource { root_dir })
    }
}

impl ResourceManager for FileSystemResource {
    fn exists(&self, path: String) -> bool {
        let path = self.root_dir.join(path);
        path.exists() && path.is_file()
    }

    fn read(&self, path: String) -> Result<Vec<u8>> {
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

#[cfg(target_os = "windows")]
struct WindowsExecutableResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
}

#[cfg(target_os = "windows")]
impl std::fmt::Debug for WindowsExecutableResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsExecutableResourceManager")
            .field("indexes", &self.indexes)
            .field("data", &"Vec<u8>")
            .finish()
    }
}

#[cfg(target_os = "windows")]
impl WindowsExecutableResourceManager {
    pub fn new() -> Result<WindowsExecutableResourceManager> {
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
        Ok(WindowsExecutableResourceManager { indexes, data })
    }
}

#[cfg(target_os = "windows")]
impl ResourceManager for WindowsExecutableResourceManager {
    fn exists(&self, path: String) -> bool {
        self.indexes.contains_key(&path)
    }

    fn read(&self, path: String) -> Result<Vec<u8>> {
        let (offset, length) = *self
            .indexes
            .get(&path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        Ok(self.data[offset..(offset + length)].to_vec())
    }

    fn extract(&self, from: String, to: &Path) -> Result<()> {
        let content = self.read(from)?;
        std::fs::write(to, content)?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
struct MacOSAppResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
}

#[cfg(target_os = "macos")]
impl std::fmt::Debug for MacOSAppResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSAppResourceManager")
            .field("indexes", &self.indexes)
            .field("data", &"Vec<u8>")
            .finish()
    }
}

#[cfg(target_os = "macos")]
impl MacOSAppResourceManager {
    pub fn new() -> Result<MacOSAppResourceManager> {
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
        Ok(MacOSAppResourceManager { indexes, data })
    }
}

#[cfg(target_os = "macos")]
impl ResourceManager for MacOSAppResourceManager {
    fn exists(&self, path: String) -> bool {
        self.indexes.contains_key(&path)
    }

    fn read(&self, path: String) -> Result<Vec<u8>> {
        let (offset, length) = *self
            .indexes
            .get(&path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        Ok(self.data[offset..(offset + length)].to_vec())
    }

    fn extract(&self, from: String, to: &Path) -> Result<()> {
        let content = self.read(from)?;
        std::fs::write(to, content)?;
        Ok(())
    }
}
