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
    let mut entries = Vec::with_capacity(ICON_SIZES.len());
    for size in ICON_SIZES {
        let img = img.resize_exact(*size, *size, FilterType::Lanczos3);
        let rgba = img.to_rgba8().into_raw();
        entries.push((*size, encode_png_rgba(*size, &rgba)?));
    }
    Ok(encode_ico(&entries))
}

fn encode_png_rgba(size: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(out)
}

/// Minimal ICO container writer (PNG payloads per entry).
/// Replaces the `ico` crate: same bytes on the wire, one less dependency
/// subtree (`ico` pins `png 0.17`, which drags the unmaintained `adler`).
/// Format: ICONDIR + ICONDIRENTRY[n] + payloads. Width/height byte 0 means
/// 256; planes=1, bitcount=32 for RGBA.
pub fn encode_ico(entries: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let n = entries.len() as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    out.extend_from_slice(&n.to_le_bytes()); // count

    let mut offset = (6 + 16 * entries.len()) as u32;
    for (size, payload) in entries {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        out.push(dim); // width
        out.push(dim); // height
        out.push(0); // color count
        out.push(0); // reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bit count
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += payload.len() as u32;
    }
    for (_, payload) in entries {
        out.extend_from_slice(payload);
    }
    out
}
