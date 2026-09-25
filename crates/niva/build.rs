use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use flate2::{Compression, write::DeflateEncoder};
use serde_json::Value;
use sha2::{Digest, Sha256};

const RUNTIME_PREFIX: &str = "packages/runtime/";
const TYPES_PREFIX: &str = "packages/types/";
const EMBEDDED_BOOTSTRAP: &str = "__bootstrap__/bootstrap.js";

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn manifest_string<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("runtime build manifest field {field} must be a string"))
}

fn safe_relative_path<'a>(value: &'a str, label: &str) -> &'a Path {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        panic!("invalid {label} path in runtime build manifest: {value}");
    }
    path
}

fn verify_build_manifest(
    repo_root: &Path,
    runtime_dist: &Path,
    build_manifest_path: &Path,
) -> BTreeMap<String, Vec<u8>> {
    let data = fs::read(build_manifest_path).unwrap_or_else(|error| {
        panic!("missing generated Niva runtime assets; run `npm run build --workspace=packages/runtime`: {error}")
    });
    let manifest: Value = serde_json::from_slice(&data)
        .unwrap_or_else(|error| panic!("invalid runtime build manifest: {error}"));
    if manifest["format"] != 1 {
        panic!(
            "unsupported runtime build manifest format; run `npm run build --workspace=packages/runtime`"
        );
    }

    let inputs = manifest["inputs"]
        .as_array()
        .unwrap_or_else(|| panic!("runtime build manifest inputs must be an array"));
    let mut input_material = Vec::with_capacity(inputs.len());
    for input in inputs {
        let relative = manifest_string(input, "path");
        let path = safe_relative_path(relative, "runtime input");
        if relative != "package.json"
            && relative != "package-lock.json"
            && !relative.starts_with(RUNTIME_PREFIX)
            && !relative.starts_with(TYPES_PREFIX)
        {
            panic!("runtime build manifest input is outside the allowlist: {relative}");
        }
        let absolute = repo_root.join(path);
        println!("cargo:rerun-if-changed={}", absolute.display());
        let bytes = fs::read(&absolute).unwrap_or_else(|error| {
            panic!("runtime build input is missing (run `npm run build --workspace=packages/runtime`): {}: {error}", absolute.display())
        });
        let actual = hex_sha256(&bytes);
        let expected = manifest_string(input, "sha256");
        if actual != expected {
            panic!(
                "Niva runtime assets are stale (run `npm run build --workspace=packages/runtime`): {relative}"
            );
        }
        input_material.push(format!("{relative}:{actual}"));
    }
    input_material.sort();
    let fingerprint = hex_sha256(input_material.join("\n").as_bytes());
    if fingerprint != manifest_string(&manifest, "inputFingerprint") {
        panic!(
            "Niva runtime input fingerprint mismatch; run `npm run build --workspace=packages/runtime`"
        );
    }

    let outputs = manifest["outputs"]
        .as_array()
        .unwrap_or_else(|| panic!("runtime build manifest outputs must be an array"));
    let mut verified_outputs = BTreeMap::new();
    for output in outputs {
        let relative = manifest_string(output, "path");
        let path = safe_relative_path(relative, "runtime output");
        let absolute = runtime_dist.join(path);
        println!("cargo:rerun-if-changed={}", absolute.display());
        let bytes = fs::read(&absolute).unwrap_or_else(|error| {
            panic!("runtime output is missing: {}: {error}", absolute.display())
        });
        if hex_sha256(&bytes) != manifest_string(output, "sha256") {
            panic!(
                "Niva runtime output does not match its build manifest: {relative}; run `npm run build --workspace=packages/runtime`"
            );
        }
        if verified_outputs
            .insert(relative.to_owned(), bytes)
            .is_some()
        {
            panic!("duplicate runtime output in build manifest: {relative}");
        }
    }
    for required in ["bootstrap.js", "runtime-assets.json"] {
        if !verified_outputs.contains_key(required) {
            panic!(
                "runtime build manifest does not cover {required}; run `npm run build --workspace=packages/runtime`"
            );
        }
    }
    verified_outputs
}

fn main() {
    let crate_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = crate_root
        .join("../..")
        .canonicalize()
        .expect("repository root must exist");
    let runtime_root = repo_root.join("packages/runtime");
    let runtime_dist = runtime_root.join("dist");
    let static_manifest = runtime_root.join("runtime-files.json");
    let assets_manifest_path = runtime_dist.join("runtime-assets.json");
    let build_manifest_path = runtime_dist.join("build-manifest.json");
    println!("cargo:rerun-if-changed={}", Path::new("build.rs").display());
    println!("cargo:rerun-if-changed={}", static_manifest.display());
    println!("cargo:rerun-if-changed={}", assets_manifest_path.display());
    println!("cargo:rerun-if-changed={}", build_manifest_path.display());

    // Embed exactly the bytes whose hashes were verified. A parallel JS build
    // must not replace files between verification and the compressed snapshot.
    let verified_outputs = verify_build_manifest(&repo_root, &runtime_dist, &build_manifest_path);
    let assets: Value = serde_json::from_slice(&verified_outputs["runtime-assets.json"])
        .unwrap_or_else(|error| panic!("invalid generated runtime asset map: {error}"));

    let bootstrap_path =
        safe_relative_path(manifest_string(&assets, "bootstrap"), "bootstrap asset");
    if bootstrap_path != Path::new("bootstrap.js") {
        panic!("runtime bootstrap output must be bootstrap.js");
    }
    let bootstrap = &verified_outputs["bootstrap.js"];
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let data_path = out_dir.join("node_compat_assets.deflate");
    let index_path = out_dir.join("node_compat_assets.rs");
    let mut compressed_assets = Vec::new();
    let mut asset_index = Vec::<(String, usize, usize, usize)>::new();
    let mut seen = BTreeSet::new();
    append_asset(
        EMBEDDED_BOOTSTRAP.to_owned(),
        bootstrap,
        &mut compressed_assets,
        &mut asset_index,
    );
    seen.insert(EMBEDDED_BOOTSTRAP.to_owned());

    let shared = strings(&assets["shared"], "shared");
    let modules = assets["modules"]
        .as_object()
        .unwrap_or_else(|| panic!("runtime asset map modules must be an object"));
    let mut declared = shared.clone();
    let mut module_assets = Vec::new();
    for (module, file_list) in modules {
        let files = strings(file_list, &format!("modules.{module}"));
        module_assets.push((module.clone(), files.clone()));
        declared.extend(files);
    }
    for relative in declared {
        safe_relative_path(&relative, "runtime asset");
        if relative.contains("..") || relative.contains('\\') || !relative.starts_with("esm/") {
            panic!("invalid runtime asset route: {relative}");
        }
        if !seen.insert(relative.clone()) {
            continue;
        }
        let bytes = verified_outputs.get(&relative).unwrap_or_else(|| {
            panic!("runtime asset is absent from the verified build manifest: {relative}")
        });
        append_asset(relative, bytes, &mut compressed_assets, &mut asset_index);
    }

    fs::write(&data_path, compressed_assets).expect("cannot write embedded runtime data");
    let mut generated = String::from(
        "pub(crate) struct EmbeddedNodeCompatAsset {\n\
         pub(crate) path: &'static str,\n\
         pub(crate) offset: usize,\n\
         pub(crate) compressed_len: usize,\n\
         pub(crate) raw_len: usize,\n\
         }\n\
         pub(crate) static EMBEDDED_NODE_COMPAT_ASSETS: &[EmbeddedNodeCompatAsset] = &[\n",
    );
    for (path, offset, compressed_len, raw_len) in &asset_index {
        generated.push_str(&format!("EmbeddedNodeCompatAsset {{ path: {path:?}, offset: {offset}, compressed_len: {compressed_len}, raw_len: {raw_len} }},\n"));
    }
    generated.push_str("];\npub(crate) static NODE_COMPAT_SHARED_ASSETS: &[&str] = &[\n");
    for path in &shared {
        generated.push_str(&format!("{path:?},\n"));
    }
    generated
        .push_str("];\npub(crate) static NODE_COMPAT_MODULE_ASSETS: &[(&str, &[&str])] = &[\n");
    for (module, files) in &module_assets {
        generated.push_str(&format!("({module:?}, &["));
        for path in files {
            generated.push_str(&format!("{path:?},"));
        }
        generated.push_str(" ]),\n");
    }
    generated.push_str("];\n");
    fs::write(&index_path, generated).expect("cannot write embedded runtime index");
}

fn strings(value: &Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("runtime asset map field {label} must be an array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("runtime asset map field {label} must contain strings"))
                .to_owned()
        })
        .collect()
}

fn append_asset(
    path: String,
    bytes: &[u8],
    output: &mut Vec<u8>,
    index: &mut Vec<(String, usize, usize, usize)>,
) {
    let offset = output.len();
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
    encoder
        .write_all(bytes)
        .unwrap_or_else(|error| panic!("cannot compress runtime asset {path}: {error}"));
    let compressed = encoder
        .finish()
        .unwrap_or_else(|error| panic!("cannot finish runtime asset {path}: {error}"));
    let compressed_len = compressed.len();
    output.extend_from_slice(&compressed);
    index.push((path, offset, compressed_len, bytes.len()));
}
