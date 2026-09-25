use std::{
    fs,
    fs::OpenOptions,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow, bail};
use oxc_resolver::{
    FileMetadata, FileSystem, FileSystemOs, ModuleType, ResolveOptions, ResolverGeneric,
};
use serde_json::{Value, json};

use crate::app::{
    NivaApp,
    api_manager::{ApiManager, ApiRequest},
    resource_manager::ResourceManager,
    window_manager::window::NivaWindow,
};

const MAX_MODULE_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RESOLVER_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MODULE_TEMP_PREFIX: &str = "niva-cjs-resources";

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("module.resolve", resolve);
    api_manager.register_blocking_api("module.load", load);
}

fn resolve(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    let (specifier, parent_filename): (String, String) = request.args().get()?;
    app.api()
        .module_resolver()?
        .resolve(&specifier, &parent_filename)
}

fn load(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let (specifier, parent_filename): (String, String) = request.args().get()?;
    Ok(app
        .api()
        .module_resolver()?
        .load(&specifier, &parent_filename)?)
}

/// Resolves Node CommonJS modules under the selected application resource
/// root. Embedded resources are unpacked once, lazily, to a private temporary
/// tree so `__filename`, `__dirname`, and ordinary fs calls refer to real files.
pub(crate) struct ModuleResolver {
    resources: Arc<dyn ResourceManager>,
    app_uuid: String,
    state: std::sync::Mutex<Option<Arc<ResolverState>>>,
}

struct ResolverState {
    root: PathBuf,
    owned_temp_root: Option<PathBuf>,
    resolver: ResolverGeneric<ModuleFileSystem>,
}

enum ModuleResolution {
    File(Arc<ResolverState>, oxc_resolver::Resolution),
    Builtin(Arc<ResolverState>, String),
}

impl ModuleResolver {
    pub(crate) fn new(resources: Arc<dyn ResourceManager>, app_uuid: String) -> Self {
        Self {
            resources,
            app_uuid,
            state: std::sync::Mutex::new(None),
        }
    }

    pub(crate) fn resolve(&self, specifier: &str, parent_filename: &str) -> Result<String> {
        let (_state, resolution) = match self.resolve_inner(specifier, parent_filename)? {
            ModuleResolution::File(state, resolution) => (state, resolution),
            ModuleResolution::Builtin(_state, resolved) => return Ok(resolved),
        };
        let filename = resolution.path();
        let filename = fs::canonicalize(filename)
            .with_context(|| format!("canonicalize module path {}", filename.display()))?;
        Ok(filename.to_string_lossy().into_owned())
    }

    pub(crate) fn load(&self, specifier: &str, parent_filename: &str) -> Result<Value> {
        let (_state, resolution) = match self.resolve_inner(specifier, parent_filename)? {
            ModuleResolution::File(state, resolution) => (state, resolution),
            ModuleResolution::Builtin(_state, resolved) => {
                bail!("ERR_UNSUPPORTED_BUILTIN_MODULE: {resolved}")
            }
        };
        let filename = resolution.path();
        let module_type = resolution.module_type();
        let module_type = Self::supported_module_type(filename, module_type)?;
        let filename = fs::canonicalize(filename)
            .with_context(|| format!("canonicalize module path {}", filename.display()))?;
        let source_bytes = read_regular_file_bounded(&filename, MAX_MODULE_SOURCE_BYTES)
            .with_context(|| format!("read CommonJS module {}", filename.display()))?;
        let source = String::from_utf8(source_bytes)
            .map_err(|_| anyhow!("ERR_INVALID_MODULE_ENCODING: source is not valid UTF-8"))?;
        let parent = filename.parent().ok_or_else(|| {
            anyhow!("ERR_INVALID_MODULE_SPECIFIER: resolved module has no parent")
        })?;
        let source = normalize_module_source(source, module_type);
        Ok(json!({
            "filename": filename.to_string_lossy(),
            "dirname": parent.to_string_lossy(),
            "type": module_type,
            "source": source,
        }))
    }

    fn resolve_inner(&self, specifier: &str, parent_filename: &str) -> Result<ModuleResolution> {
        anyhow::ensure!(
            !specifier.is_empty(),
            "ERR_INVALID_MODULE_SPECIFIER: empty specifier"
        );
        anyhow::ensure!(
            !specifier.contains('\0'),
            "ERR_INVALID_MODULE_SPECIFIER: NUL byte in specifier"
        );
        let state = self.state()?;
        let resolution = if parent_filename.is_empty() {
            // The initial page has no Node filename. Start at the resource
            // root; the JS loader supplies real absolute filenames after its
            // first module is loaded.
            state.resolver.resolve(&state.root, specifier)
        } else {
            let parent = PathBuf::from(parent_filename);
            anyhow::ensure!(
                parent.is_absolute() && !parent_filename.contains('\0'),
                "ERR_INVALID_MODULE_SPECIFIER: parentFilename must be empty or an absolute filesystem path"
            );
            state
                .resolver
                .resolve_file(Path::new(parent_filename), specifier)
        };
        match resolution {
            Ok(resolution) => {
                let filename = resolution.path();
                anyhow::ensure!(
                    filename.is_absolute(),
                    "ERR_MODULE_RESOLUTION: resolved path is not absolute"
                );
                Ok(ModuleResolution::File(state, resolution))
            }
            Err(oxc_resolver::ResolveError::Builtin { resolved, .. }) => {
                Ok(ModuleResolution::Builtin(state, resolved))
            }
            Err(error) => Err(map_resolve_error(error)),
        }
    }

    fn supported_module_type(
        filename: &Path,
        module_type: Option<ModuleType>,
    ) -> Result<&'static str> {
        match module_type {
            Some(ModuleType::CommonJs) => Ok("commonjs"),
            Some(ModuleType::Json) => Ok("json"),
            Some(ModuleType::Module) => bail!(
                "ERR_REQUIRE_ESM: CommonJS require cannot load ES module {}",
                filename.display()
            ),
            Some(ModuleType::Addon) => bail!(
                "ERR_DLOPEN_DISABLED: native addon loading is unsupported ({})",
                filename.display()
            ),
            Some(ModuleType::Wasm) => bail!(
                "ERR_UNKNOWN_MODULE_FORMAT: WebAssembly modules are unsupported ({})",
                filename.display()
            ),
            // Oxc Resolver leaves the type unset for ordinary .js files
            // outside a package with an explicit `type` field. Node treats
            // those files as CommonJS, including files in legacy packages.
            None if filename
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("js") || extension.eq_ignore_ascii_case("cjs")
                }) =>
            {
                Ok("commonjs")
            }
            None => bail!(
                "ERR_UNKNOWN_MODULE_FORMAT: unable to determine module type ({})",
                filename.display()
            ),
        }
    }

    fn state(&self) -> Result<Arc<ResolverState>> {
        let mut state_slot = self
            .state
            .lock()
            .map_err(|_| anyhow!("module resolver state poisoned"))?;
        if let Some(state) = state_slot.as_ref() {
            return Ok(state.clone());
        }
        let state = Arc::new(self.build_state()?);
        *state_slot = Some(state.clone());
        Ok(state)
    }

    pub(crate) fn cleanup(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.take();
        }
    }

    fn build_state(&self) -> Result<ResolverState> {
        let (root, owned_temp_root) = match self.resources.filesystem_root() {
            Some(root) => {
                let root = root.canonicalize().with_context(|| {
                    format!("canonicalize application resource root {}", root.display())
                })?;
                anyhow::ensure!(root.is_dir(), "invalid application resource root");
                (root, None)
            }
            None => {
                let root = create_private_temp_root(&std::env::temp_dir(), &self.app_uuid)?;
                if let Err(error) = self.resources.extract_all(&root) {
                    let _ = fs::remove_dir_all(&root);
                    return Err(error).context("materialize embedded application resources");
                }
                let root = root
                    .canonicalize()
                    .context("canonicalize extracted application resources")?;
                (root.clone(), Some(root))
            }
        };
        let mut options = ResolveOptions::default();
        options.cwd = Some(root.clone());
        // This runtime has no native addon capability, so it must not select
        // a `node-addons` export branch ahead of a JavaScript fallback.
        options.condition_names = vec!["node".into(), "require".into()];
        // Node's extensionless CommonJS lookup order is .js, .json, .node.
        // Explicit .cjs/.mjs requests still resolve by their full path and
        // are classified below; .node is resolved only to return a deliberate
        // native-addon rejection instead of silently skipping it.
        options.extensions = vec![".js".into(), ".json".into(), ".node".into()];
        options.main_fields = vec!["main".into()];
        options.main_files = vec!["index".into()];
        options.modules = vec!["node_modules".into()];
        options.builtin_modules = true;
        options.module_type = true;
        options.symlinks = true;
        // Trusted local module loading follows Node's ancestor node_modules
        // lookup. Bridge origin/token authentication is the permission bound;
        // unlike HTTP resource serving, this is not a resource-root sandbox.
        Ok(ResolverState {
            root,
            owned_temp_root,
            resolver: ResolverGeneric::new_with_file_system(ModuleFileSystem, options),
        })
    }
}

fn map_resolve_error(error: oxc_resolver::ResolveError) -> anyhow::Error {
    use oxc_resolver::ResolveError;
    match error {
        error @ (ResolveError::NotFound(_) | ResolveError::MatchedAliasNotFound(_, _)) => {
            anyhow!("MODULE_NOT_FOUND: {error}")
        }
        ResolveError::IOError(error) => {
            let error: io::Error = error.into();
            if error.kind() == io::ErrorKind::NotFound {
                anyhow!("MODULE_NOT_FOUND: {error}")
            } else {
                anyhow!(error)
            }
        }
        error @ ResolveError::PackagePathNotExported { .. } => {
            anyhow!("ERR_PACKAGE_PATH_NOT_EXPORTED: {error}")
        }
        error @ (ResolveError::InvalidModuleSpecifier(_, _)
        | ResolveError::PackageImportNotDefined(_, _)
        | ResolveError::Specifier(_)) => anyhow!("ERR_INVALID_MODULE_SPECIFIER: {error}"),
        error @ (ResolveError::Json(_)
        | ResolveError::InvalidPackageTarget(_, _, _)
        | ResolveError::InvalidPackageConfig(_)
        | ResolveError::InvalidPackageConfigDefault(_)
        | ResolveError::InvalidPackageConfigDirectory(_)) => {
            anyhow!("ERR_INVALID_PACKAGE_CONFIG: {error}")
        }
        other => anyhow!("ERR_MODULE_RESOLUTION: {other}"),
    }
}

fn normalize_module_source(mut source: String, module_type: &str) -> String {
    if let Some(without_bom) = source.strip_prefix('\u{feff}') {
        source = without_bom.to_owned();
    }
    if module_type == "commonjs" && source.starts_with("#!") {
        if let Some(newline) = source.find('\n') {
            source.replace_range(..newline, "");
        } else {
            source.clear();
        }
    }
    source
}

impl Drop for ResolverState {
    fn drop(&mut self) {
        if let Some(root) = &self.owned_temp_root {
            let _ = fs::remove_dir_all(root);
        }
    }
}

#[derive(Clone, Debug)]
struct ModuleFileSystem;

impl FileSystem for ModuleFileSystem {
    fn new() -> Self {
        Self
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        read_regular_file_bounded(path, MAX_RESOLVER_FILE_BYTES)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 file"))
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let metadata = FileSystemOs::metadata(path)?;
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "module resolution refuses special files",
            ));
        }
        Ok(metadata)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let metadata = FileSystemOs::symlink_metadata(path)?;
        if !metadata.is_file() && !metadata.is_dir() && !metadata.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "module resolution refuses special files",
            ));
        }
        Ok(metadata)
    }

    fn read_link(&self, path: &Path) -> std::result::Result<PathBuf, oxc_resolver::ResolveError> {
        FileSystemOs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        FileSystemOs::canonicalize(path)
    }
}

fn read_regular_file_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy().to_ascii_lowercase();
        if text.starts_with("\\\\.\\") || text.starts_with("\\\\?\\globalroot\\") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "device paths are not regular module files",
            ));
        }
    }

    let before = fs::metadata(path)?;
    if !before.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "module source is not a regular file",
        ));
    }
    if before.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "module source exceeds the configured size limit",
        ));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A path can be replaced by a FIFO after metadata(); nonblocking open
        // plus fstat keeps that race from hanging the resolver thread.
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    let opened_before = file.metadata()?;
    if !opened_before.is_file() || opened_before.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "module source changed to a non-regular or oversized file",
        ));
    }
    let mut bytes = Vec::with_capacity(opened_before.len() as usize);
    Read::take(&mut file, limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "module source exceeds the configured size limit",
        ));
    }
    let opened_after = file.metadata()?;
    if opened_before.len() != opened_after.len()
        || opened_before.modified().ok() != opened_after.modified().ok()
    {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "module source changed while reading",
        ));
    }
    Ok(bytes)
}

fn create_private_temp_root(temp_dir: &Path, app_uuid: &str) -> Result<PathBuf> {
    fs::create_dir_all(temp_dir).with_context(|| format!("create {}", temp_dir.display()))?;
    for _ in 0..16 {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|error| anyhow!("random module temp name: {error}"))?;
        let nonce = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let root = temp_dir.join(format!(
            "{MODULE_TEMP_PREFIX}-{}-{}-{nonce}",
            std::process::id(),
            app_uuid
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&root) {
            Ok(()) => return Ok(root),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("create private CommonJS resource directory"),
        }
    }
    bail!("unable to allocate a unique CommonJS resource directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("niva-module-resolver-{id}"));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn resolver(root: &Path) -> ModuleResolver {
        let resources: Arc<dyn ResourceManager> =
            super::super::super::resource_manager::FileSystemResource::new(root).unwrap();
        ModuleResolver::new(resources, "a51c1728-d174-42d4-8f57-7d296c966b51".into())
    }

    #[test]
    fn resolves_commonjs_extensions_package_main_and_exports_conditions() {
        let temp = TestDir::new();
        fs::create_dir_all(temp.path().join("node_modules/pkg/lib")).unwrap();
        fs::write(temp.path().join("entry.cjs"), "").unwrap();
        fs::write(
            temp.path().join("node_modules/pkg/package.json"),
            r#"{"main":"lib/main.js","exports":{".":{"require":"./lib/require.cjs","import":"./lib/import.mjs"}}}"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/pkg/lib/main.js"),
            "module.exports=1;",
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/pkg/lib/require.cjs"),
            "module.exports=2;",
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/pkg/lib/import.mjs"),
            "export default 3;",
        )
        .unwrap();
        let resolver = resolver(temp.path());
        let parent = temp.path().join("entry.cjs").to_string_lossy().into_owned();
        assert!(
            resolver
                .resolve("pkg", &parent)
                .unwrap()
                .ends_with("require.cjs")
        );
        let loaded = resolver.load("pkg", &parent).unwrap();
        assert_eq!(loaded["type"], "commonjs");
        assert_eq!(loaded["source"], "module.exports=2;");
        assert!(
            resolver
                .resolve("./node_modules/pkg/lib/main", &parent)
                .unwrap()
                .ends_with("main.js")
        );
        assert_eq!(
            resolver
                .load("./node_modules/pkg/lib/main.js", &parent)
                .unwrap()["type"],
            "commonjs"
        );
    }

    #[test]
    fn exports_do_not_select_the_unavailable_node_addons_condition() {
        let temp = TestDir::new();
        fs::create_dir_all(temp.path().join("node_modules/native-fallback")).unwrap();
        fs::write(temp.path().join("entry.cjs"), "").unwrap();
        fs::write(
            temp.path().join("node_modules/native-fallback/package.json"),
            r#"{"exports":{".":{"node-addons":"./addon.node","require":"./fallback.cjs","default":"./fallback.cjs"}}}"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/native-fallback/addon.node"),
            "not executable",
        )
        .unwrap();
        fs::write(
            temp.path()
                .join("node_modules/native-fallback/fallback.cjs"),
            "module.exports = 'fallback';",
        )
        .unwrap();
        let resolver = resolver(temp.path());
        let parent = temp.path().join("entry.cjs").to_string_lossy().into_owned();
        assert!(
            resolver
                .resolve("native-fallback", &parent)
                .unwrap()
                .ends_with("fallback.cjs")
        );
    }

    #[test]
    fn resolution_errors_distinguish_missing_modules_from_export_policy() {
        let temp = TestDir::new();
        fs::create_dir_all(temp.path().join("node_modules/exports-only")).unwrap();
        fs::write(temp.path().join("entry.cjs"), "").unwrap();
        fs::write(
            temp.path().join("node_modules/exports-only/package.json"),
            r#"{"exports":{"./public":"./public.cjs"}}"#,
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/exports-only/public.cjs"),
            "module.exports = true;",
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/exports-only/private.cjs"),
            "module.exports = false;",
        )
        .unwrap();
        let resolver = resolver(temp.path());
        let missing = resolver.resolve("absent-package", "").unwrap_err();
        assert!(missing.to_string().starts_with("MODULE_NOT_FOUND:"));
        let parent = temp.path().join("entry.cjs").to_string_lossy().into_owned();
        let denied = resolver
            .resolve("exports-only/private", &parent)
            .unwrap_err();
        assert!(
            denied
                .to_string()
                .starts_with("ERR_PACKAGE_PATH_NOT_EXPORTED:")
        );
    }

    #[test]
    fn package_imports_resolve_local_files_and_return_builtin_ids_for_js_dispatch() {
        let temp = TestDir::new();
        fs::write(
            temp.path().join("package.json"),
            r##"{"imports":{"#local":"./local.cjs","#fs":"node:fs"}}"##,
        )
        .unwrap();
        fs::write(temp.path().join("entry.cjs"), "").unwrap();
        fs::write(temp.path().join("local.cjs"), "module.exports = 'local';").unwrap();
        let resolver = resolver(temp.path());
        let parent = temp.path().join("entry.cjs").to_string_lossy().into_owned();
        let local = resolver.load("#local", &parent).unwrap();
        assert!(local["filename"].as_str().unwrap().ends_with("local.cjs"));
        assert_eq!(local["source"], "module.exports = 'local';");
        assert_eq!(resolver.resolve("#fs", &parent).unwrap(), "node:fs");
        assert!(
            resolver
                .load("#fs", &parent)
                .unwrap_err()
                .to_string()
                .starts_with("ERR_UNSUPPORTED_BUILTIN_MODULE:")
        );
    }

    #[test]
    fn resolve_reports_esm_and_addon_paths_but_load_rejects_them() {
        let temp = TestDir::new();
        fs::write(temp.path().join("entry.cjs"), "").unwrap();
        fs::write(temp.path().join("esm.mjs"), "export default 1;").unwrap();
        fs::write(temp.path().join("addon.node"), "not a native addon").unwrap();
        let resolver = resolver(temp.path());
        let parent = temp.path().join("entry.cjs").to_string_lossy().into_owned();
        assert!(
            resolver
                .resolve("./esm.mjs", &parent)
                .unwrap()
                .ends_with("esm.mjs")
        );
        assert!(
            resolver
                .resolve("./addon.node", &parent)
                .unwrap()
                .ends_with("addon.node")
        );
        assert!(resolver.load("./esm.mjs", &parent).is_err());
        assert!(resolver.load("./addon.node", &parent).is_err());
    }

    #[test]
    fn json_modules_are_marked_for_the_javascript_loader() {
        let temp = TestDir::new();
        fs::write(temp.path().join("entry.js"), "").unwrap();
        fs::write(temp.path().join("settings.json"), r#"{"ok":true}"#).unwrap();
        let resolver = resolver(temp.path());
        let parent = temp.path().join("entry.js").to_string_lossy().into_owned();
        assert_eq!(
            resolver.load("./settings.json", &parent).unwrap()["type"],
            "json"
        );
    }

    #[test]
    fn ordinary_js_without_package_type_is_commonjs() {
        let temp = TestDir::new();
        fs::write(temp.path().join("entry.js"), "module.exports = 1;").unwrap();
        let resolver = resolver(temp.path());
        let loaded = resolver.load("./entry.js", "").unwrap();
        assert_eq!(loaded["type"], "commonjs");
        assert_eq!(loaded["source"], "module.exports = 1;");
    }

    #[test]
    fn create_require_accepts_a_missing_absolute_parent_filename() {
        let temp = TestDir::new();
        let directory = temp.path().join("virtual");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("sibling.js"), "module.exports = 'sibling';").unwrap();
        let absolute_target = temp.path().join("absolute-target.cjs");
        fs::write(&absolute_target, "module.exports = 'absolute';").unwrap();
        let missing_parent = directory.join("entry-not-created.js");
        assert!(!missing_parent.exists());
        let resolver = resolver(temp.path());
        assert!(
            resolver
                .resolve("./sibling.js", &missing_parent.to_string_lossy())
                .unwrap()
                .ends_with("sibling.js")
        );
        let virtual_parent = temp
            .path()
            .join("directory-that-does-not-exist/nested/require.js");
        assert!(!virtual_parent.parent().unwrap().exists());
        assert!(
            resolver
                .resolve(
                    &absolute_target.to_string_lossy(),
                    &virtual_parent.to_string_lossy()
                )
                .unwrap()
                .ends_with("absolute-target.cjs")
        );
    }

    #[test]
    fn load_removes_utf8_bom_and_commonjs_hashbang() {
        let temp = TestDir::new();
        fs::write(
            temp.path().join("cli.js"),
            "\u{feff}#!/usr/bin/env node\nmodule.exports = 1;",
        )
        .unwrap();
        fs::write(
            temp.path().join("settings.json"),
            "\u{feff}{\"enabled\":true}",
        )
        .unwrap();
        let resolver = resolver(temp.path());
        let cli = resolver.load("./cli.js", "").unwrap();
        assert_eq!(cli["source"], "\nmodule.exports = 1;");
        let settings = resolver.load("./settings.json", "").unwrap();
        assert_eq!(settings["source"], "{\"enabled\":true}");
    }

    #[test]
    fn empty_parent_starts_at_resource_root_and_module_parent_searches_ancestors() {
        let temp = TestDir::new();
        let checkout = temp.path();
        let compiled = checkout.join(".niva-compiled");
        fs::create_dir_all(compiled.join("backend")).unwrap();
        fs::write(
            compiled.join("backend/app.js"),
            "module.exports = require('root-package');",
        )
        .unwrap();
        fs::create_dir_all(checkout.join("node_modules/root-package")).unwrap();
        fs::write(
            checkout.join("node_modules/root-package/package.json"),
            r#"{"main":"index.js"}"#,
        )
        .unwrap();
        fs::write(
            checkout.join("node_modules/root-package/index.js"),
            "module.exports = 'ancestor';",
        )
        .unwrap();

        let resolver = resolver(&compiled);
        let app = resolver.load("./backend/app.js", "").unwrap();
        assert!(
            Path::new(app["filename"].as_str().unwrap())
                .ends_with(Path::new("backend").join("app.js"))
        );
        let package = resolver
            .load("root-package", app["filename"].as_str().unwrap())
            .unwrap();
        assert_eq!(package["source"], "module.exports = 'ancestor';");
    }
}
