use anyhow::{anyhow, Result};
use serde_json::{Map as JsonMap, Value as JsonValue};

pub fn merge_json_value(dest: JsonValue, src: JsonValue) -> JsonValue {
    match (dest, src) {
        (JsonValue::Null, src) => src,
        (dest, JsonValue::Null) => dest,
        (JsonValue::Object(mut dest_map), JsonValue::Object(src_map)) => {
            for (key, src_val) in src_map {
                let dest_val = dest_map.entry(key).or_insert(JsonValue::Null);
                *dest_val = merge_json_value(dest_val.take(), src_val);
            }
            JsonValue::Object(dest_map)
        }
        (_, src) => src,
    }
}

pub fn set_json_value(target: &mut JsonValue, path: &str, value: &JsonValue) -> Result<()> {
    let keys: Vec<&str> = path.split('.').collect();
    let last_key = keys.last().copied();

    let mut current = target;

    for key in keys {
        let is_last_key = Some(key) == last_key;

        if let Some(obj) = current.as_object_mut() {
            if is_last_key {
                // 最后一个键，设置值
                obj.insert(key.to_string(), value.clone());
            } else {
                // 获取或创建子对象
                current = obj
                    .entry(key.to_string())
                    .or_insert(JsonValue::Object(JsonMap::new()));
            }
        } else if let Some(arr) = current.as_array_mut() {
            if let Ok(index) = key.parse::<usize>() {
                if is_last_key {
                    // 最后一个键，设置值
                    if index >= arr.len() {
                        arr.resize(index + 1, JsonValue::Null);
                    }
                    arr[index] = value.clone();
                } else {
                    // 获取或创建子对象
                    if index >= arr.len() {
                        arr.resize(index + 1, JsonValue::Object(JsonMap::new()));
                    }
                    current = &mut arr[index];
                }
            } else {
                // 非数字索引，无法继续设置值
                return Err(anyhow!("Invalid config path: {}", path));
            }
        } else {
            // 非对象或数组类型，无法继续设置值
            return Err(anyhow!("Invalid config path: {}", path));
        }
    }
    Ok(())
}
