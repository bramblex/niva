use std::io::Cursor;

use anyhow::{Result, anyhow};
use tao::window::Icon;

pub fn png_to_rgba(png_data: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    use png::ColorType;

    let mut cursor = Cursor::new(png_data);

    let mut decoder = png::Decoder::new(&mut cursor);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|err| anyhow!("Only PNG icons are supported: {err}"))?;
    let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buffer)?;

    // Convert the pixel data to RGBA format
    let rgba = match info.color_type {
        ColorType::Rgba => buffer,
        ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
            for chunk in buffer.chunks(3) {
                rgba.push(chunk[0]);
                rgba.push(chunk[1]);
                rgba.push(chunk[2]);
                rgba.push(255);
            }
            rgba
        }
        ColorType::Grayscale => {
            let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
            for chunk in buffer.iter() {
                rgba.push(*chunk);
                rgba.push(*chunk);
                rgba.push(*chunk);
                rgba.push(255);
            }
            rgba
        }
        ColorType::GrayscaleAlpha => {
            let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
            for chunk in buffer.chunks(2) {
                rgba.push(chunk[0]);
                rgba.push(chunk[0]);
                rgba.push(chunk[0]);
                rgba.push(chunk[1]);
            }
            rgba
        }
        _ => return Err(anyhow!("Unsupported color type")),
    };

    Ok((rgba, info.width, info.height))
}

pub fn png_to_icon(png_data: &[u8]) -> Result<tao::window::Icon> {
    let (rgba, width, height) = png_to_rgba(png_data)?;
    Ok(Icon::from_rgba(rgba, width, height)?)
}

pub fn png_to_muda_icon(png_data: &[u8]) -> Result<muda::Icon> {
    let (rgba, width, height) = png_to_rgba(png_data)?;
    let (rgba, width, height) = resize_rgba(rgba, width, height, 32)?;
    Ok(muda::Icon::from_rgba(rgba, width, height)?)
}

pub fn png_to_tray_icon(png_data: &[u8]) -> Result<tray_icon::Icon> {
    let (rgba, width, height) = png_to_rgba(png_data)?;
    let (rgba, width, height) = resize_rgba(rgba, width, height, 64)?;
    Ok(tray_icon::Icon::from_rgba(rgba, width, height)?)
}

fn resize_rgba(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    max_dim: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    if width == 0 || height == 0 || max_dim == 0 {
        return Err(anyhow!("Invalid icon dimensions"));
    }
    if width <= max_dim && height <= max_dim {
        return Ok((rgba, width, height));
    }
    let scale = max_dim as f64 / width.max(height) as f64;
    let target_width = ((width as f64 * scale).round() as u32).max(1);
    let target_height = ((height as f64 * scale).round() as u32).max(1);
    let image = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow!("Invalid PNG RGBA buffer"))?;
    let resized = image::imageops::resize(
        &image,
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );
    Ok((resized.into_raw(), target_width, target_height))
}

#[cfg(test)]
mod tests {
    use super::resize_rgba;

    #[test]
    fn resizes_large_icon_without_changing_aspect_ratio() {
        let rgba = vec![255; 1024 * 512 * 4];
        let (resized, width, height) = resize_rgba(rgba, 1024, 512, 64).unwrap();
        assert_eq!((width, height), (64, 32));
        assert_eq!(resized.len(), 64 * 32 * 4);
    }
}
