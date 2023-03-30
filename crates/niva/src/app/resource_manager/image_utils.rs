use std::io::Cursor;

use anyhow::{anyhow, Result};
use tao::window::Icon;

pub fn png_to_icon(png_data: &[u8]) -> Result<tao::window::Icon> {
    use png::ColorType;

    let mut cursor = Cursor::new(png_data);

    let mut decoder = png::Decoder::new(&mut cursor);
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
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

    let width = info.width;
    let height = info.height;

    Ok(Icon::from_rgba(rgba, width, height)?)
}

// pub fn jpg_to_icon(jpg_data: &[u8]) -> Result<tao::window::Icon> {
//     use jpeg_decoder::{Decoder, PixelFormat};
//     let mut cursor = Cursor::new(jpg_data);

//     let mut decoder = Decoder::new(&mut cursor);
//     let pixels: Vec<u8> = decoder.decode()?;
//     let info = decoder.info().ok_or(anyhow!("No info"))?;

//     let width = info.width as u32;
//     let height = info.height as u32;

//      let rgba: Vec<u8> = match info.pixel_format {
//         PixelFormat::L8 => {
//             // 灰度图像，将 L8 转换为 RGBA 格式
//             pixels
//                 .iter()
//                 .flat_map(|&l| vec![l, l, l, 255])
//                 .collect()
//         },
//         PixelFormat::L16 => {
//             // 灰度图像，将 L16 转换为 RGBA 格式
//             pixels
//                 .iter()
//                 .flat_map(|&l| {
//                     #[allow(arithmetic_overflow)]
//                     let r = l >> 8;
//                     let g = l;
//                     vec![r, g, 0, 255]
//                 })
//                 .collect()
//         },
//         PixelFormat::RGB24 => {
//             // RGB24 格式，将其转换为 RGBA 格式
//             pixels
//                 .chunks(3)
//                 .flat_map(|chunk| vec![chunk[0], chunk[1], chunk[2], 255])
//                 .collect()
//         },
//         PixelFormat::CMYK32 => {
//             // CMYK32 格式，将其转换为 RGBA 格式
//             pixels
//                 .chunks(4)
//                 .flat_map(|chunk| {
//                     let (c, m, y, k) = (chunk[0], chunk[1], chunk[2], chunk[3]);
//                     let r = (255 - c) * (255 - k) / 255;
//                     let g = (255 - m) * (255 - k) / 255;
//                     let b = (255 - y) * (255 - k) / 255;
//                     vec![r, g, b, 255]
//                 })
//                 .collect()
//         },
//     };

//     Ok(Icon::from_rgba(rgba, width, height)?)
// }
