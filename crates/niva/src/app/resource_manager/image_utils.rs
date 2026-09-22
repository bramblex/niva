use std::io::Cursor;

use anyhow::{Result, anyhow};
use tao::window::Icon;

pub fn png_to_rgba(png_data: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    use png::ColorType;

    let mut cursor = Cursor::new(png_data);

    let mut decoder = png::Decoder::new(&mut cursor);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
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
    Ok(muda::Icon::from_rgba(rgba, width, height)?)
}

pub fn png_to_tray_icon(png_data: &[u8]) -> Result<tray_icon::Icon> {
    let (rgba, width, height) = png_to_rgba(png_data)?;
    Ok(tray_icon::Icon::from_rgba(rgba, width, height)?)
}
