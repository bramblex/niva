//! Windows exe 后注入打包器：替代 `ResourceHacker.exe` + `icon_creator.exe`。
//!
//! 背景见 `crates/win_packager/README.md`，调用对照见
//! `packages/devtools/src/build-scripts/build-windows.ts`。
//!
//! 职责（一期全量）：
//! - [`bundle`]: 资源目录 + `niva.json` -> `RESOURCE_INDEXES` / `RESOURCE_DATA`
//!   两段字节（复刻前端 `build-scripts/base.ts`，压缩在 crate 内做）。
//! - [`icon`]: PNG -> 多尺寸 ICO 内存转换（逻辑从 `icon_creator` 搬入）。
//! - [`pack`]: 拷贝模板 exe -> Windows 上写 `RCDATA` + 图标 + 版本信息。
//!
//! 平台说明：备料（打包/转图标）跨平台可跑，真正写 PE 资源只能在 Windows 上。
//! 非 Windows 下 [`pack`] 走完备料和拷贝后返回明确错误，见函数文档。

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

pub mod bundle;
pub mod icon;
pub mod version_info;

#[cfg(target_os = "windows")]
mod windows_impl;

/// 默认语言 ID：与现有 ResourceHacker 脚本保持一致（英语-美国）。
pub const DEFAULT_LANG: u16 = 1033;
/// 默认图标组 ID：与现有脚本的 `ICONGROUP,1` 保持一致。
pub const DEFAULT_ICON_GROUP_ID: u16 = 1;

/// 一条 `RCDATA` 资源（低层模式）：`name` 是 exe 内的资源名
/// （如 `RESOURCE_INDEXES`），`file` 是磁盘上已备好的资源文件。
#[derive(Debug, Clone)]
pub struct RcDataEntry {
    pub name: String,
    pub file: PathBuf,
}

/// 图标替换项（低层模式）：`ico_file` 是现成的多尺寸 `.ico`
///（旧链路 `icon_creator` 的产物）。
#[derive(Debug, Clone)]
pub struct IconEntry {
    pub ico_file: PathBuf,
    pub group_id: u16,
}

/// 一次打包请求。
///
/// 推荐高层模式：`resource_dir` + `config_file` + 可选 `icon_png`，
/// 备料全在 crate 内完成，devtools 只需调一次，不再需要
/// `icon_creator.exe` 落盘和前端 `pako` 压缩。
/// 低层模式（`rcdata` / `icon`）保留，用于调试和兼容旧备料。
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

/// 执行一次打包：校验 -> 备料 -> 拷贝模板 -> 写资源。
///
/// 非 Windows 下：备料和拷贝照常执行（方便 CI smoke test 和 mac 上调通备料），
/// 写资源阶段返回 `only supported on Windows` 错误。
/// Windows 下把 [`PreparedData`] 委托给 `windows_impl::apply` 写入。
pub fn pack(req: &PackRequest) -> Result<()> {
    req.validate()?;
    let prepared = prepare(req)?;
    if !req.template_exe.is_file() {
        return Err(anyhow!(
            "template exe not found: {}",
            req.template_exe.display()
        ));
    }
    if let Some(parent) = req.output_exe.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&req.template_exe, &req.output_exe)?;

    #[cfg(target_os = "windows")]
    {
        windows_impl::apply(req, &prepared)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (req, prepared);
        Err(anyhow!(
            "win_packager::pack only supported on Windows (resource-write stage)"
        ))
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
}
