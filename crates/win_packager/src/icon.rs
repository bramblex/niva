//! PNG -> 多尺寸 ICO（内存转换）。
//!
//! 逻辑从 `crates/icon_creator/src/main.rs` 原样搬入：
//! 尺寸 `[16, 24, 32, 48, 64, 128, 256]`，`resize_exact(Lanczos3)`，RGBA8。
//! 区别：原来是 `input.png -> output.ico` 两个文件，这里是
//! `&[u8] -> Vec<u8>]`，省掉 devtools 那次 `icon_creator.exe` 落盘 +
//! 子进程调用（`build-windows.ts` 的 `GENERATING_ICON` 整步可删）。
//!
//! `icon_creator` 在新链路跑通前保留（旧链路仍在用它），跑通后删除，
//! 见 `crates/win_packager/README.md` §6。

use anyhow::Result;
use image::imageops::FilterType;

pub const ICON_SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256];

pub fn png_to_ico_bytes(png: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(png)?;
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in ICON_SIZES {
        let img = img.resize_exact(*size, *size, FilterType::Lanczos3);
        let rgba = img.to_rgba8().into_raw();
        let icon_img = ico::IconImage::from_rgba_data(*size, *size, rgba);
        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_img)?);
    }
    let mut out = Vec::new();
    icon_dir.write(&mut out)?;
    Ok(out)
}
