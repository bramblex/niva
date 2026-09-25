use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};
use zip::{ZipWriter, write::SimpleFileOptions};

pub fn zip_app(app: &Path, output: &Path, store_prefix: Option<&str>) -> Result<()> {
    let mut zip = ZipWriter::new(fs::File::create(output)?);
    let parent = app.parent().context("app needs a parent directory")?;
    append(&mut zip, parent, app, store_prefix)?;
    zip.finish()?.sync_all()?;
    Ok(())
}

/// Create a portable directory ZIP whose entries start at `root` (no enclosing
/// directory). Resource prefixes can be stored without recompression so large
/// already-compressed business media stays uncompressed in green packages.
pub fn zip_directory(root: &Path, output: &Path, store_prefix: &str) -> Result<()> {
    let mut zip = ZipWriter::new(fs::File::create(output)?);
    let mut entries = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for entry in entries {
        append(&mut zip, root, &entry, Some(store_prefix))?;
    }
    zip.finish()?.sync_all()?;
    Ok(())
}

fn append(
    zip: &mut ZipWriter<fs::File>,
    root: &Path,
    path: &Path,
    store_prefix: Option<&str>,
) -> Result<()> {
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
            append(zip, root, &entry, store_prefix)?;
        }
    } else {
        ensure!(metadata.is_file(), "only regular files can be archived");
        // Explicit target metadata: Windows hosts cannot query Unix execute bits.
        let mode = if name.contains("/Contents/MacOS/")
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.to_ascii_lowercase().ends_with(".exe"))
        {
            0o755
        } else {
            0o644
        };
        let store = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().ends_with(".exe"))
            || store_prefix.is_some_and(|prefix| name.starts_with(prefix));
        let compression = if store {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        zip.start_file(
            name,
            SimpleFileOptions::default()
                .unix_permissions(mode)
                .compression_method(compression),
        )?;
        let mut file = fs::File::open(path)?;
        std::io::copy(&mut file, zip)?;
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
        zip_app(&app, &output, None).unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
        let file = archive.by_name("Demo.app/Contents/MacOS/Demo").unwrap();
        assert_eq!(file.unix_mode().unwrap() & 0o777, 0o755);
    }

    #[test]
    fn green_archives_store_runtime_and_external_business_resources() {
        let tmp = tempfile::tempdir().unwrap();
        let package = tmp.path().join("package");
        fs::create_dir_all(package.join("resources")).unwrap();
        fs::write(package.join("Demo.exe"), b"runtime").unwrap();
        fs::write(
            package.join("resources/video.mp4"),
            b"already compressed media",
        )
        .unwrap();
        let output = tmp.path().join("green.zip");
        zip_directory(&package, &output, "resources/").unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
        let executable = archive.by_name("Demo.exe").unwrap();
        assert_eq!(executable.compression(), zip::CompressionMethod::Stored);
        assert_eq!(executable.unix_mode().unwrap() & 0o777, 0o755);
        drop(executable);
        let media = archive.by_name("resources/video.mp4").unwrap();
        assert_eq!(media.compression(), zip::CompressionMethod::Stored);
    }

    #[test]
    fn external_mac_app_resources_are_stored_in_the_outer_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("Demo.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::create_dir_all(app.join("Contents/Resources/app")).unwrap();
        fs::write(app.join("Contents/MacOS/Demo"), b"runtime").unwrap();
        fs::write(app.join("Contents/Resources/app/movie.mov"), b"movie data").unwrap();
        let output = tmp.path().join("app.zip");
        zip_app(&app, &output, Some("Demo.app/Contents/Resources/app/")).unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
        let movie = archive
            .by_name("Demo.app/Contents/Resources/app/movie.mov")
            .unwrap();
        assert_eq!(movie.compression(), zip::CompressionMethod::Stored);
    }
}
