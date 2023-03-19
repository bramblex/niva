use anyhow::Result;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    api_manager::{ApiManager, ApiRequest},
    environment::EnvironmentRef,
};
use std::{time::UNIX_EPOCH, path::Path};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("fs.stat", stat);
    api_manager.register_async_api("fs.exists", exists);
    api_manager.register_async_api("fs.read", read);
    api_manager.register_async_api("fs.write", write);
    api_manager.register_async_api("fs.copy", copy);
    api_manager.register_async_api("fs.move", move_);
    api_manager.register_async_api("fs.remove", remove);
    api_manager.register_async_api("fs.createDir", create_dir);
    api_manager.register_async_api("fs.createDirAll", create_dir_all);
    api_manager.register_async_api("fs.readDir", read_dir);
    api_manager.register_async_api("fs.readDirAll", read_dir_all);
}


fn stat(_: EnvironmentRef, request: ApiRequest) -> Result<Value> {
    let path = request.args().get_single::<String>()?;
    let meta = std::fs::metadata(path)?;

    Ok(json!({
        "isDir": meta.is_dir(),
        "isFile": meta.is_file(),
        "isSymlink": meta.file_type().is_symlink(),
        "size": meta.len(),
        "modified": meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis(),
        "accessed": meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis(),
        "created": meta.created()?.duration_since(UNIX_EPOCH)?.as_millis(),
    }))
}

fn exists(_: EnvironmentRef, request: ApiRequest) -> Result<bool> {
    let path = request.args().get_single::<String>()?;
    let path = std::path::Path::new(&path);
    Ok(path.exists())
}

#[derive(Deserialize)]
enum EncodeType {
    #[serde(rename = "utf8")]
    UTF8,
    #[serde(rename = "base64")]
    BASE64,
}

fn read(_: EnvironmentRef, request: ApiRequest) -> Result<String> {
    let (path, encode) = request
        .args()
        .get_with_optional::<(String, Option<EncodeType>)>(2)?;

    let encode = encode.unwrap_or(EncodeType::UTF8);
    let content = match encode {
        EncodeType::UTF8 => std::fs::read_to_string(path)?,
        EncodeType::BASE64 => {
            let content = std::fs::read(path)?;
            base64::engine::general_purpose::STANDARD.encode(content)
        }
    };

    Ok(content)
}

fn write(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let (path, content, encode) = request
        .args()
        .get_with_optional::<(String, String, Option<EncodeType>)>(3)?;
    let encode = encode.unwrap_or(EncodeType::UTF8);

    match encode {
        EncodeType::BASE64 => {
            let content = base64::engine::general_purpose::STANDARD.decode(content)?;
            std::fs::write(path, content)?
        }
        EncodeType::UTF8 => std::fs::write(path, content)?,
    };
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopyOptions {
    pub overwrite: Option<bool>,
    pub skip_exist: Option<bool>,
    pub buffer_size: Option<usize>,
    pub copy_inside: Option<bool>,
    pub content_only: Option<bool>,
    pub depth: Option<u64>,
}

fn _create_file_copy_options(options: Option<CopyOptions>) -> fs_extra::file::CopyOptions {
    match options {
        Some(options) => fs_extra::file::CopyOptions {
            overwrite: options.overwrite.unwrap_or(false),
            skip_exist: options.skip_exist.unwrap_or(false),
            buffer_size: options.buffer_size.unwrap_or(64000),
        },
        None => fs_extra::file::CopyOptions::default(),
    }
}

fn _create_dir_copy_options(options: Option<CopyOptions>) -> fs_extra::dir::CopyOptions {
    match options {
        Some(options) => fs_extra::dir::CopyOptions {
            overwrite: options.overwrite.unwrap_or(false),
            skip_exist: options.skip_exist.unwrap_or(false),
            buffer_size: options.buffer_size.unwrap_or(64000),
            copy_inside: options.copy_inside.unwrap_or(false),
            content_only: options.content_only.unwrap_or(false),
            depth: options.depth.unwrap_or(0),
        },
        None => fs_extra::dir::CopyOptions::default(),
    }
}

fn move_(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let (from, to, options) = request
        .args()
        .get_with_optional::<(String, String, Option<CopyOptions>)>(3)?;
    let from = std::path::Path::new(&from);

    if from.is_dir() {
        use fs_extra::dir;
        let options = _create_dir_copy_options(options);
        dir::move_dir(from, to, &options)?;
    } else {
        use fs_extra::file;
        let options = _create_file_copy_options(options);
        file::move_file(from, to, &options)?;
    }
    Ok(())
}

fn copy(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let (from, to, options) = request
        .args()
        .get_with_optional::<(String, String, Option<CopyOptions>)>(3)?;
    let from = std::path::Path::new(&from);

    if from.is_dir() {
        use fs_extra::dir;
        let options = _create_dir_copy_options(options);
        dir::copy(from, to, &options)?;
    } else {
        use fs_extra::file;
        let options = _create_file_copy_options(options);
        file::copy(from, to, &options)?;
    }

    Ok(())
}

fn remove(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let path = request.args().get_single::<String>()?;
    let path = std::path::Path::new(&path);

    if path.is_dir() {
        fs_extra::dir::remove(path)?
    } else {
        fs_extra::file::remove(path)?
    };

    Ok(())
}

fn create_dir(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let path = request.args().get_single::<String>()?;
    std::fs::create_dir(path)?;
    Ok(())
}

fn create_dir_all(_: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let path = request.args().get_single::<String>()?;
    std::fs::create_dir_all(path)?;
    Ok(())
}

fn read_dir(_: EnvironmentRef, request: ApiRequest) -> Result<Vec<String>> {
    let (path,) = request.args().get_with_optional::<(Option<String>,)>(1)?;
    let path = path.unwrap_or(".".to_string());

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        entries.push(file_name);
    }
    Ok(entries)
}

fn _visit_dirs(dir: &Path, prefix: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel_path = path.strip_prefix(prefix).unwrap();
        if path.is_dir() {
            _visit_dirs(&path, prefix, files)?;
        } else {
            files.push(rel_path.to_str().unwrap().to_string());
        }
    }
    Ok(())
}

fn read_dir_all(_: EnvironmentRef, request: ApiRequest) -> Result<Vec<String>> {
    let path = request.args().get_single::<String>()?;
    let path = Path::new(&path);
    let mut files: Vec<String> = Vec::new();
    _visit_dirs(path, path, &mut files)?;
    Ok(files)
}