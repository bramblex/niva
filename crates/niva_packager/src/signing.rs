use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value};

use crate::resources;

#[derive(Debug, Clone)]
pub struct MacSigning {
    identity: String,
    entitlements: Option<String>,
    notarize: Option<Notarization>,
}

#[derive(Debug, Clone)]
enum Notarization {
    Profile(String),
    AppleId { apple_id: String, team_id: String },
}

#[derive(Debug, Clone)]
pub struct WindowsSigning {
    pfx: String,
    timestamp: String,
}

/// Return certificate and entitlement files that must never become application
/// resources, even when a project happens to keep them under its resource tree.
pub fn protected_project_files(config: &Value) -> Result<Vec<String>> {
    let mut files = Vec::new();
    if let Some(macos) = mac_options(config)? {
        if let Some(path) = macos.entitlements {
            files.push(path);
        }
    }
    if let Some(windows) = windows_options(config)? {
        files.push(windows.pfx);
    }
    Ok(files)
}

/// `None` retains the existing verified ad-hoc bundle signature. Formal
/// identity signing is opt-in through `niva.json` and is only run on macOS.
pub fn sign_macos(
    config: &Value,
    project_root: &Path,
    app: &Path,
    staging: &Path,
) -> Result<Option<&'static str>> {
    let Some(options) = mac_options(config)? else {
        return Ok(None);
    };
    ensure!(
        cfg!(target_os = "macos"),
        "macOS identity signing and notarization require a macOS build host"
    );
    let entitlements = options
        .entitlements
        .as_deref()
        .map(|path| resources::safe_file(project_root, path))
        .transpose()?;
    let mut args = vec![
        OsString::from("--deep"),
        OsString::from("--force"),
        OsString::from("--options"),
        OsString::from("runtime"),
        OsString::from("--sign"),
        OsString::from(&options.identity),
    ];
    if let Some(path) = entitlements.as_deref() {
        args.push(OsString::from("--entitlements"));
        args.push(path.as_os_str().to_owned());
    }
    args.push(app.as_os_str().to_owned());
    run_command(Path::new("codesign"), &args, "codesign")?;
    run_command(
        Path::new("codesign"),
        &[
            OsString::from("--verify"),
            OsString::from("--deep"),
            OsString::from("--strict"),
            OsString::from("--verbose=2"),
            app.as_os_str().to_owned(),
        ],
        "codesign verification",
    )?;

    let Some(notarization) = options.notarize else {
        return Ok(Some(if options.identity == "-" {
            "ad-hoc"
        } else {
            "developer-id"
        }));
    };

    let upload_zip = staging.join("notarization-upload.zip");
    run_command(
        Path::new("ditto"),
        &[
            OsString::from("-c"),
            OsString::from("-k"),
            OsString::from("--sequesterRsrc"),
            OsString::from("--keepParent"),
            app.as_os_str().to_owned(),
            upload_zip.as_os_str().to_owned(),
        ],
        "create notarization archive",
    )?;
    let mut notary_args = vec![
        OsString::from("notarytool"),
        OsString::from("submit"),
        upload_zip.as_os_str().to_owned(),
    ];
    match notarization {
        Notarization::Profile(profile) => {
            notary_args.extend([
                OsString::from("--keychain-profile"),
                OsString::from(profile),
            ]);
        }
        Notarization::AppleId { apple_id, team_id } => {
            let password = std::env::var("NIVA_APPLE_ID_PASSWORD")
                .context("NIVA_APPLE_ID_PASSWORD must be supplied in the environment")?;
            notary_args.extend([
                OsString::from("--apple-id"),
                OsString::from(apple_id),
                OsString::from("--team-id"),
                OsString::from(team_id),
                OsString::from("--password"),
                OsString::from(password),
            ]);
        }
    }
    notary_args.extend([OsString::from("--wait")]);
    run_command(Path::new("xcrun"), &notary_args, "notarization submission")?;
    run_command(
        Path::new("xcrun"),
        &[
            OsString::from("stapler"),
            OsString::from("staple"),
            app.as_os_str().to_owned(),
        ],
        "notarization stapling",
    )?;
    run_command(
        Path::new("xcrun"),
        &[
            OsString::from("stapler"),
            OsString::from("validate"),
            app.as_os_str().to_owned(),
        ],
        "notarization ticket verification",
    )?;
    let _ = std::fs::remove_file(upload_zip);
    Ok(Some("notarized"))
}

/// `None` means an unsigned Windows artifact. A configured PFX is never
/// silently ignored: signing requires Windows and NIVA_WIN_CERT_PASSWORD.
pub fn sign_windows(
    config: &Value,
    project_root: &Path,
    executable: &Path,
) -> Result<Option<&'static str>> {
    let Some(options) = windows_options(config)? else {
        return Ok(None);
    };
    ensure!(
        cfg!(target_os = "windows"),
        "Windows Authenticode signing requires a Windows build host"
    );
    let pfx = resources::safe_file(project_root, &options.pfx)?;
    let password = std::env::var("NIVA_WIN_CERT_PASSWORD")
        .context("NIVA_WIN_CERT_PASSWORD must be supplied in the environment")?;
    let signtool = find_signtool().context("Windows SDK signtool.exe was not found")?;
    run_command(
        &signtool,
        &[
            OsString::from("sign"),
            OsString::from("/fd"),
            OsString::from("SHA256"),
            OsString::from("/f"),
            pfx.as_os_str().to_owned(),
            OsString::from("/p"),
            OsString::from(password),
            OsString::from("/tr"),
            OsString::from(&options.timestamp),
            OsString::from("/td"),
            OsString::from("SHA256"),
            executable.as_os_str().to_owned(),
        ],
        "Windows Authenticode signing",
    )?;
    run_command(
        &signtool,
        &[
            OsString::from("verify"),
            OsString::from("/pa"),
            OsString::from("/v"),
            executable.as_os_str().to_owned(),
        ],
        "Windows Authenticode verification",
    )?;
    Ok(Some("authenticode"))
}

fn mac_options(config: &Value) -> Result<Option<MacSigning>> {
    let Some(sign) = object_field(config, "sign", "niva.json")? else {
        return Ok(None);
    };
    ensure_allowed_fields(sign, &["macos", "windows"], "sign")?;
    let Some(options) = object_field_value(sign, "macos", "sign.macos")? else {
        return Ok(None);
    };
    ensure_allowed_fields(
        options,
        &["identity", "entitlements", "notarize"],
        "sign.macos",
    )?;
    let identity = required_string(options, "identity", "sign.macos.identity")?.to_owned();
    ensure!(!identity.is_empty(), "sign.macos.identity cannot be empty");
    let entitlements = optional_string(options, "entitlements", "sign.macos.entitlements")?
        .map(validate_relative_path)
        .transpose()?;
    let notarize = match object_field_value(options, "notarize", "sign.macos.notarize")? {
        None => None,
        Some(notarize) => {
            ensure_allowed_fields(
                notarize,
                &["profile", "appleId", "teamId"],
                "sign.macos.notarize",
            )?;
            let profile = optional_string(notarize, "profile", "sign.macos.notarize.profile")?;
            if let Some(profile) = profile {
                ensure!(
                    !profile.is_empty(),
                    "notarytool keychain profile cannot be empty"
                );
                Some(Notarization::Profile(profile.to_owned()))
            } else {
                let apple_id =
                    required_string(notarize, "appleId", "sign.macos.notarize.appleId")?.to_owned();
                let team_id =
                    required_string(notarize, "teamId", "sign.macos.notarize.teamId")?.to_owned();
                ensure!(
                    !apple_id.is_empty() && !team_id.is_empty(),
                    "notary Apple ID and team ID cannot be empty"
                );
                Some(Notarization::AppleId { apple_id, team_id })
            }
        }
    };
    ensure!(
        notarize.is_none() || identity != "-",
        "ad-hoc signatures cannot be notarized"
    );
    Ok(Some(MacSigning {
        identity,
        entitlements,
        notarize,
    }))
}

fn windows_options(config: &Value) -> Result<Option<WindowsSigning>> {
    let Some(sign) = object_field(config, "sign", "niva.json")? else {
        return Ok(None);
    };
    ensure_allowed_fields(sign, &["macos", "windows"], "sign")?;
    let Some(options) = object_field_value(sign, "windows", "sign.windows")? else {
        return Ok(None);
    };
    ensure_allowed_fields(options, &["pfx", "timestamp"], "sign.windows")?;
    let pfx = validate_relative_path(required_string(options, "pfx", "sign.windows.pfx")?)?;
    let timestamp = optional_string(options, "timestamp", "sign.windows.timestamp")?
        .filter(|value| !value.is_empty())
        .unwrap_or("http://timestamp.digicert.com")
        .to_owned();
    Ok(Some(WindowsSigning { pfx, timestamp }))
}

fn object_field<'a>(
    value: &'a Value,
    name: &str,
    label: &str,
) -> Result<Option<&'a Map<String, Value>>> {
    match value {
        Value::Object(value) => object_field_value(value, name, label),
        Value::Null => Ok(None),
        _ => bail!("{label} must be an object"),
    }
}

fn object_field_value<'a>(
    value: &'a Map<String, Value>,
    name: &str,
    label: &str,
) -> Result<Option<&'a Map<String, Value>>> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value)),
        Some(_) => bail!("{label} must be an object"),
    }
}

fn required_string<'a>(value: &'a Map<String, Value>, name: &str, label: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .with_context(|| format!("{label} must be a string"))
}

fn optional_string<'a>(
    value: &'a Map<String, Value>,
    name: &str,
    label: &str,
) -> Result<Option<&'a str>> {
    match value.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => bail!("{label} must be a string"),
    }
}

fn ensure_allowed_fields(value: &Map<String, Value>, allowed: &[&str], label: &str) -> Result<()> {
    if let Some(unknown) = value.keys().find(|key| !allowed.contains(&key.as_str())) {
        bail!(
            "{label} contains unsupported field {unknown:?}; secret values belong in the environment or keychain"
        );
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<String> {
    ensure!(
        !path.is_empty()
            && !path.contains('\\')
            && !path.contains(':')
            && Path::new(path)
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        "signing asset path must be relative and remain inside the project"
    );
    Ok(path.to_owned())
}

fn find_signtool() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("signtool.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22000.0\x64\signtool.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe"),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate == Path::new("signtool.exe") || candidate.is_file())
}

fn run_command(program: &Path, args: &[OsString], stage: &str) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("unable to start {stage}"))?;
    ensure!(
        output.status.success(),
        "{stage} failed (exit code {:?}); tool output suppressed",
        output.status.code()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_paths_are_protected_but_passwords_cannot_enter_config() {
        let config = serde_json::json!({
            "sign": {
                "macos": {
                    "identity": "Developer ID Application: Example (TEAMID)",
                    "entitlements": "signing/app.entitlements",
                    "notarize": { "profile": "niva-notary" }
                },
                "windows": {
                    "pfx": "signing/app.pfx",
                    "timestamp": "https://timestamp.example"
                }
            }
        });
        assert_eq!(
            protected_project_files(&config).unwrap(),
            ["signing/app.entitlements", "signing/app.pfx"]
        );
        assert!(
            protected_project_files(
                &serde_json::json!({"sign":{"windows":{"pfx":"x.pfx","password":"secret"}}})
            )
            .is_err()
        );
        assert!(
            protected_project_files(
                &serde_json::json!({"sign":{"macos":{"identity":"-","entitlements":"../outside"}}})
            )
            .is_err()
        );
    }

    #[test]
    fn absent_signing_options_do_not_invoke_platform_tools() {
        assert_eq!(mac_options(&serde_json::json!({})).unwrap().is_none(), true);
        assert_eq!(
            windows_options(&serde_json::json!({})).unwrap().is_none(),
            true
        );
    }
}
