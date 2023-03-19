use anyhow::Result;
mod options;


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

    pub config: ProjectOptions,

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

fn get_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let arg_pairs: Vec<(String, String)> = args[1..]
        .chunks(2)
        .map(|c| {
            (
                c[0].clone(),
                if c.len() > 1 {
                    c[1].clone()
                } else {
                    "".to_string()
                },
            )
        })
        .collect();
    let mut args = Args {
        resource_dir: None,
        debug_entry: None,
        devtools: None,
    };
    for (key, value) in arg_pairs {
        match key.as_str() {
            "--resource-dir" => {
                let path = Path::new(value.as_str());
                let path = if path.is_relative() {
                    let current_dir = std::env::current_dir().unwrap();
                    current_dir.join(path)
                } else {
                    path.to_path_buf()
                };
                args.resource_dir = Some(path);
            }
            "--debug-entry" => {
                args.debug_entry = Some(value);
            }
            "--devtools" => {
                args.devtools = Some(value);
            }
            _ => {}
        }
    }
    args
}

fn get_options(resource: ResourceManagerRef) -> Result<ProjectOptions> {
    let options_content = resource.read("tauri_lite.json".to_string())?;
    let options = serde_json::from_slice::<ProjectOptions>(&options_content)?;
    Ok(options)
}

pub fn init() -> Result<Arc<Environment>> {
    let args = get_args();
    let resource = resource_manager::create(args.resource_dir)?;

    let mut options = get_options(resource.clone())?;
    options.window.title = Some(options.window.title.unwrap_or_else(|| options.name.clone()));
    if let Some(debug) = args.devtools {
        if (debug == "true") || (debug == "1") {
            options.window.devtools = Some(true);
        }
    }

    let project_name = options.name.clone();
    let project_uuid = options.uuid.clone();

    let temp_dir = std::env::temp_dir().join(project_name.clone() + "." + &project_uuid);

    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| Error::new(ErrorKind::Other, "BaseDirs not found"))?;
    let data_dir = base_dirs
        .data_dir()
        .join(project_name.clone() + "." + &project_uuid);

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
        config: options,
        debug_entry: args.debug_entry,
    }))
}
