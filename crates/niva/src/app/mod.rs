mod utils;

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, path::PathBuf};

pub struct NivaLaunchInfo {
    name: String,         // Name of the project.
    id: String, // Identifier of the project. This is used to create data directory, cache directory and temporary directory. Generate by combining name and first eight characters of uuid. e.g. "niva.example.12345678"
    options: NivaOptions, // Project options, read from niva.json and command line arguments.
}

pub struct NivaArguments {
    niva_file: Option<JsonValue>,
    niva_base: Option<JsonValue>,
    niva_options: Vec<(String, JsonValue)>,
}

impl NivaArguments {
    fn is_str_start_with_digit(s: &str) -> bool {
        if let Some(first_char) = s.chars().next() {
            first_char.is_numeric()
        } else {
            false
        }
    }
    fn parse_niva_arg<'a>(arg: &'a str) -> Option<(&'a str, JsonValue)> {
        if !arg.starts_with("--") {
            return None;
        }

        let arg = arg.trim_start_matches("--");
        let mut parts = arg.splitn(2, '=');

        let key_str = parts.next()?;
        let value_str = parts.next();

        if let Some(value_str) = value_str {
            let mut value = JsonValue::Null;
            if value_str == "null" {
            } else if value_str == "true" {
                value = JsonValue::Bool(true);
            } else if value_str == "false" {
                value = JsonValue::Bool(false);
            } else if Self::is_str_start_with_digit(value_str) {
                let result = value_str.parse::<f64>();
                if let Ok(nu) = result {
                    value = JsonValue::Number(nu);
                } else {
                    value = JsonValue::String(value_str.to_string());
                }
            } else {
                value = JsonValue::String(value_str.to_string());
            }
            Some((key_str, value))
        } else {
            Some((key_str, JsonValue::Bool(true)))
        }
    }

    pub fn new() -> Result<Self> {
        let args = std::env::args().collect::<Vec<String>>();

        let mut niva_options: Vec<(String, JsonValue)> = vec![];

        for arg in args {
            if !arg.starts_with("--niva") {
                continue;
            }

            // let [key_str, value_str] = parts[..];

            // if parts.len() != 2 {
            //     return Err(anyhow!("Unexpected Niva Command line argument: {}", arg));
            // }

            // if key_str == "--niva-file" {
            // } else if  key_str == "--niva" {
            // } else if key {
            // }

            // if arg == "--niva-file" {
            // } else if arg == "--niva" {
            // } else {
            //     let option_path = arg.trim_start_matches("--niva");
            // }
            // if let Some((key, value)) = Self::parse_niva_arg() {
            //     niva_args.insert(key, value);
            // }
        }

        Ok(Self {
            niva_file: None,
            niva_base: None,
            niva_options,
        })
    }
}

pub struct NivaOptions {}
