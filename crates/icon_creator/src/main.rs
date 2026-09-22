use anyhow::Result;
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

    let png = std::fs::read(source)?;
    let ico = win_packager::icon::png_to_ico_bytes(&png)?;
    std::fs::write(target, ico)?;
    Ok(())
}
