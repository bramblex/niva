use super::*;

use anyhow::{anyhow, Result};
use options::NivaOptions;
use serde_json::{json, Value as JsonValue};

use utils::json::{merge_json_value, set_json_value};
use arguments::NivaArguments;

#[derive(Debug)]
pub enum NivaLaunchMode {
    AppMode,
    LibraryMode,
}

#[derive(Debug)]
pub struct NivaLaunchInfo {
    pub mode: NivaLaunchMode, // App launch mode.
    pub name: String,         // Name of the project.
    pub id: String, // Identifier of the project. This is used to create data directory, cache directory and temporary directory. Generate by combining name and first eight characters of uuid. e.g. "niva.example.12345677"
    pub options: NivaOptions, // Project options, read from niva.json and command line arguments.
}

impl NivaLaunchInfo {
    fn parse_options_json(content: &str) -> Result<JsonValue> {
        let mut options_json = serde_json::from_str::<JsonValue>(content)?;

        let platform = std::env::consts::OS;
        let platform_options = options_json.get(platform).cloned();

        if let Some(platform_options) = platform_options {
            merge_json_value(&mut options_json, platform_options);
        }

        Ok(options_json)
    }

    fn parse_options(content: &str) -> Result<NivaOptions> {
        let options = Self::parse_options_json(content)?;
        Ok(serde_json::from_value::<NivaOptions>(options)?)
    }

    fn load_options_from_bytes(bytes: &[u8]) -> Result<NivaOptions> {
        let content = std::str::from_utf8(bytes)?;
        let options = Self::parse_options(&content)?;
        Ok(options)
    }

    fn load_options_from_file(niva_file: &std::path::Path) -> Result<NivaOptions> {
        let content = std::fs::read_to_string(niva_file)?;
        let options = Self::parse_options(&content)?;
        Ok(options)
    }

    fn load_app_mode_options() -> Result<NivaOptions> {
        #[cfg(target_os = "macos")]
        {
            use utils::mac::get_app_folder;
            if let Some(app_dir) = get_app_folder() {
                let niva_file = app_dir.join("Resources").join("niva.json");
                return Self::load_options_from_file(&niva_file);
            }
        };

        #[cfg(target_os = "windows")]
        {
            use utils::win::load_resource;
            if let Ok(content) = load_resource("niva.json") {
                return Self::load_options_from_bytes(&content);
            }
        };

        if let Some(root_dir) = std::env::current_exe()?.parent() {
            let niva_file = root_dir.join("niva.json");
            return Self::load_options_from_file(&niva_file);
        }

        Err(anyhow!("Cannot find niva.json file."))
    }

    fn load_library_mode_options(arguments: NivaArguments) -> Result<NivaOptions> {
        let mut options_json = json!({});

        if let Some(niva_file) = &arguments.niva_file {
            if let Some(niva_file) = niva_file.as_str() {
                let content = std::fs::read_to_string(niva_file)?;
                let opts = Self::parse_options_json(&content)?;
                merge_json_value(&mut options_json, opts);
            }
        }

        if let Some(niva_base) = &arguments.niva_base {
            merge_json_value(&mut options_json, niva_base.clone());
        }

        for (path, value) in arguments.niva_options {
            let path = path.trim_start_matches("niva.");
            set_json_value(&mut options_json, &path, value)?;
        }

        Ok(serde_json::from_value::<NivaOptions>(options_json)?)
    }

    pub fn new() -> Result<NivaLaunchInfo> {
        let arguments = NivaArguments::new()?;

        let (mode, options) = if arguments.empty() {
            (NivaLaunchMode::AppMode, Self::load_app_mode_options()?)
        } else {
            (
                NivaLaunchMode::LibraryMode,
                Self::load_library_mode_options(arguments)?,
            )
        };

        let name = options.name.clone();
        let id = options.id.clone();

        Ok(Self {
            mode,
            name,
            id,
            options,
        })
    }
}

