use anyhow::Result;

use crate::app::NivaApp;
use crate::app::api_manager::ApiRequest;
use crate::app::api_manager::{ApiManager, CallContext};
use crate::app::resource_manager::ResourceManager;
use crate::app::window_manager::window::NivaWindow;
use std::path::Path;
use std::sync::Arc;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("resource.exists", exists);
    api_manager.register_blocking_api("resource.extract", extract);
    api_manager.register_stream_api("resource.readStream", read_stream);
}

fn exists(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<bool> {
    let (path,) = request.args().get::<(String,)>()?;
    Ok(resource_exists(app.resource().as_ref(), &path))
}

fn extract(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (from, to) = request.args().get::<(String, String)>()?;
    extract_resource(app.resource().as_ref(), &from, Path::new(&to))
}

fn resource_exists(resources: &dyn ResourceManager, path: &str) -> bool {
    resources.exists(path)
}

fn extract_resource(resources: &dyn ResourceManager, from: &str, to: &Path) -> Result<()> {
    anyhow::ensure!(!to.is_dir(), "resource extraction target must be a file");
    let existing_permissions = std::fs::metadata(to)
        .ok()
        .map(|metadata| metadata.permissions());
    let parent = to
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    anyhow::ensure!(
        parent.is_dir(),
        "resource destination directory does not exist"
    );
    if to.exists() {
        // Replacing atomically must not bypass the destination's ordinary
        // write permission merely because its parent directory is writable.
        std::fs::OpenOptions::new().write(true).open(to)?;
    }
    let mut random = [0u8; 16];
    getrandom::fill(&mut random)
        .map_err(|error| anyhow::anyhow!("temporary resource path: {error}"))?;
    let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let temporary = parent.join(format!(".niva-extract-{suffix}"));
    // Both backends stream into a new sibling file. Only a complete copy may
    // replace the requested destination, preserving the public overwrite
    // behavior without materializing a large resource in memory.
    let result = (|| -> Result<()> {
        resources.extract(from, &temporary)?;
        if let Some(permissions) = existing_permissions {
            std::fs::set_permissions(&temporary, permissions)?;
        }
        std::fs::rename(&temporary, to)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Streaming embedded-resource read: binary chunks + terminal `{size}`.
async fn read_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use serde_json::json;

    let (path,): (String,) = request.args().get()?;
    let data = ctx.app.resource().load(&path)?;
    let Some(size) = stream_resource_bytes(
        &data,
        || ctx.is_cancelled(),
        |chunk, end| ctx.chunk(chunk, end),
    ) else {
        return Ok(());
    };
    ctx.respond(Ok(json!({ "size": size })));
    Ok(())
}

/// Emits an embedded resource as binary chunks and reports its byte length.
/// `None` means streaming was cancelled before completion.
fn stream_resource_bytes(
    data: &[u8],
    mut is_cancelled: impl FnMut() -> bool,
    mut emit_chunk: impl FnMut(&[u8], bool),
) -> Option<usize> {
    const CHUNK: usize = 65536;
    if data.is_empty() {
        if is_cancelled() {
            return None;
        }
        emit_chunk(&[], true);
        return Some(0);
    }

    let mut offset = 0;
    while offset < data.len() {
        if is_cancelled() {
            return None;
        }
        let end = (offset + CHUNK).min(data.len());
        emit_chunk(&data[offset..end], end == data.len());
        offset = end;
    }
    Some(data.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::resource_manager::FileSystemResource;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(std::path::PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("niva-resource-api-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn exists_checks_a_resource_path_and_rejects_missing_or_escaping_files() {
        let temp = TestDir::new();
        let root = temp.path().join("resources");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("nested/guide.txt"), b"guide").unwrap();
        std::fs::write(temp.path().join("outside.txt"), b"outside").unwrap();
        let resources = FileSystemResource::new(&root).unwrap();

        assert!(resource_exists(resources.as_ref(), "nested/guide.txt"));
        assert!(!resource_exists(resources.as_ref(), "missing.txt"));
        assert!(!resource_exists(resources.as_ref(), "../outside.txt"));
    }

    #[test]
    fn extract_writes_resource_bytes_and_preserves_destination_on_missing_source() {
        let temp = TestDir::new();
        let root = temp.path().join("resources");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("nested/guide.bin"), [0, 255, 1, 128]).unwrap();
        let resources = FileSystemResource::new(&root).unwrap();
        let destination = temp.path().join("extracted.bin");

        extract_resource(resources.as_ref(), "nested/guide.bin", &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), [0, 255, 1, 128]);

        std::fs::write(&destination, b"old destination").unwrap();
        extract_resource(resources.as_ref(), "nested/guide.bin", &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), [0, 255, 1, 128]);

        std::fs::write(&destination, b"keep me").unwrap();
        assert!(extract_resource(resources.as_ref(), "missing.bin", &destination).is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"keep me");
    }

    #[test]
    fn read_stream_emits_exact_binary_chunks_and_terminal_size() {
        let data: Vec<u8> = (0..65543).map(|n| (n % 251) as u8).collect();
        let mut chunks = Vec::new();

        let size = stream_resource_bytes(
            &data,
            || false,
            |chunk, end| chunks.push((chunk.to_vec(), end)),
        );

        assert_eq!(size, Some(data.len()));
        assert_eq!(
            chunks
                .iter()
                .map(|(chunk, _)| chunk.len())
                .collect::<Vec<_>>(),
            [65536, 7]
        );
        assert_eq!(
            chunks.iter().map(|(_, end)| *end).collect::<Vec<_>>(),
            [false, true]
        );
        assert_eq!(
            chunks
                .iter()
                .flat_map(|(chunk, _)| chunk.iter().copied())
                .collect::<Vec<_>>(),
            data
        );
    }

    #[test]
    fn read_stream_sends_an_empty_terminal_chunk_and_stops_on_cancellation() {
        let mut empty_chunks = Vec::new();
        assert_eq!(
            stream_resource_bytes(
                &[],
                || false,
                |chunk, end| { empty_chunks.push((chunk.to_vec(), end)) }
            ),
            Some(0)
        );
        assert_eq!(empty_chunks, [(Vec::new(), true)]);

        let data = [1, 2, 3];
        let mut chunks = Vec::new();
        assert_eq!(
            stream_resource_bytes(
                &data,
                || true,
                |chunk, end| { chunks.push((chunk.to_vec(), end)) }
            ),
            None
        );
        assert!(chunks.is_empty());
    }
}
