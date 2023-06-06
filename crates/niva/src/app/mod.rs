mod utils;

use serde_json::{Value as JsonValue};
use std::{collections::HashMap, path::PathBuf};

pub struct NivaLaunchInfo {
    name: String,         // Name of the project.
    id: String, // Identifier of the project. This is used to create data directory, cache directory and temporary directory. Generate by combining name and first eight characters of uuid. e.g. "niva.example.12345678"
    options: NivaOptions, // Project options, read from niva.json and command line arguments.
}

pub struct NivaArguments {
    niva_config_file: Option<JsonValue>,
    niva_options: Option<JsonValue>,
    niva_args: Vec<(String, JsonValue)>,
}
impl NivaArguments { pub fn new() -> Self {
        let args = std::env::args().collect::<Vec<String>>();

        let mut niva_args = HashMap::<String, JsonValue>::new();

        for arg in args {
            if !arg.starts_with("--niva") {
                continue;
            }
            // let arg = arg.trim_start_matches("--niva");
            // if arg == "--niva"
            // if let Some((key, value)) = Self::parse_niva_arg() {
            //     niva_args.insert(key, value);
            // }
        }

        Self {
            niva_config_file: None,
            niva_options: None,
            niva_args: vec![],
        }
    }

    pub fn patch(&self, options: &mut serde_json::Value) {
    }

    fn parse_niva_arg() -> Option<(String, JsonValue)> {
        None
    }
}

pub struct NivaOptions {}
