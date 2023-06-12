use super::*;

use anyhow::{anyhow, Result};
use options::NivaOptions;
use serde_json::{json, Value as JsonValue};
use std::path::PathBuf;

use crate::utils::json::{merge_json_value, set_json_value};
use arguments::NivaArguments;

#[derive(Debug)]
pub enum NivaLaunchMode {
    AppMode,
    EmbMode,
}

#[derive(Debug)]
pub struct NivaLaunchInfo {
    pub mode: NivaLaunchMode, // App launch mode.
    pub name: String,         // Name of the project.
    pub id: String, // Identifier of the project. This is used to create data directory, cache directory and temporary directory. Generate by combining name and first eight characters of uuid. e.g. "niva.example.12345677"
    pub options: NivaOptions, // Project options, read from niva.json and command line arguments.
    pub workspace: PathBuf, // Workspace directory.
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

    fn load_app_mode_options() -> Result<(NivaOptions, PathBuf)> {
        #[cfg(target_os = "windows")]
        use crate::utils::win::load_resource;

        #[cfg(target_os = "macos")]
        use crate::utils::mac::load_resource;

        if let Ok(content) = load_resource("niva.json") {
            let mut options = Self::load_options_from_bytes(&content)?;
            options
                .resource
                .merge_default("$INNER/base.bin".to_string());
            return Ok((options, std::env::current_dir()?.to_path_buf()));
        }

        if let Some(root_dir) = std::env::current_exe()?.parent() {
            let niva_file = root_dir.join("niva.json");
            let mut options = Self::load_options_from_file(&niva_file)?;
            options
                .resource
                .merge_default(root_dir.to_string_lossy().to_string());
            return Ok((options, root_dir.to_path_buf()));
        }

        Err(anyhow!("Cannot find niva.json file."))
    }

    fn load_library_mode_options(arguments: NivaArguments) -> Result<(NivaOptions, PathBuf)> {
        let mut options_json = json!({});
        let mut workspace = std::env::current_dir()?;

        if let Some(JsonValue::String(niva_file)) = &arguments.file {
            let niva_file = std::path::Path::new(niva_file);
            let content = std::fs::read_to_string(niva_file)?;
            let options = Self::parse_options_json(&content)?;
            workspace = niva_file.parent().unwrap().to_path_buf();
            merge_json_value(&mut options_json, serde_json::to_value(options)?);
        }

        if let Some(niva_base) = &arguments.base {
            merge_json_value(&mut options_json, niva_base.clone());
        }

        for (path, value) in arguments.options {
            let path = path.trim_start_matches("niva.");
            set_json_value(&mut options_json, &path, value)?;
        }

        Ok((
            serde_json::from_value::<NivaOptions>(options_json)?,
            workspace,
        ))
    }

    pub fn new() -> Result<NivaLaunchInfo> {
        let arguments = NivaArguments::new()?;

        let mode: NivaLaunchMode;
        let options: NivaOptions;
        let workspace;

        if arguments.empty() {
            mode = NivaLaunchMode::AppMode;
            (options, workspace) = Self::load_app_mode_options()?;
        } else {
            mode = NivaLaunchMode::EmbMode;
            (options, workspace) = Self::load_library_mode_options(arguments)?;
        };

        let name = options.name.clone();
        let id = options.id.clone();

        Ok(Self {
            mode,
            name,
            id,
            options,
            workspace,
        })
    }
}
