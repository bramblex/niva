use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

/// Resolved copy options (bools filled in by callers).
/// Mirrors `fs_extra` semantics so existing `CopyOptions` API payloads
/// behave identically after the `fs_extra` removal.
#[derive(Clone, Debug, Default)]
pub struct CopyOptions {
    pub overwrite: bool,
    pub skip_exist: bool,
    pub copy_inside: bool,
    pub content_only: bool,
    pub depth: u64,
}

fn decide_dest_exists(dest: &Path, options: &CopyOptions, what: &str) -> Result<bool> {
    if dest.exists() {
        if options.skip_exist {
            return Ok(true); // skip: caller treats as success with 0 bytes
        }
        if !options.overwrite {
            return Err(anyhow!(
                "Destination \"{}\" already exists ({what})",
                dest.display()
            ));
        }
    }
    Ok(false)
}

fn copy_permissions(from: &Path, to: &Path) -> Result<()> {
    let perm = std::fs::metadata(from)?.permissions();
    std::fs::set_permissions(to, perm)?;
    Ok(())
}

/// Copy a single file. If `to` is an existing directory, the file lands
/// inside it (cp-like). Parent directories are created. Permission bits
/// are carried over. Returns bytes copied (0 when skipped).
pub fn copy_file(from: &Path, to: &Path, options: &CopyOptions) -> Result<u64> {
    if !from.exists() {
        return Err(anyhow!("Path \"{}\" does not exist", from.display()));
    }
    if !from.is_file() {
        return Err(anyhow!("Path \"{}\" is not a file", from.display()));
    }
    let to = if to.exists() && to.is_dir() {
        to.join(from.file_name().ok_or(anyhow!("Invalid file name"))?)
    } else {
        to.to_path_buf()
    };
    if decide_dest_exists(&to, options, "file")? {
        return Ok(0);
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = std::fs::copy(from, &to)?;
    // Best effort: keep going even if permission copy fails (e.g. FAT32).
    let _ = copy_permissions(from, &to);
    Ok(bytes)
}

/// Move a single file. Falls back to copy+remove across devices.
pub fn move_file(from: &Path, to: &Path, options: &CopyOptions) -> Result<u64> {
    if !from.exists() {
        return Err(anyhow!("Path \"{}\" does not exist", from.display()));
    }
    if !from.is_file() {
        return Err(anyhow!("Path \"{}\" is not a file", from.display()));
    }
    let to = if to.exists() && to.is_dir() {
        to.join(from.file_name().ok_or(anyhow!("Invalid file name"))?)
    } else {
        to.to_path_buf()
    };
    if decide_dest_exists(&to, options, "file")? {
        return Ok(0);
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(from, &to) {
        Ok(()) => Ok(std::fs::metadata(&to).map(|m| m.len()).unwrap_or(0)),
        Err(_) => {
            // Cross-device move: copy, then remove the source.
            let bytes = copy_file(from, &to, options)?;
            std::fs::remove_file(from)?;
            Ok(bytes)
        }
    }
}

/// Resolve the effective destination dir, mirroring `fs_extra`:
/// `to/<basename>` unless `content_only`, or (`copy_inside` with a
/// non-existent `to` — then contents land directly in `to`).
fn resolve_dir_dest(from: &Path, to: &Path, options: &CopyOptions) -> Result<PathBuf> {
    let dir_name = from.components().last().ok_or(anyhow!("Invalid folder"))?;
    let mut dest = to.to_path_buf();
    if (to.exists() || !options.copy_inside) && !options.content_only {
        dest.push(dir_name);
    }
    Ok(dest)
}

fn walk_dirs(dir: &Path, max_depth: u64, current: u64, out: &mut Vec<PathBuf>) -> Result<()> {
    // depth 0 = unlimited; otherwise stop descending past max_depth.
    if max_depth != 0 && current > max_depth {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.push(path.clone());
            walk_dirs(&path, max_depth, current + 1, out)?;
        }
    }
    Ok(())
}

fn walk_files(dir: &Path, max_depth: u64, current: u64, out: &mut Vec<PathBuf>) -> Result<()> {
    if max_depth != 0 && current > max_depth {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk_files(&path, max_depth, current + 1, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Copy a directory tree. Returns bytes copied.
pub fn copy_dir(from: &Path, to: &Path, options: &CopyOptions) -> Result<u64> {
    if !from.exists() {
        return Err(anyhow!("Path \"{}\" does not exist", from.display()));
    }
    if !from.is_dir() {
        return Err(anyhow!("Path \"{}\" is not a directory", from.display()));
    }
    let dest = resolve_dir_dest(from, to, options)?;

    let mut dirs = vec![from.to_path_buf()];
    walk_dirs(from, options.depth, 1, &mut dirs)?;
    for dir in &dirs {
        let rel = dir
            .strip_prefix(from)
            .map_err(|_| anyhow!("Invalid path"))?;
        let target = dest.join(rel);
        if !target.exists() {
            std::fs::create_dir_all(&target)?;
        }
    }

    let mut files = Vec::new();
    walk_files(from, options.depth, 1, &mut files)?;
    let mut bytes = 0u64;
    let file_opt = CopyOptions {
        content_only: false,
        copy_inside: true,
        depth: 0,
        ..options.clone()
    };
    for file in &files {
        let rel = file
            .strip_prefix(from)
            .map_err(|_| anyhow!("Invalid path"))?;
        let target = dest.join(rel);
        bytes += copy_file(file, &target, &file_opt)?;
    }
    Ok(bytes)
}

/// Move a directory tree: copy, then remove the source (unless the move was
/// skipped wholesale via `skip_exist` on an existing destination).
pub fn move_dir(from: &Path, to: &Path, options: &CopyOptions) -> Result<u64> {
    if !from.exists() {
        return Err(anyhow!("Path \"{}\" does not exist", from.display()));
    }
    if !from.is_dir() {
        return Err(anyhow!("Path \"{}\" is not a directory", from.display()));
    }
    // Matches fs_extra: a fully-skipped move leaves the source in place.
    let mut remove_source = true;
    if options.skip_exist && to.exists() && !options.overwrite {
        remove_source = false;
    }
    // Fast path: same-device rename preserving the dest-name rule.
    let dest = resolve_dir_dest(from, to, options)?;
    if dest.parent().map(|p| p.exists()).unwrap_or(false) {
        match std::fs::rename(from, &dest) {
            Ok(()) => return Ok(0),
            Err(_) => { /* fall through to copy+remove */ }
        }
    }
    let bytes = copy_dir(from, to, options)?;
    if remove_source {
        std::fs::remove_dir_all(from)?;
    }
    Ok(bytes)
}

/// Remove a file or directory tree.
pub fn remove_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    pub(super) fn tmp() -> PathBuf {
        let id = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("niva-fsops-{id}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    pub(super) fn seed_tree(root: &Path) {
        std::fs::create_dir_all(root.join("src/sub")).unwrap();
        std::fs::write(root.join("src/a.txt"), "hello").unwrap();
        std::fs::write(root.join("src/sub/b.txt"), "world").unwrap();
    }

    pub(super) fn read(p: &Path) -> String {
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn copy_dir_default_nests_basename() {
        let root = tmp();
        seed_tree(&root);
        copy_dir(&root.join("src"), &root.join("d1"), &CopyOptions::default()).unwrap();
        assert_eq!(read(&root.join("d1/src/a.txt")), "hello");
        assert_eq!(read(&root.join("d1/src/sub/b.txt")), "world");
    }

    #[test]
    fn copy_dir_inside_and_content_only() {
        let root = tmp();
        seed_tree(&root);
        let inside = CopyOptions {
            copy_inside: true,
            ..Default::default()
        };
        copy_dir(&root.join("src"), &root.join("d2"), &inside).unwrap();
        assert_eq!(read(&root.join("d2/a.txt")), "hello");

        let content = CopyOptions {
            content_only: true,
            ..Default::default()
        };
        copy_dir(&root.join("src"), &root.join("d3"), &content).unwrap();
        assert_eq!(read(&root.join("d3/sub/b.txt")), "world");
    }

    #[test]
    fn copy_file_overwrite_rules() {
        let root = tmp();
        seed_tree(&root);
        let dst = root.join("out.txt");
        copy_file(&root.join("src/a.txt"), &dst, &CopyOptions::default()).unwrap();
        assert_eq!(read(&dst), "hello");
        // no overwrite -> error
        assert!(copy_file(&root.join("src/a.txt"), &dst, &CopyOptions::default()).is_err());
        // skip_exist -> silent ok
        let skipped = CopyOptions {
            skip_exist: true,
            ..Default::default()
        };
        assert_eq!(
            copy_file(&root.join("src/a.txt"), &dst, &skipped).unwrap(),
            0
        );
        // overwrite -> ok
        let over = CopyOptions {
            overwrite: true,
            ..Default::default()
        };
        copy_file(&root.join("src/sub/b.txt"), &dst, &over).unwrap();
        assert_eq!(read(&dst), "world");
    }

    #[test]
    fn move_file_and_dir() {
        let root = tmp();
        seed_tree(&root);
        move_file(
            &root.join("src/a.txt"),
            &root.join("moved.txt"),
            &CopyOptions::default(),
        )
        .unwrap();
        assert!(!root.join("src/a.txt").exists());
        assert_eq!(read(&root.join("moved.txt")), "hello");

        move_dir(&root.join("src"), &root.join("dm"), &CopyOptions::default()).unwrap();
        assert!(!root.join("src").exists());
        assert_eq!(read(&root.join("dm/src/sub/b.txt")), "world");
    }

    #[test]
    fn remove_file_and_dir() {
        let root = tmp();
        seed_tree(&root);
        remove_path(&root.join("src/sub")).unwrap();
        assert!(!root.join("src/sub").exists());
        assert!(root.join("src/a.txt").exists());
    }
}

#[cfg(test)]
mod e2e_repro_tests {
    use super::tests::{read, seed_tree, tmp};
    use super::*;

    #[test]
    fn move_after_content_only_seed() {
        let root = tmp();
        seed_tree(&root);
        let content = CopyOptions {
            content_only: true,
            ..Default::default()
        };
        copy_dir(&root.join("src"), &root.join("d3"), &content).unwrap();
        assert_eq!(read(&root.join("d3/sub/b.txt")), "world");
        move_dir(&root.join("d3"), &root.join("dm"), &CopyOptions::default()).unwrap();
        assert!(!root.join("d3").exists());
        assert_eq!(read(&root.join("dm/d3/sub/b.txt")), "world");
    }
}
