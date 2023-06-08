pub mod options;
pub mod utils;

use std::option;

use anyhow::{anyhow, Result};
use options::NivaOptions;
use serde_json::{json, Value as JsonValue};

use utils::json::{merge_json_value, set_json_value};

#[derive(Debug)]
pub enum NivaLaunchMode {
    AppMode,
    LibraryMode,
}

#[derive(Debug)]
pub struct NivaLaunchInfo {
    pub mode: NivaLaunchMode, // App launch mode.
    pub name: String,         // Name of the project.
    pub id: String, // Identifier of the project. This is used to create data directory, cache directory and temporary directory. Generate by combining name and first eight characters of uuid. e.g. "niva.example.12345678"
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

        #[cfg(target_os = "windows ")]
        {
            // todo!("Load app mode options.");
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

#[derive(Debug)]
pub struct NivaArguments {
    pub niva_file: Option<JsonValue>,
    pub niva_base: Option<JsonValue>,
    pub niva_options: Vec<(String, JsonValue)>,
}

impl NivaArguments {
    fn parse_arg_value(value_str: Option<&str>) -> Result<JsonValue> {
        if let Some(value_str) = value_str {
            if value_str == "null" {
                Ok(JsonValue::Null)
            } else if value_str == "true" {
                Ok(JsonValue::Bool(true))
            } else if value_str == "false" {
                Ok(JsonValue::Bool(true))
            } else {
                let head = value_str.chars().next();
                if let Some(head) = head {
                    if head == '[' || head == '{' || head == '"' {
                        let value = serde_json::from_str(value_str)?;
                        Ok(value)
                    } else if head.is_numeric() {
                        let parsed = serde_json::from_str::<JsonValue>(value_str);
                        match parsed {
                            Ok(value) => Ok(value),
                            Err(_) => Ok(JsonValue::String(value_str.to_string())),
                        }
                    } else {
                        Ok(JsonValue::String(value_str.to_string()))
                    }
                } else {
                    Err(anyhow!("Empty value."))
                }
            }
        } else {
            Ok(JsonValue::Bool(true))
        }
    }

    fn parse_arg<'a>(arg: &'a str) -> Result<(&'a str, JsonValue)> {
        let arg = arg.trim_start_matches("--");
        let mut parts = arg.splitn(2, '=');

        let key = parts
            .next()
            .ok_or(anyhow!("Unexpected command line argument `{}`.", arg))?;
        let parsed_value = Self::parse_arg_value(parts.next());

        match parsed_value {
            Ok(value) => Ok((key, value)),
            Err(err) => Err(anyhow!(
                "Parse command line argument value error `{}`. {}",
                arg,
                err
            )),
        }
    }

    pub fn new() -> Result<Self> {
        let args = std::env::args().collect::<Vec<String>>();

        let mut niva_options: Vec<(String, JsonValue)> = vec![];
        let mut niva_file: Option<JsonValue> = None;
        let mut niva_base: Option<JsonValue> = None;

        for arg in args {
            if !arg.starts_with("--niva") {
                continue;
            }

            let (key, value) = Self::parse_arg(&arg)?;

            if key == "niva-file" {
                match &value {
                    JsonValue::String(_) => niva_file = Some(value),
                    _ => {
                        return Err(anyhow!(
                            "Unexpected argument `{}`, require file path string.",
                            arg
                        ))
                    }
                }
            } else if key == "niva" {
                match &value {
                    JsonValue::Object(_) => niva_base = Some(value),
                    _ => {
                        return Err(anyhow!(
                            "Unexpected argument `{}`, require json object.",
                            arg
                        ))
                    }
                }
            } else {
                niva_options.push((key.to_string(), value));
            }
        }

        Ok(Self {
            niva_file,
            niva_base,
            niva_options,
        })
    }

    pub fn empty(&self) -> bool {
        self.niva_base.is_none() && self.niva_file.is_none() && self.niva_options.len() <= 0
    }
}
