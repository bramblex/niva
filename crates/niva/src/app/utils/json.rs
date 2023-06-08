use anyhow::{anyhow, Result};
use serde_json::{Map as JsonMap, Value as JsonValue};

pub fn merge_json_value(dest: &mut JsonValue, src: JsonValue) {
    if dest.is_object() && src.is_object() {
        if let JsonValue::Object(dest_map) = dest {
            if let JsonValue::Object(src_map) = src {
                for (key, src_val) in src_map {
                    let dest_val = dest_map.entry(key).or_insert(JsonValue::Null);
                    merge_json_value(dest_val, src_val);
                }
            }
        }
    } else if !src.is_null() {
        *dest = src;
    }
}

enum JsonPathItem<'a> {
    Key(&'a str),
    Index(usize),
}

type JsonPath<'a> = Vec<JsonPathItem<'a>>;

fn parse_json_path<'a>(path_str: &'a str) -> Result<JsonPath<'a>> {
    let mut path = JsonPath::new();
    let mut it = path_str.split('.');

    while let Some(item) = it.next() {
        if item.len() <= 0 {
            return Err(anyhow!("Unexpected json path `{}`.", path_str));
        }
        let parsed_result = item.parse::<usize>();
        match parsed_result {
            Ok(index) => path.push(JsonPathItem::Index(index)),
            Err(_) => path.push(JsonPathItem::Key(item)),
        }
    }

    Ok(path)
}

fn _set_json_value(target: &mut JsonValue, path: &[&JsonPathItem], value: JsonValue) -> Result<()> {
    if path.len() == 0 {
        *target = value;
    } else {
        let item = path[0];
        let next_path = &path[1..path.len()];

        match item {
            JsonPathItem::Key(key) => {
                if target.is_null() {
                    *target = JsonValue::Object(JsonMap::new());
                }
                if let Some(obj) = target.as_object_mut() {
                    let next = &mut obj.entry(key.to_string()).or_insert(JsonValue::Null);
                    _set_json_value(next, next_path, value)?;
                } else {
                    return Err(anyhow!("Cannot set value."));
                }
            }
            JsonPathItem::Index(index) => {
                if target.is_null() {
                    *target = JsonValue::Array(vec![]);
                }
                if let Some(arr) = target.as_array_mut() {
                    if *index >= arr.len() {
                        arr.resize(index + 1, JsonValue::Null);
                    }
                    let next = &mut arr[*index];
                    _set_json_value(next, next_path, value)?;
                } else {
                    return Err(anyhow!("Cannot set value."));
                }
            }
        }
    }
    Ok(())
}

pub fn set_json_value(target: &mut JsonValue, path_str: &str, value: JsonValue) -> Result<()> {
    let path = parse_json_path(path_str)?;
    let path = path.iter().collect::<Vec<&JsonPathItem>>();
    return _set_json_value(target, &path, value);
}