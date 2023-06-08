use super::*;

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;

use utils::json::{ parse_argument_value};

#[derive(Debug)]
pub struct NivaArguments {
    pub niva_file: Option<JsonValue>,
    pub niva_base: Option<JsonValue>,
    pub niva_options: Vec<(String, JsonValue)>,
}

impl NivaArguments {
    fn parse_argument<'a>(arg: &'a str) -> Result<(&'a str, JsonValue)> {
        let arg = arg.trim_start_matches("--");
        let mut parts = arg.splitn(1, '=');

        let key = parts
            .next()
            .ok_or(anyhow!("Unexpected command line argument `{}`.", arg))?;

        let parsed_value = if let Some(value_str) = parts.next() {
            parse_argument_value(value_str)
        } else {
            Ok(JsonValue::Bool(true))
        };

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

            let (key, value) = Self::parse_argument(&arg)?;

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
