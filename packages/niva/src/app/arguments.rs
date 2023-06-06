use super::*;

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;

use crate::utils::json::parse_argument_value;

#[derive(Debug)]
pub struct NivaArguments {
    pub file: Option<JsonValue>,
    pub base: Option<JsonValue>,
    pub options: Vec<(String, JsonValue)>,
}

impl NivaArguments {
    fn parse_argument<'a>(arg: &'a str) -> Result<(&'a str, JsonValue)> {
        let arg = arg.trim_start_matches("--");
        let mut parts = arg.splitn(2, '=');

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

        let mut file: Option<JsonValue> = None;
        let mut base: Option<JsonValue> = None;
        let mut options: Vec<(String, JsonValue)> = vec![];

        for arg in args {
            if !arg.starts_with("--niva") {
                continue;
            }

            let (key, value) = Self::parse_argument(&arg)?;

            if key == "niva-file" {
                match &value {
                    JsonValue::String(_) => file = Some(value),
                    _ => return Err(anyhow!("Unexpected argument `{}`, require string.", arg)),
                }
            } else if key == "niva" {
                match &value {
                    JsonValue::Object(_) => base = Some(value),
                    _ => {
                        return Err(anyhow!(
                            "Unexpected argument `{}`, require json object.",
                            arg
                        ))
                    }
                }
            } else if key.starts_with("niva.") {
                options.push((key.to_string(), value));
            } else {
                return Err(anyhow!("Unexpected niva argument `{}`.", arg));
            }
        }

        Ok(Self {
            file,
            base,
            options,
        })
    }

    pub fn empty(&self) -> bool {
        self.base.is_none() && self.options.len() <= 0
    }
}
