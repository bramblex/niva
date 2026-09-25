mod archive;
mod macos;
mod resources;
mod signing;
mod windows;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_RUNTIME_BYTES: u64 = 3_300_000;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceLayout {
    /// Compress business resources into the runtime (Windows EXE / macOS app).
    #[default]
    Embedded,
    /// Keep business files outside the runtime and read them on demand.
    External,
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
    pub resource_layout: ResourceLayout,
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

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn build(
    manifest_path: &Path,
    config_path: &Path,
    resource_dir: &Path,
    output_dir: &Path,
    targets: &[Target],
    resource_layout: ResourceLayout,
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
    let project_root = config_path.parent().unwrap_or(Path::new("."));
    let protected_files = signing::protected_project_files(&config)?;
    ensure!(!targets.is_empty(), "select at least one target");
    ensure!(
        targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == targets.len(),
        "duplicate targets"
    );
    let kit_root = manifest_path.parent().unwrap_or(Path::new("."));
    resources::validate_licenses(kit_root)?;
    fs::create_dir_all(output_dir).context("create output directory")?;
    let build_staging = tempfile::Builder::new()
        .prefix(".niva-packager-build-")
        .tempdir_in(output_dir)?;
    let resources = resources::prepare(
        resource_dir,
        &config_bytes,
        &config,
        resource_layout == ResourceLayout::External,
        build_staging.path(),
        &protected_files,
        Some(kit_root),
    )?;
    let icon = config
        .get("icon")
        .and_then(|v| v.as_str())
        .map(|name| {
            let snapshot = build_staging.path().join("icon.png");
            resources::snapshot_file(resource_dir, name, &snapshot)?;
            Ok::<PathBuf, anyhow::Error>(snapshot)
        })
        .transpose()?;
    let mut results = Vec::new();
    for &target in targets {
        let mut result = BuildResult {
            target,
            resource_layout,
            status: "failed",
            path: None,
            sha256: None,
            signature: None,
            runtime_version: None,
            error: None,
        };
        match build_target(
            &manifest,
            kit_root,
            project_root,
            target,
            name,
            &config,
            &resources,
            icon.as_deref(),
            output_dir,
            build_staging.path(),
            resource_layout,
        ) {
            Ok((path, hash, version, signature)) => {
                result.status = "complete";
                result.path = Some(path);
                result.sha256 = Some(hash);
                result.runtime_version = Some(version);
                result.signature = Some(signature);
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
    project_root: &Path,
    target: Target,
    name: &str,
    config: &serde_json::Value,
    resources: &resources::Package,
    icon: Option<&Path>,
    output_dir: &Path,
    build_staging: &Path,
    resource_layout: ResourceLayout,
) -> Result<(PathBuf, String, String, &'static str)> {
    let runtime = manifest
        .runtimes
        .get(&target)
        .context("runtime target missing from manifest")?;
    ensure!(
        runtime.version == manifest.version,
        "runtime version does not match kit version"
    );
    let runtime_relative = runtime
        .path
        .to_str()
        .context("runtime path must be UTF-8")?;
    let runtime_path = build_staging.join(format!("runtime-{}", target.name()));
    resources::snapshot_file(root, runtime_relative, &runtime_path)?;
    ensure!(
        fs::metadata(&runtime_path)?.len() < MAX_RUNTIME_BYTES,
        "runtime exceeds the strict {} byte size limit",
        MAX_RUNTIME_BYTES
    );
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
    let is_windows = target == Target::WindowsX86_64;
    let extension = if is_windows && resource_layout == ResourceLayout::Embedded {
        "exe"
    } else {
        "zip"
    };
    let suffix = if is_windows && resource_layout == ResourceLayout::External {
        "-green"
    } else {
        ""
    };
    let filename = format!("{name}-{}{suffix}.{extension}", target.name());
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
    let signature = if is_windows && resource_layout == ResourceLayout::Embedded {
        windows::assemble(&runtime_path, &artifact, config, resources, icon)?;
        signing::sign_windows(config, project_root, &artifact)?.unwrap_or("unsigned")
    } else if is_windows {
        let package_root = staging.path().join("portable");
        fs::create_dir(&package_root)?;
        let executable = package_root.join(format!("{name}.exe"));
        windows::assemble(&runtime_path, &executable, config, resources, icon)?;
        let signature =
            signing::sign_windows(config, project_root, &executable)?.unwrap_or("unsigned");
        resources::copy_external_resources(resources, &package_root.join("resources"))?;
        archive::zip_directory(&package_root, &artifact, "resources/")?;
        signature
    } else {
        let app = staging.path().join(format!("{name}.app"));
        let signature = macos::assemble(
            &runtime_path,
            &app,
            config,
            resources,
            icon,
            project_root,
            staging.path(),
        )?;
        let stored_resource_prefix = (resource_layout == ResourceLayout::External)
            .then(|| format!("{name}.app/Contents/Resources/app/"));
        archive::zip_app(&app, &artifact, stored_resource_prefix.as_deref())?;
        signature
    };
    let hash = sha256_file(&artifact)?;
    // Hard-link publication is atomic and refuses a concurrent overwrite. Both
    // paths are on the same filesystem because staging is under output_dir.
    fs::hard_link(&artifact, &destination).context("publish artifact without overwriting")?;
    Ok((
        destination.canonicalize()?,
        hash,
        runtime.version.clone(),
        signature,
    ))
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
