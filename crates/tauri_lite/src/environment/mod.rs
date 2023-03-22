use anyhow::{anyhow, Result};
mod options;

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::resource_manager::{self, ResourceManagerRef};
pub use options::*;

#[derive(Debug)]
pub struct Environment {
    pub project_name: String,
    pub project_uuid: String,

    // resource
    pub resource: ResourceManagerRef,

    pub temp_dir: PathBuf,
    pub data_dir: PathBuf,

    pub options: ProjectOptions,

    // debug entry url
    pub debug_entry: Option<String>,
}

unsafe impl Send for Environment {}
unsafe impl Sync for Environment {}

pub type EnvironmentRef = Arc<Environment>;

struct Args {
    pub resource_dir: Option<PathBuf>,
    pub debug_entry: Option<String>,
    pub devtools: Option<String>,
}

fn parse_args(args: &[String]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for arg in args.iter().skip(1) {
        if let Some(arg) = arg.strip_prefix("--") {
            let (key, value) = match arg.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => (arg.to_string(), "".to_string()),
            };
            result.insert(key, value);
        }
    }
    result
}

fn get_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let parsed_args = parse_args(&args);

    Args {
        resource_dir: {
            match &parsed_args.get("resource-dir") {
                Some(path) => {
                    let path = Path::new(path);
                    let path = if path.is_relative() {
                        let current_dir = std::env::current_dir().unwrap();
                        current_dir.join(path)
                    } else {
                        path.to_path_buf()
                    };
                    Some(path)
                }
                None => None,
            }
        },
        debug_entry: parsed_args.get("debug-entry").cloned(),
        devtools: parsed_args.get("devtools").cloned(),
    }
}

fn get_options(resource: ResourceManagerRef) -> Result<ProjectOptions> {
    let options_content = resource.read("tauri_lite.json".to_string())?;
    let options = serde_json::from_slice::<ProjectOptions>(&options_content)?;
    Ok(options)
}

pub fn init() -> Result<Arc<Environment>> {
    print!("Initializing environment... ");
    let args = get_args();
    println!("get args done.");
    let resource = resource_manager::create(args.resource_dir)?;
    print!("create resource manager done.");

    let mut options = get_options(resource.clone())?;
    options.window.title = Some(options.window.title.unwrap_or_else(|| options.name.clone()));
    if let Some(debug) = args.devtools {
        if (debug == "true") || (debug == "1") {
            options.window.devtools = Some(true);
        }
    }

    let project_name = options.name.clone();
    let project_uuid = options.uuid.clone();

    let project_dir_name =
        project_name.clone() + "_" + project_uuid.get(0..8).ok_or(anyhow!("uuid too short"))?;
    let temp_dir = std::env::temp_dir().join(&project_dir_name);

    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| Error::new(ErrorKind::Other, "BaseDirs not found"))?;
    let data_dir = base_dirs.data_dir().join(&project_dir_name);

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)?;
    }

    Ok(Arc::new(Environment {
        project_name,
        project_uuid,
        resource,
        temp_dir,
        data_dir,
        options,
        debug_entry: args.debug_entry,
    }))
}
