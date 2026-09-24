use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use zip::{ZipWriter, write::SimpleFileOptions};

pub fn zip_app(app: &Path, output: &Path) -> Result<()> {
    let mut zip = ZipWriter::new(fs::File::create(output)?);
    let parent = app.parent().context("app needs a parent directory")?;
    append(&mut zip, parent, app)?;
    zip.finish()?.sync_all()?;
    Ok(())
}
fn append(zip: &mut ZipWriter<fs::File>, root: &Path, path: &Path) -> Result<()> {
    let relative = path.strip_prefix(root)?;
    let name = relative
        .components()
        .map(|c| c.as_os_str().to_str().context("archive path must be UTF-8"))
        .collect::<Result<Vec<_>>>()?
        .join("/");
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        let target = target.to_str().context("symlink target must be UTF-8")?;
        zip.add_symlink(
            name,
            target,
            SimpleFileOptions::default().unix_permissions(0o777),
        )?;
    } else if metadata.is_dir() {
        zip.add_directory(
            format!("{name}/"),
            SimpleFileOptions::default().unix_permissions(0o755),
        )?;
        let mut entries = fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        for entry in entries {
            append(zip, root, &entry)?;
        }
    } else {
        ensure!(metadata.is_file(), "only regular files can be archived");
        // Explicit target metadata: Windows hosts cannot query Unix execute bits.
        let mode = if name.contains("/Contents/MacOS/") {
            0o755
        } else {
            0o644
        };
        zip.start_file(
            name,
            SimpleFileOptions::default()
                .unix_permissions(mode)
                .compression_method(zip::CompressionMethod::Deflated),
        )?;
        let mut bytes = Vec::new();
        fs::File::open(path)?.read_to_end(&mut bytes)?;
        zip.write_all(&bytes)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_preserves_executable_metadata_on_every_host() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("Demo.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/Demo"), b"runtime").unwrap();
        let output = tmp.path().join("app.zip");
        zip_app(&app, &output).unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
        let file = archive.by_name("Demo.app/Contents/MacOS/Demo").unwrap();
        assert_eq!(file.unix_mode().unwrap() & 0o777, 0o755);
    }
}
