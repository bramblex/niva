//! Windows PE resource helpers. The production application packaging path is
//! `niva-packager`, which uses this crate's ICO and VERSIONINFO encoders. This
//! crate's standalone CLI remains available for low-level Windows smoke use.
//! Resource preparation can run on other hosts; [`pack`] writes PE resources
//! only on Windows.

use anyhow::{Context, Result, anyhow, bail};
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub mod bundle;
#[cfg(any(windows, test))]
#[path = "../../shared/windows_file_identity.rs"]
mod file_identity;
#[cfg(test)]
mod file_identity_tests;
pub mod icon;
pub mod version_info;

#[cfg(target_os = "windows")]
mod windows_impl;

/// 默认语言 ID：与现有 ResourceHacker 脚本保持一致（英语-美国）。
pub const DEFAULT_LANG: u16 = 1033;
/// 默认图标组 ID：与现有脚本的 `ICONGROUP,1` 保持一致。
pub const DEFAULT_ICON_GROUP_ID: u16 = 1;

pub(crate) fn validate_windows_path_no_nul(path: &std::path::Path) -> anyhow::Result<()> {
    if path.as_os_str().to_string_lossy().contains('\0') {
        return Err(anyhow!("Windows path contains NUL"));
    }
    Ok(())
}

/// Return whether two paths can name the same file, including existing hard
/// links and output paths whose final parent components have not been created.
pub fn paths_refer_to_same_file(input: &Path, output: &Path) -> Result<bool> {
    let input_abs = std::fs::canonicalize(input)
        .with_context(|| format!("resolve input file {}", input.display()))?;
    let output_abs = resolve_path_with_missing_suffix(output)?;
    if paths_have_same_name(&input_abs, &output_abs) {
        return Ok(true);
    }
    if output.exists() {
        return same_file(input, output);
    }
    Ok(false)
}

fn resolve_path_with_missing_suffix(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut resolved = PathBuf::new();
    let mut missing = Vec::new();
    for component in absolute.components() {
        match component {
            std::path::Component::Prefix(prefix) => resolved.push(prefix.as_os_str()),
            std::path::Component::RootDir => resolved.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if missing.pop().is_none() {
                    resolved.pop();
                }
            }
            std::path::Component::Normal(part) => {
                if missing.is_empty() {
                    let candidate = resolved.join(part);
                    match std::fs::canonicalize(&candidate) {
                        Ok(canonical) => resolved = canonical,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            missing.push(part.to_os_string());
                        }
                        Err(error) => {
                            return Err(error).with_context(|| {
                                format!("resolve output path component {}", candidate.display())
                            });
                        }
                    }
                } else {
                    missing.push(part.to_os_string());
                }
            }
        }
    }
    for component in missing {
        resolved.push(component);
    }
    Ok(resolved)
}

fn paths_have_same_name(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn same_file(left: &Path, right: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let left_meta =
            std::fs::metadata(left).with_context(|| format!("inspect file {}", left.display()))?;
        let right_meta = std::fs::metadata(right)
            .with_context(|| format!("inspect file {}", right.display()))?;
        if left_meta.ino() == 0 || right_meta.ino() == 0 {
            bail!("cannot safely determine input/output file identity");
        }
        Ok(left_meta.dev() == right_meta.dev() && left_meta.ino() == right_meta.ino())
    }
    #[cfg(windows)]
    {
        use std::fs::File;

        let left_file =
            File::open(left).with_context(|| format!("open file {}", left.display()))?;
        let right_file =
            File::open(right).with_context(|| format!("open file {}", right.display()))?;
        let left_id = file_identity::from_file(&left_file)
            .context("cannot determine input file identity")?
            .context("input file identity is unknown")?;
        let right_id = file_identity::from_file(&right_file)
            .context("cannot determine output file identity")?
            .context("output file identity is unknown")?;
        Ok(left_id == right_id)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (left, right);
        bail!("cannot safely determine input/output file identity on this platform")
    }
}

/// Compare an already-opened file handle with a path that was re-resolved
/// after opening. On Windows this uses the stable file ID API rather than
/// unstable std `MetadataExt` file-index methods.
pub fn opened_file_matches_path(
    file: &std::fs::File,
    path: &std::path::Path,
) -> anyhow::Result<bool> {
    #[cfg(windows)]
    {
        return file_identity::opened_file_matches_path(file, path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let current = std::fs::File::open(path)?;
        let opened = file.metadata()?;
        let current = current.metadata()?;
        Ok(opened.dev() == current.dev() && opened.ino() == current.ino())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let current = std::fs::File::open(path)?;
        let opened = file.metadata()?;
        let current = current.metadata()?;
        Ok(opened.len() == current.len() && opened.modified().ok() == current.modified().ok())
    }
}

/// 一条 `RCDATA` 资源（低层模式）：`name` 是 exe 内的资源名
/// （如 `RESOURCE_INDEXES`），`file` 是磁盘上已备好的资源文件。
#[derive(Debug, Clone)]
pub struct RcDataEntry {
    pub name: String,
    pub file: PathBuf,
}

/// 图标替换项（低层模式）：`ico_file` 是现成的多尺寸 `.ico` 文件。
#[derive(Debug, Clone)]
pub struct IconEntry {
    pub ico_file: PathBuf,
    pub group_id: u16,
}

/// 一次打包请求。
///
/// Bundle mode: `resource_dir` + `config_file` + optional `icon_png`.
/// Low-level `rcdata` / `icon` inputs remain available for smoke fixtures and
/// callers with prebuilt resource data.
#[derive(Debug, Clone)]
pub struct PackRequest {
    /// 模板 exe（即 `process.currentExe()`）。
    pub template_exe: PathBuf,
    /// 产物 exe。必须与模板不同，签名要在打包后做。
    pub output_exe: PathBuf,
    /// 高层：项目 build 资源目录（对应 `config.build.resource`）。
    pub resource_dir: Option<PathBuf>,
    /// 高层：`niva.json` 路径，包内固定为 `"niva.json"`。与 `resource_dir` 成对出现。
    pub config_file: Option<PathBuf>,
    /// 高层：PNG 图标源（对应 `config.icon` 指向的文件），crate 内转 ICO。
    /// 与 `icon` 互斥。
    pub icon_png: Option<PathBuf>,
    /// 低层：已备好的 `RCDATA` 条目（`NAME=FILE`）。
    pub rcdata: Vec<RcDataEntry>,
    /// 低层：现成 `.ico` 直接写。与 `icon_png` 互斥。
    pub icon: Option<IconEntry>,
    /// 替换图标前要删除的旧 `ICON` id（对应脚本 `-delete ICON,n`）。
    /// 为空表示不删除。
    pub delete_icon_ids: Vec<u16>,
    /// 可选 `VERSIONINFO` 的 `.rc` 文本文件
    ///（`windows-version-info-template.ts` 的产物）。`None` 表示不动版本信息。
    pub version_info_rc: Option<PathBuf>,
    /// 资源语言 ID，默认 1033。
    pub lang: u16,
}

impl Default for PackRequest {
    fn default() -> Self {
        Self {
            template_exe: PathBuf::new(),
            output_exe: PathBuf::new(),
            resource_dir: None,
            config_file: None,
            icon_png: None,
            rcdata: Vec::new(),
            icon: None,
            delete_icon_ids: Vec::new(),
            version_info_rc: None,
            lang: DEFAULT_LANG,
        }
    }
}

impl PackRequest {
    pub fn validate(&self) -> Result<()> {
        validate_windows_path_no_nul(&self.template_exe)?;
        validate_windows_path_no_nul(&self.output_exe)?;
        let has_bundle = self.resource_dir.is_some() || self.config_file.is_some();
        if !has_bundle
            && self.rcdata.is_empty()
            && self.icon.is_none()
            && self.icon_png.is_none()
            && self.version_info_rc.is_none()
        {
            return Err(anyhow!(
                "nothing to pack: no resource_dir/rcdata/icon/icon_png/version_info"
            ));
        }
        if self.resource_dir.is_some() != self.config_file.is_some() {
            return Err(anyhow!(
                "resource_dir and config_file must be given together"
            ));
        }
        if let Some(d) = &self.resource_dir
            && !d.is_dir()
        {
            return Err(anyhow!("resource dir not found: {}", d.display()));
        }
        if let Some(f) = &self.config_file
            && !f.is_file()
        {
            return Err(anyhow!("config file not found: {}", f.display()));
        }
        if self.icon.is_some() && self.icon_png.is_some() {
            return Err(anyhow!("icon and icon_png are exclusive"));
        }
        for e in &self.rcdata {
            if e.name.is_empty() {
                return Err(anyhow!("rcdata entry with empty name"));
            }
            if !e.file.is_file() {
                return Err(anyhow!("rcdata file not found: {}", e.file.display()));
            }
        }
        if let Some(icon) = &self.icon
            && !icon.ico_file.is_file()
        {
            return Err(anyhow!("icon file not found: {}", icon.ico_file.display()));
        }
        if let Some(p) = &self.icon_png
            && !p.is_file()
        {
            return Err(anyhow!("icon png not found: {}", p.display()));
        }
        if let Some(rc) = &self.version_info_rc
            && !rc.is_file()
        {
            return Err(anyhow!("version_info file not found: {}", rc.display()));
        }
        if self.template_exe == self.output_exe {
            return Err(anyhow!("template_exe and output_exe must differ"));
        }
        if self.output_exe.file_name().is_none() {
            return Err(anyhow!("output_exe path must include a filename"));
        }
        Ok(())
    }
}

/// 备好料后的待写数据：跨平台准备，Windows 上才真正写入 exe。
#[allow(dead_code)] // 非 Windows 下只构造不消费（写盘 stub 在 windows_impl 里）
#[derive(Debug, Default)]
pub(crate) struct PreparedData {
    /// `(资源名, 字节)`，按写入顺序排列。
    pub rcdata: Vec<(String, Vec<u8>)>,
    /// `(ico字节, 组id)`。
    pub icon: Option<(Vec<u8>, u16)>,
    /// 编译好的 `VS_VERSIONINFO` 资源数据。
    pub version_info: Option<Vec<u8>>,
}

/// 备料：读文件 + 打资源包 + PNG 转 ICO。全跨平台，Windows 实现只负责写盘。
fn prepare(req: &PackRequest) -> Result<PreparedData> {
    let mut rcdata: Vec<(String, Vec<u8>)> = Vec::new();
    if let (Some(dir), Some(cfg)) = (&req.resource_dir, &req.config_file) {
        let pkg = bundle::bundle_resource_package(dir, cfg)?;
        rcdata.push((bundle::INDEXES_NAME.to_string(), pkg.indexes_json));
        rcdata.push((bundle::DATA_NAME.to_string(), pkg.data_deflated));
    }
    for e in &req.rcdata {
        rcdata.push((e.name.clone(), std::fs::read(&e.file)?));
    }
    let icon = if let Some(p) = &req.icon_png {
        let ico = icon::png_to_ico_bytes(&std::fs::read(p)?)?;
        icon::split_ico(&ico).context("validate generated ICO")?;
        Some((ico, DEFAULT_ICON_GROUP_ID))
    } else if let Some(ic) = &req.icon {
        let ico = std::fs::read(&ic.ico_file)?;
        icon::split_ico(&ico).context("validate ICO file")?;
        Some((ico, ic.group_id))
    } else {
        None
    };
    let version_info = req
        .version_info_rc
        .as_ref()
        .map(|path| {
            let source = std::fs::read_to_string(path)
                .map_err(|error| anyhow!("read version info {}: {error}", path.display()))?;
            version_info::compile_version_info_rc(&source)
                .map_err(|error| anyhow!("compile version info {}: {error:#}", path.display()))
        })
        .transpose()?;
    Ok(PreparedData {
        rcdata,
        icon,
        version_info,
    })
}

/// 执行一次打包：校验 -> 备料 -> 同目录暂存模板 -> 写资源 -> 原子发布。
///
/// 非 Windows 下：备料和暂存拷贝照常执行（方便 CI smoke test 和 mac 上调通备料），
/// 随后返回明确错误并清理暂存文件，不发布或删除已有输出。
/// Windows 下把 [`PreparedData`] 委托给 `windows_impl::apply` 写入。
pub fn pack(req: &PackRequest) -> Result<()> {
    req.validate()?;
    let prepared = prepare(req)?;
    #[cfg(target_os = "windows")]
    {
        pack_with_apply(req, &prepared, windows_impl::apply)
    }
    #[cfg(not(target_os = "windows"))]
    {
        pack_with_apply(req, &prepared, |_, _| {
            Err(anyhow!(
                "win_packager::pack only supported on Windows (resource-write stage)"
            ))
        })
    }
}

fn pack_with_apply(
    req: &PackRequest,
    prepared: &PreparedData,
    apply: impl FnOnce(&PackRequest, &PreparedData) -> Result<()>,
) -> Result<()> {
    if !req.template_exe.is_file() {
        return Err(anyhow!(
            "template exe not found: {}",
            req.template_exe.display()
        ));
    }
    if paths_refer_to_same_file(&req.template_exe, &req.output_exe)? {
        return Err(anyhow!("template_exe and output_exe must differ"));
    }

    let parent = req
        .output_exe
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create output directory {}", parent.display()))?;

    let staged = StagedOutput::create(parent)?;
    std::fs::copy(&req.template_exe, staged.path()).with_context(|| {
        format!(
            "copy template {} to staging file {}",
            req.template_exe.display(),
            staged.path().display()
        )
    })?;
    let mut staged_request = req.clone();
    staged_request.output_exe = staged.path().to_path_buf();
    apply(&staged_request, prepared)?;
    staged.publish(&req.output_exe)
}

static NEXT_STAGED_OUTPUT: AtomicU64 = AtomicU64::new(0);

struct StagedOutput {
    path: PathBuf,
    published: bool,
}

impl StagedOutput {
    fn create(parent: &Path) -> Result<Self> {
        for _ in 0..64 {
            let sequence = NEXT_STAGED_OUTPUT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".niva-packager-{}-{sequence}.tmp",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    drop(file);
                    return Ok(Self {
                        path,
                        published: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("create staging output {}", path.display()));
                }
            }
        }
        bail!("could not allocate a unique staged output file")
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn publish(mut self, output: &Path) -> Result<()> {
        std::fs::rename(&self.path, output).with_context(|| {
            format!(
                "publish staged output {} to {}",
                self.path.display(),
                output.display()
            )
        })?;
        self.published = true;
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.published {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> PackRequest {
        PackRequest {
            template_exe: PathBuf::from("a.exe"),
            output_exe: PathBuf::from("b.exe"),
            rcdata: vec![RcDataEntry {
                name: "R".into(),
                file: PathBuf::from("file"),
            }],
            lang: DEFAULT_LANG,
            ..Default::default()
        }
    }

    #[test]
    fn empty_request_rejected() {
        let mut r = req();
        r.rcdata.clear();
        assert!(r.validate().is_err());
    }

    #[test]
    fn same_exe_rejected_before_copy() {
        let mut r = req();
        // 用真实临时文件让校验走到「相同路径」分支，而不是「文件不存在」分支
        let dir = std::env::temp_dir();
        let f = dir.join("win_packager_same_exe_probe.tmp");
        std::fs::write(&f, b"x").unwrap();
        r.template_exe = f.clone();
        r.output_exe = f.clone();
        // rcdata file 也指向真实文件，避免先报 rcdata 缺失
        r.rcdata[0].file = f.clone();
        let err = r.validate().unwrap_err().to_string();
        std::fs::remove_file(&f).ok();
        assert!(err.contains("must differ"), "{err}");
    }

    #[test]
    fn icon_and_icon_png_exclusive() {
        let mut r = req();
        r.icon = Some(IconEntry {
            ico_file: PathBuf::from("x.ico"),
            group_id: 1,
        });
        r.icon_png = Some(PathBuf::from("x.png"));
        assert!(r.validate().is_err());
    }

    #[test]
    fn windows_path_validation_rejects_embedded_nul() {
        let path = PathBuf::from(std::ffi::OsString::from("runtime\0:stream.exe"));
        let error = validate_windows_path_no_nul(&path).unwrap_err();
        assert!(error.to_string().contains("NUL"));
    }

    #[test]
    fn rejects_hard_link_aliases_between_template_and_output() {
        let base = test_dir("hard-link-alias");
        std::fs::create_dir_all(&base).unwrap();
        let input = base.join("template.exe");
        let output = base.join("output.exe");
        std::fs::write(&input, b"template").unwrap();
        std::fs::hard_link(&input, &output).unwrap();

        assert!(paths_refer_to_same_file(&input, &output).unwrap());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn missing_parent_and_parent_component_still_detect_runtime_alias() {
        let base = test_dir("missing-parent-alias");
        std::fs::create_dir_all(&base).unwrap();
        let input = base.join("template.exe");
        let output = base.join("missing").join("..").join("template.exe");
        std::fs::write(&input, b"template").unwrap();

        assert!(!output.parent().unwrap().exists());
        assert!(paths_refer_to_same_file(&input, &output).unwrap());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn path_alias_check_preserves_symlink_parent_semantics() {
        use std::os::unix::fs::symlink;

        let base = test_dir("symlink-parent-alias");
        std::fs::create_dir_all(base.join("real/subdir")).unwrap();
        let outside = base.join("template.exe");
        let inside = base.join("real/template.exe");
        let link = base.join("link");
        std::fs::write(&outside, b"outside").unwrap();
        std::fs::write(&inside, b"inside").unwrap();
        symlink(base.join("real/subdir"), &link).unwrap();
        let output = link.join("..").join("template.exe");

        assert!(!paths_refer_to_same_file(&outside, &output).unwrap());
        assert!(paths_refer_to_same_file(&inside, &output).unwrap());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_alias_check_ignores_ascii_case() {
        assert!(paths_have_same_name(
            Path::new(r"C:\Niva\runtime.exe"),
            Path::new(r"c:\niva\RUNTIME.EXE")
        ));
    }

    #[test]
    fn failed_staged_apply_preserves_existing_output_and_cleans_temporary_file() {
        let base = test_dir("staged-failure");
        std::fs::create_dir_all(&base).unwrap();
        let template = base.join("template.exe");
        let output = base.join("output.exe");
        std::fs::write(&template, b"template bytes").unwrap();
        std::fs::write(&output, b"previous output").unwrap();
        let request = PackRequest {
            template_exe: template,
            output_exe: output.clone(),
            ..Default::default()
        };

        let result = pack_with_apply(&request, &PreparedData::default(), |staged, _| {
            assert_ne!(staged.output_exe, request.output_exe);
            std::fs::write(&staged.output_exe, b"partial resource update").unwrap();
            Err(anyhow!("simulated PE update failure"))
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"previous output");
        assert_no_staged_output(&base);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn successful_staged_apply_atomically_replaces_existing_output() {
        let base = test_dir("staged-success");
        std::fs::create_dir_all(&base).unwrap();
        let template = base.join("template.exe");
        let output = base.join("output.exe");
        std::fs::write(&template, b"template bytes").unwrap();
        std::fs::write(&output, b"previous output").unwrap();
        let request = PackRequest {
            template_exe: template,
            output_exe: output.clone(),
            ..Default::default()
        };

        pack_with_apply(&request, &PreparedData::default(), |staged, _| {
            assert_eq!(
                std::fs::read(&staged.output_exe).unwrap(),
                b"template bytes"
            );
            std::fs::write(&staged.output_exe, b"complete packaged output")?;
            Ok(())
        })
        .unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"complete packaged output");
        assert_no_staged_output(&base);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn template_output_alias_is_rejected_before_apply() {
        let base = test_dir("staged-alias");
        std::fs::create_dir_all(&base).unwrap();
        let template = base.join("template.exe");
        let output = base.join("output.exe");
        std::fs::write(&template, b"template bytes").unwrap();
        std::fs::hard_link(&template, &output).unwrap();
        let request = PackRequest {
            template_exe: template.clone(),
            output_exe: output.clone(),
            ..Default::default()
        };

        let result = pack_with_apply(&request, &PreparedData::default(), |_, _| {
            panic!("resource update must not start when output aliases template")
        });

        assert!(result.unwrap_err().to_string().contains("must differ"));
        assert_eq!(std::fs::read(&template).unwrap(), b"template bytes");
        assert_eq!(std::fs::read(&output).unwrap(), b"template bytes");
        assert_no_staged_output(&base);
        std::fs::remove_dir_all(base).unwrap();
    }

    fn assert_no_staged_output(parent: &Path) {
        assert!(std::fs::read_dir(parent).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".niva-packager-")
        }));
    }

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("win_packager-{name}-{}", std::process::id()))
    }

    #[test]
    fn default_language_matches_resource_manager_locale() {
        assert_eq!(PackRequest::default().lang, DEFAULT_LANG);
    }

    #[test]
    fn bundle_round_trip() {
        use std::io::Read;
        let base = std::env::temp_dir().join(format!("win_packager_bundle_{}", std::process::id()));
        let res = base.join("res");
        std::fs::create_dir_all(res.join("sub")).unwrap();
        let cfg = br#"{"name":"t"}"#;
        std::fs::write(base.join("niva.json"), cfg).unwrap();
        std::fs::write(res.join("niva.json"), br#"{"name":"wrong"}"#).unwrap();
        std::fs::write(res.join("a.txt"), b"hello").unwrap();
        std::fs::write(res.join("sub").join("b.bin"), [0u8, 1, 2, 3]).unwrap();

        let pkg = bundle::bundle_resource_package(&res, &base.join("niva.json")).unwrap();
        let idx: std::collections::BTreeMap<String, (usize, usize)> =
            serde_json::from_slice(&pkg.indexes_json).unwrap();
        assert_eq!(idx["niva.json"], (0, cfg.len()));
        // key 的 `\` 已统一为 `/`
        assert!(idx.contains_key("a.txt"));
        assert!(idx.contains_key("sub/b.bin"));

        let mut dec = flate2::read::DeflateDecoder::new(&pkg.data_deflated[..]);
        let mut raw = Vec::new();
        dec.read_to_end(&mut raw).unwrap();
        assert_eq!(&raw[..cfg.len()], cfg);
        let (off, len) = idx["sub/b.bin"];
        assert_eq!(&raw[off..off + len], &[0u8, 1, 2, 3]);
        let (off, len) = idx["a.txt"];
        assert_eq!(&raw[off..off + len], b"hello");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn png_to_ico_header() {
        use image::ImageEncoder;
        let mut rgba = image::RgbaImage::new(64, 64);
        for p in rgba.pixels_mut() {
            *p = image::Rgba([200, 30, 30, 255]);
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgba, 64, 64, image::ExtendedColorType::Rgba8)
            .unwrap();
        let ico = icon::png_to_ico_bytes(&png).unwrap();
        // ICONDIR: reserved(0) + type(1) + count(7)
        assert_eq!(&ico[0..4], &[0, 0, 1, 0]);
        assert_eq!(
            u16::from_le_bytes([ico[4], ico[5]]) as usize,
            icon::ICON_SIZES.len()
        );
        // Every entry: dims + planes/bitcount + sane offset/size,
        // payload starts with PNG magic and decodes to RGBA dims.
        for (i, size) in icon::ICON_SIZES.iter().enumerate() {
            let e = 6 + i * 16;
            let dim = if *size >= 256 { 0 } else { *size as u8 };
            assert_eq!(ico[e], dim, "width entry {i}");
            assert_eq!(ico[e + 1], dim, "height entry {i}");
            assert_eq!(&ico[e + 4..e + 6], &[1, 0], "planes entry {i}");
            assert_eq!(&ico[e + 6..e + 8], &[32, 0], "bitcount entry {i}");
            let len = u32::from_le_bytes(ico[e + 8..e + 12].try_into().unwrap()) as usize;
            let off = u32::from_le_bytes(ico[e + 12..e + 16].try_into().unwrap()) as usize;
            assert!(len > 8 && off + len <= ico.len(), "entry {i} bounds");
            assert_eq!(
                &ico[off..off + 8],
                &[137, 80, 78, 71, 13, 10, 26, 10],
                "PNG magic entry {i}"
            );
            let decoded = image::load_from_memory(&ico[off..off + len]).unwrap();
            assert_eq!(decoded.width(), *size);
            assert_eq!(decoded.height(), *size);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn replaces_icons_when_template_ids_are_missing_or_present() {
        use image::ImageEncoder;

        let base = std::env::temp_dir().join(format!("win_packager_icons_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let png_path = base.join("icon.png");
        let mut rgba = image::RgbaImage::new(64, 64);
        for pixel in rgba.pixels_mut() {
            *pixel = image::Rgba([20, 80, 180, 255]);
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgba, 64, 64, image::ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(&png_path, png).unwrap();

        let first = base.join("first.exe");
        let second = base.join("second.exe");
        let mut request = PackRequest {
            template_exe: std::env::current_exe().unwrap(),
            output_exe: first.clone(),
            icon_png: Some(png_path),
            delete_icon_ids: (1..=7).collect(),
            ..Default::default()
        };
        pack(&request).unwrap();
        request.template_exe = first;
        request.output_exe = second.clone();
        request.delete_icon_ids = (8..=14).collect();
        pack(&request).unwrap();
        assert!(second.metadata().unwrap().len() > 0);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn failed_resource_update_removes_incomplete_output() {
        let base =
            std::env::temp_dir().join(format!("win_packager_failure_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let template = base.join("invalid.exe");
        let output = base.join("output.exe");
        std::fs::write(&template, b"not a PE executable").unwrap();
        let request = PackRequest {
            template_exe: template.clone(),
            output_exe: output.clone(),
            rcdata: vec![RcDataEntry {
                name: "TEST".into(),
                file: template,
            }],
            ..Default::default()
        };
        assert!(pack(&request).is_err());
        assert!(!output.exists());
        std::fs::remove_dir_all(base).unwrap();
    }
}
