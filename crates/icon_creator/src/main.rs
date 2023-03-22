use anyhow::Result;
use image::imageops::FilterType;
/*
  TODO: Resource Hacker 替换图标配置
  -delete ICON,1,1033
  -delete ICON,2,1033
  -delete ICON,3,1033
  -addoverwrite icon.ico, ICONGROUP,1,1033
*/

use std::path::Path;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    let source = Path::new(&args[1]);
    let target = Path::new(&args[2]);
    let target_bitmap = Path::new(&args[3]);

    let img = image::open(source)?;
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in &[16, 24, 32, 48, 64, 128, 256] {
        let img = img.resize_exact(*size, *size, FilterType::Lanczos3);
        let rgba = img
            .to_rgba8()
            .to_vec();
        let icon_img = ico::IconImage::from_rgba_data(*size, *size, rgba);
        let icon = ico::IconDirEntry::encode(&icon_img)?;
        icon_dir.add_entry(icon);
    }

    let bitmap_img = img.resize_exact(32, 32, FilterType::Lanczos3).to_rgba8().to_vec();
    std::fs::write(target_bitmap, bitmap_img)?;

    let target = std::fs::File::create(target)?;
    icon_dir.write(target)?;
    Ok(())
}
