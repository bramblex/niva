mod archive;
mod macos;
mod resources;
mod windows;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    #[serde(rename = "windows-x86_64")]
    #[value(name = "windows-x86_64")]
    WindowsX86_64,
    #[serde(rename = "macos-aarch64")]
    #[value(name = "macos-aarch64")]
    MacosAarch64,
    #[serde(rename = "macos-x86_64")]
    #[value(name = "macos-x86_64")]
    MacosX86_64,
}
impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Self::WindowsX86_64 => "windows-x86_64",
            Self::MacosAarch64 => "macos-aarch64",
            Self::MacosX86_64 => "macos-x86_64",
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: String,
    pub runtimes: BTreeMap<Target, Runtime>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub path: PathBuf,
    pub sha256: String,
    pub version: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildResult {
    pub target: Target,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
#[derive(Serialize)]
pub struct Report {
    pub results: Vec<BuildResult>,
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn build(
    manifest_path: &Path,
    config_path: &Path,
    resource_dir: &Path,
    output_dir: &Path,
    targets: &[Target],
    compat: Option<&Path>,
) -> Result<Report> {
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(manifest_path).context("read runtime manifest")?)
            .context("parse runtime manifest")?;
    ensure!(
        manifest.schema_version == 1,
        "unsupported runtime manifest schema"
    );
    ensure!(
        manifest.version == env!("CARGO_PKG_VERSION"),
        "kit version {} does not match packager {}",
        manifest.version,
        env!("CARGO_PKG_VERSION")
    );
    let config_bytes = fs::read(config_path).context("read niva.json")?;
    let config: serde_json::Value =
        serde_json::from_slice(&config_bytes).context("parse niva.json")?;
    let name = config
        .get("name")
        .and_then(|v| v.as_str())
        .context("niva.json requires a name")?;
    validate_name(name)?;
    ensure!(!targets.is_empty(), "select at least one target");
    ensure!(
        targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == targets.len(),
        "duplicate targets"
    );
    let kit_compat = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("node-compat");
    let compat = compat.or_else(|| kit_compat.is_dir().then_some(kit_compat.as_path()));
    let resources = resources::prepare(resource_dir, &config_bytes, &config, compat)?;
    let icon = config
        .get("icon")
        .and_then(|v| v.as_str())
        .map(|name| resources::safe_file(resource_dir, name))
        .transpose()?;
    fs::create_dir_all(output_dir).context("create output directory")?;
    let mut results = Vec::new();
    for &target in targets {
        let mut result = BuildResult {
            target,
            status: "failed",
            path: None,
            sha256: None,
            signature: None,
            runtime_version: None,
            error: None,
        };
        match build_target(
            &manifest,
            manifest_path.parent().unwrap_or(Path::new(".")),
            target,
            name,
            &config,
            &resources,
            icon.as_deref(),
            output_dir,
        ) {
            Ok((path, hash, version)) => {
                result.status = "complete";
                result.path = Some(path);
                result.sha256 = Some(hash);
                result.runtime_version = Some(version);
                result.signature = Some(if target == Target::WindowsX86_64 {
                    "unsigned"
                } else {
                    "ad-hoc"
                });
            }
            Err(error) => result.error = Some(format!("{error:#}")),
        }
        results.push(result);
    }
    Ok(Report { results })
}
#[allow(clippy::too_many_arguments)]
fn build_target(
    manifest: &Manifest,
    root: &Path,
    target: Target,
    name: &str,
    config: &serde_json::Value,
    resources: &resources::Package,
    icon: Option<&Path>,
    output_dir: &Path,
) -> Result<(PathBuf, String, String)> {
    let runtime = manifest
        .runtimes
        .get(&target)
        .context("runtime target missing from manifest")?;
    ensure!(
        runtime.version == manifest.version,
        "runtime version does not match kit version"
    );
    let runtime_path = resources::safe_file(
        root,
        runtime
            .path
            .to_str()
            .context("runtime path must be UTF-8")?,
    )?;
    let bytes = fs::read(&runtime_path)?;
    ensure!(
        runtime.sha256.len() == 64 && runtime.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid runtime SHA256"
    );
    ensure!(
        sha256(&bytes).eq_ignore_ascii_case(&runtime.sha256),
        "runtime SHA256 mismatch"
    );
    validate_architecture(target, &bytes)?;
    let filename = format!(
        "{name}-{}.{}",
        target.name(),
        if target == Target::WindowsX86_64 {
            "exe"
        } else {
            "zip"
        }
    );
    let destination = output_dir.join(filename);
    ensure!(
        !destination.exists(),
        "output already exists: {}",
        destination.display()
    );
    let staging = tempfile::Builder::new()
        .prefix(".niva-packager-")
        .tempdir_in(output_dir)?;
    let artifact = staging.path().join("artifact");
    if target == Target::WindowsX86_64 {
        windows::assemble(
            &runtime_path,
            &artifact,
            config,
            &resources.indexes,
            &resources.data,
            icon,
        )?;
    } else {
        let app = staging.path().join(format!("{name}.app"));
        macos::assemble(
            &runtime_path,
            &app,
            config,
            &resources.indexes,
            &resources.data,
            icon,
        )?;
        archive::zip_app(&app, &artifact)?;
    }
    let hash = sha256(&fs::read(&artifact)?);
    // Hard-link publication is atomic and refuses a concurrent overwrite. Both
    // paths are on the same filesystem because staging is under output_dir.
    fs::hard_link(&artifact, &destination).context("publish artifact without overwriting")?;
    Ok((destination.canonicalize()?, hash, runtime.version.clone()))
}
fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= 120
            && !name.ends_with([' ', '.'])
            && !name
                .chars()
                .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
            && name != "."
            && name != "..",
        "name is not a portable application filename"
    );
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    ensure!(
        ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
        ]
        .contains(&base.as_str()),
        "reserved Windows application filename"
    );
    Ok(())
}
fn validate_architecture(target: Target, bytes: &[u8]) -> Result<()> {
    match target {
        Target::WindowsX86_64 => {
            ensure!(bytes.get(..2) == Some(b"MZ"), "runtime is not PE");
            let offset = bytes.get(0x3c..0x40).context("truncated DOS header")?;
            let offset = u32::from_le_bytes(offset.try_into()?) as usize;
            let header = bytes
                .get(offset..offset.saturating_add(6))
                .context("truncated PE header")?;
            ensure!(
                &header[..4] == b"PE\0\0" && u16::from_le_bytes(header[4..6].try_into()?) == 0x8664,
                "runtime is not Windows x86_64"
            );
        }
        Target::MacosAarch64 | Target::MacosX86_64 => {
            ensure!(
                bytes.get(..4) == Some(&[0xcf, 0xfa, 0xed, 0xfe]),
                "runtime is not a thin 64-bit Mach-O"
            );
            let cpu = u32::from_le_bytes(bytes.get(4..8).context("truncated Mach-O")?.try_into()?);
            let expected = if target == Target::MacosAarch64 {
                0x0100000c
            } else {
                0x01000007
            };
            if cpu != expected {
                bail!("runtime architecture does not match {}", target.name());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_cannot_escape_or_use_windows_device_names() {
        for name in ["../x", "a/b", "a\\b", "CON", "nul.txt", "hello.", "a:", ""] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        assert!(validate_name("测试 App").is_ok());
    }
    #[test]
    fn rejects_mismatched_and_truncated_runtimes() {
        assert!(validate_architecture(Target::WindowsX86_64, b"MZ").is_err());
        let arm = [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
        assert!(validate_architecture(Target::MacosAarch64, &arm).is_ok());
        assert!(validate_architecture(Target::MacosX86_64, &arm).is_err());
    }
}
