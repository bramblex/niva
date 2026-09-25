//! Immutable startup metadata. Current working directory and other changing
//! OS observations deliberately remain native queries.

use serde_json::{Value, json};
use std::{collections::BTreeMap, io::IsTerminal, path::PathBuf, sync::OnceLock};

use crate::app::os_native as native;

pub(crate) fn metadata(main_window: bool) -> Value {
    static STARTUP: OnceLock<Value> = OnceLock::new();
    let mut data = STARTUP.get_or_init(snapshot).clone();
    if !main_window {
        data.as_object_mut().unwrap().remove("process");
    }
    data
}

fn snapshot() -> Value {
    let platform = node_platform(std::env::consts::OS);
    let arch = node_arch(std::env::consts::ARCH);
    let version = native::version().unwrap_or_else(|| {
        let info = os_info::get();
        native::OsVersion {
            os_type: info.os_type().to_string(),
            release: info.version().to_string(),
            version: info.to_string(),
        }
    });
    let user_info = native::user_info();
    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_owned())
        .or_else(|| {
            user_info
                .as_ref()
                .and_then(|info| info.get("homedir"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
        });
    let env: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let argv: Vec<String> = std::env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    let mut os = json!({
        "info": crate::app::api::os::startup_info(),
        "platform": platform,
        "arch": arch,
        "homedir": home,
        "tmpdir": std::env::temp_dir(),
        "type": version.os_type,
        "release": version.release,
        "version": version.version,
        "EOL": if platform == "win32" { "\r\n" } else { "\n" },
    });
    if let Some(hostname) = native::hostname() {
        os["hostname"] = json!(hostname);
    }
    if let Some(totalmem) = native::total_memory() {
        os["totalmem"] = json!(totalmem);
    }
    if let Some(user_info) = user_info {
        os["userInfo"] = user_info;
    }

    json!({
        "os": os,
        "process": {
            "arch": arch,
            "platform": platform,
            "argv0": argv.first(),
            "argv": argv,
            "env": env,
            "execPath": std::env::current_exe().ok(),
            "pid": std::process::id(),
            "stdioIsTTY": {
                "stdin": std::io::stdin().is_terminal(),
                "stdout": std::io::stdout().is_terminal(),
                "stderr": std::io::stderr().is_terminal(),
            },
            // Node libraries use these fields for API-version branches. They
            // describe our declared compatibility target, not an embedded V8
            // or Node engine. The actual product version remains separate.
            "version": "v22.14.0",
            "versions": { "niva": env!("CARGO_PKG_VERSION"), "node": "22.14.0", "nodeCompat": "22.14.0" },
        },
    })
}

fn node_platform(platform: &str) -> &str {
    match platform {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn node_arch(arch: &str) -> &str {
    match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_metadata_uses_node_platform_and_arch_names() {
        assert_eq!(node_platform("macos"), "darwin");
        assert_eq!(node_platform("windows"), "win32");
        assert_eq!(node_arch("x86_64"), "x64");
        assert_eq!(node_arch("aarch64"), "arm64");
        assert_eq!(node_arch("riscv64"), "riscv64");
    }

    #[test]
    fn non_main_metadata_has_no_process_information() {
        assert!(metadata(false).get("process").is_none());
        assert!(metadata(true)["process"]["pid"].is_number());
        assert!(metadata(true)["process"]["stdioIsTTY"].is_object());
        for key in ["stdin", "stdout", "stderr"] {
            assert!(metadata(true)["process"]["stdioIsTTY"][key].is_boolean());
        }
        assert_eq!(metadata(true)["process"]["version"], "v22.14.0");
        assert_eq!(metadata(true)["process"]["versions"]["node"], "22.14.0");
        assert_eq!(
            metadata(true)["process"]["versions"]["niva"],
            env!("CARGO_PKG_VERSION")
        );
        assert!(metadata(true)["os"].get("cwd").is_none());
        for key in [
            "arch", "platform", "homedir", "tmpdir", "hostname", "release", "totalmem", "type",
            "version", "userInfo",
        ] {
            assert!(metadata(true)["os"].get(key).is_some(), "missing os.{key}");
        }
        assert!(metadata(true)["os"]["hostname"].is_string());
        assert!(metadata(true)["os"]["totalmem"].is_number());
        assert!(metadata(true)["os"]["userInfo"].is_object());
    }
}
