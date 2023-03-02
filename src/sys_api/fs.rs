use std::time::UNIX_EPOCH;

use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;

pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "stat" => stat(request),
        "exists" => exists(request),

        "read" => read(request),
        "write" => write(request),

        "mv" => mv(request),
        "cp" => cp(request),
        "rm" => rm(request),

        "ls" => ls(request),
        "mkDir" => mk_dir(request),
        "rmDir" => rm_dir(request),

        _ => ApiResponse::err("Method not found".to_string()),
    };
}

#[derive(Deserialize)]
struct LsOptions {
    pub path: Option<String>,
}

fn ls(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<LsOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path.unwrap_or(".".to_string());

    // list dir
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        let file_type = match path.is_dir() {
            true => "dir",
            false => "file",
        };
        entries.push(json!({
            "name": file_name,
            "type": file_type
        }));
    }

    return ApiResponse::ok(json!({ "entries": entries }));
}

#[derive(Deserialize)]
struct ReadOptions {
    pub path: String,
}

fn read(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let content_result = std::fs::read_to_string(path);

    if content_result.is_err() {
        return ApiResponse::err("Cannot read file".to_string());
    }

    return ApiResponse::ok(json!({
        "content": content_result.unwrap()
    }));
}

#[derive(Deserialize)]
struct WriteOptions {
    pub path: String,
    pub content: String,
}

fn write(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<WriteOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let options = data_result.unwrap();

    let write_result = std::fs::write(options.path, options.content);

    if write_result.is_err() {
        return ApiResponse::err("Cannot write file".to_string());
    }

    return ApiResponse::ok(json!({}));
}

fn exists(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let exists = std::path::Path::new(&path).exists();

    return ApiResponse::ok(json!({ "exists": exists }));
}

fn stat(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let meta_result = std::fs::metadata(path);
    if meta_result.is_err() {
        return ApiResponse::err("Cannot read file meta".to_string());
    }

    let meta = meta_result.unwrap();

    return ApiResponse::ok(json!({
        "metadata": {
            "isDir": meta.is_dir(),
            "isFile": meta.is_file(),
            "isSymlink": meta.file_type().is_symlink(),
            "size": meta.len(),
            "modified": meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            "accessed": meta.accessed().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            "created": meta.created().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis(),
        }
    }));
}

#[derive(Deserialize)]
struct MvOptions {
    pub from: String,
    pub to: String,
}

fn mv(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<MvOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let options = data_result.unwrap();

    let mv_result = std::fs::rename(options.from, options.to);

    if mv_result.is_err() {
        return ApiResponse::err("Cannot move file".to_string());
    }

    return ApiResponse::ok(json!({}));
}

fn cp(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<MvOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let options = data_result.unwrap();

    let cp_result = std::fs::copy(options.from, options.to);

    if cp_result.is_err() {
        return ApiResponse::err("Cannot copy file".to_string());
    }

    return ApiResponse::ok(json!({}));
}

fn rm(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let rm_result = std::fs::remove_file(path);

    if rm_result.is_err() {
        return ApiResponse::err("Cannot remove file".to_string());
    }

    return ApiResponse::ok(json!({}));
}

fn mk_dir(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let mkdir_result = std::fs::create_dir_all(path);

    if mkdir_result.is_err() {
        return ApiResponse::err("Cannot create directory".to_string());
    }

    return ApiResponse::ok(json!({}));
}

fn rm_dir(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<ReadOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let path = data_result.unwrap().path;
    let rmdir_result = std::fs::remove_dir_all(path);

    if rmdir_result.is_err() {
        return ApiResponse::err("Cannot remove directory".to_string());
    }

    return ApiResponse::ok(json!({}));
}
