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

/// A parsed ICO image, ready to store as a Windows `RT_ICON` resource.
#[allow(dead_code)] // The Windows-only writer consumes these fields.
pub(crate) struct IcoImage<'a> {
    /// The first 12 bytes of `ICONDIRENTRY`, before the file offset field.
    pub header: [u8; 12],
    pub data: &'a [u8],
}

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

/// Split an ICO container into its image entries for `RT_ICON` resources.
pub(crate) fn split_ico(ico: &[u8]) -> Result<Vec<IcoImage<'_>>> {
    if ico.len() < 6 {
        anyhow::bail!("ICO header is truncated");
    }
    let reserved = read_u16(ico, 0)?;
    let kind = read_u16(ico, 2)?;
    let count = usize::from(read_u16(ico, 4)?);
    if reserved != 0 || kind != 1 || count == 0 {
        anyhow::bail!("invalid ICO header");
    }
    let table_len = count
        .checked_mul(16)
        .ok_or_else(|| anyhow::anyhow!("ICO entry table length overflow"))?;
    let table_end = 6usize
        .checked_add(table_len)
        .filter(|end| *end <= ico.len())
        .ok_or_else(|| anyhow::anyhow!("ICO entry table is truncated"))?;

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = 6 + index * 16;
        let header = &ico[start..start + 16];
        if header[3] != 0 {
            anyhow::bail!("ICO entry {index} has a nonzero reserved byte");
        }
        let length = usize::try_from(read_u32(ico, start + 8)?)?;
        let offset = usize::try_from(read_u32(ico, start + 12)?)?;
        let end = offset
            .checked_add(length)
            .filter(|end| offset >= table_end && *end <= ico.len())
            .ok_or_else(|| anyhow::anyhow!("ICO entry {index} image is outside the file"))?;
        if length == 0 {
            anyhow::bail!("ICO entry {index} has empty image data");
        }
        entries.push(IcoImage {
            header: header[..12].try_into().unwrap(),
            data: &ico[offset..end],
        });
    }
    Ok(entries)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("ICO field is truncated"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("ICO field is truncated"))?;
    Ok(u32::from_le_bytes(slice.try_into()?))
}

#[cfg(test)]
mod tests {
    use image::ImageEncoder;

    use super::{ICON_SIZES, png_to_ico_bytes, split_ico};

    #[test]
    fn splits_png_ico_entries_for_resource_injection() {
        let mut rgba = image::RgbaImage::new(64, 64);
        for pixel in rgba.pixels_mut() {
            *pixel = image::Rgba([20, 80, 180, 255]);
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgba, 64, 64, image::ExtendedColorType::Rgba8)
            .unwrap();

        let ico = png_to_ico_bytes(&png).unwrap();
        let entries = split_ico(&ico).unwrap();
        assert_eq!(entries.len(), ICON_SIZES.len());
        for (index, (entry, size)) in entries.iter().zip(ICON_SIZES).enumerate() {
            let dim = if *size >= 256 { 0 } else { *size as u8 };
            assert_eq!(&entry.header[..2], &[dim, dim], "entry {index}");
            assert_eq!(&entry.header[4..8], &[1, 0, 32, 0], "entry {index}");
            assert!(entry.data.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        }
    }

    #[test]
    fn rejects_truncated_ico_entries() {
        assert!(split_ico(&[0, 0, 1, 0, 1, 0]).is_err());
    }
}
