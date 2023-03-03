use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;

pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "pid" => pid(request),
        "cwd" => cwd(request),
        "chDir" => ch_dir(request),
        "env" => env(request),
        "exit" => exit(request),
        "exec" => exec(request),
        _ => ApiResponse::err(request.callback_id,"Method not found".to_string()),
    };
}

pub fn pid(request: ApiRequest) -> ApiResponse {
    let pid = std::process::id();
    ApiResponse::ok(
        request.callback_id,
        json!({
                "pid": pid,
        }),
    )
}

pub fn cwd(request: ApiRequest) -> ApiResponse {
    let cwd = std::env::current_dir().unwrap();
    ApiResponse::ok(
        request.callback_id,
        json!({
                "cwd": cwd,
        }),
    )
}

pub fn env(request: ApiRequest) -> ApiResponse {
    let env = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    ApiResponse::ok(
        request.callback_id,
        json!({
                "env": env,
        }),
    )
}

#[derive(Deserialize)]
struct ChDirOptions {
    pub path: String,
}

pub fn ch_dir(request: ApiRequest) -> ApiResponse {
    let options_result = serde_json::from_value::<ChDirOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id,"Invalid options".to_string());
    }
    let path = options_result.unwrap().path;
    let result = std::env::set_current_dir(path);
    if result.is_err() {
        return ApiResponse::err(request.callback_id,"Failed to change directory".to_string());
    }
    ApiResponse::ok(request.callback_id, json!({}))
}

pub fn exit(request: ApiRequest) -> ! {
    std::process::exit(0);
}

#[derive(Deserialize)]
struct ExecOptions {
    pub command: String,
    pub cwd: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub detached: Option<bool>,
}

pub fn exec(request: ApiRequest) -> ApiResponse {
    // 执行命令
    let options_result = serde_json::from_value::<ExecOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id,"Invalid options".to_string());
    }
    let options = options_result.unwrap();
    let mut command = std::process::Command::new(options.command);

    // 设置工作目录
    if options.cwd.is_some() {
        command.current_dir(options.cwd.unwrap());
    }

    // 设置参数
    if options.args.is_some() {
        command.args(options.args.unwrap());
    }

    // 设置环境变量
    if options.env.is_some() {
        command.envs(options.env.unwrap());
    }

    // detach child process
    if options.detached.unwrap_or(false) {
        let result = command.spawn();
        match result {
            Ok(child) => {
                return ApiResponse::ok(
                    request.callback_id,
                    json!({
                        "pid": child.id(),
                    }),
                )
            }
            Err(_) => return ApiResponse::err(request.callback_id,"Failed to exec command".to_string()),
        }
    }

    let output_result = command.output();
    if output_result.is_err() {
        return ApiResponse::err(request.callback_id,"Failed to exec command".to_string());
    }
    let output = output_result.unwrap();

    ApiResponse::ok(
        request.callback_id,
        json!({
            "status": output.status.code(),
            "stdout": String::from_utf8(output.stdout).unwrap(),
            "stderr": String::from_utf8(output.stderr).unwrap(),
        }),
    )
}
