use std::{
    collections::BTreeSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use flate2::{Compression, write::DeflateEncoder};
use serde_json::Value;

fn strings(value: &Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("runtime-files.json field {label} must be an array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("runtime-files.json field {label} must contain strings"))
                .to_owned()
        })
        .collect()
}

fn main() {
    let manifest_path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../packages/node-compat/runtime-files.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let package_root = manifest_path
        .parent()
        .expect("runtime manifest must be inside packages/node-compat")
        .to_path_buf();
    let manifest_bytes = fs::read(&manifest_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest_path.display()));
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .unwrap_or_else(|error| panic!("invalid {}: {error}", manifest_path.display()));

    let classic_files = strings(&manifest["classic"], "classic");
    let shared_files = strings(&manifest["shared"], "shared");
    let modules = manifest["modules"]
        .as_object()
        .unwrap_or_else(|| panic!("runtime-files.json field modules must be an object"));
    let required_classic = [
        "src/runtime/bridge.js",
        "src/runtime/vendor.js",
        "src/runtime/registration.js",
    ];
    for required in required_classic {
        if !classic_files.iter().any(|file| file == required) {
            panic!("classic source order must include {required}");
        }
    }

    let mut classic = String::new();
    for relative in &classic_files {
        let source_path = package_root.join(relative);
        println!("cargo:rerun-if-changed={}", source_path.display());
        let source = fs::read_to_string(&source_path).unwrap_or_else(|error| {
            panic!(
                "missing NodeCompat source {}: {error}",
                source_path.display()
            )
        });
        classic.push_str(&format!("/* {relative} */\n{}\n", source.trim()));
    }
    classic.push_str(
        r#"(function (root) {
  "use strict";
  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (!root.Niva) throw new Error("Load the Niva initialize script before node-compat.");
  root.NivaNodeCompatReady = runtime.registerNodeCompat(root.Niva);
})(globalThis);"#,
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let data_path = out_dir.join("node_compat_assets.deflate");
    let index_path = out_dir.join("node_compat_assets.rs");
    let mut compressed_assets = Vec::new();
    let mut asset_index = Vec::<(String, usize, usize, usize)>::new();
    let mut seen = BTreeSet::new();

    let classic_bytes = classic.into_bytes();
    append_asset(
        "node-compat.js".to_owned(),
        &classic_bytes,
        &mut compressed_assets,
        &mut asset_index,
    );
    seen.insert("node-compat.js".to_owned());

    let mut declared_assets = shared_files;
    for (module, file_list) in modules {
        let files = strings(file_list, &format!("modules.{module}"));
        declared_assets.extend(files);
    }
    for relative in declared_assets {
        if !seen.insert(relative.clone()) {
            continue;
        }
        let source_path = package_root.join(&relative);
        println!("cargo:rerun-if-changed={}", source_path.display());
        let bytes = fs::read(&source_path).unwrap_or_else(|error| {
            panic!(
                "missing NodeCompat asset {}: {error}",
                source_path.display()
            )
        });
        append_asset(relative, &bytes, &mut compressed_assets, &mut asset_index);
    }

    fs::write(&data_path, compressed_assets)
        .unwrap_or_else(|error| panic!("cannot write embedded NodeCompat data: {error}"));

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
        generated.push_str(&format!(
            "EmbeddedNodeCompatAsset {{ path: {path:?}, offset: {offset}, compressed_len: {compressed_len}, raw_len: {raw_len} }},\n"
        ));
    }
    generated.push_str("];\n");

    generated.push_str("pub(crate) static NODE_COMPAT_SHARED_ASSETS: &[&str] = &[\n");
    for relative in strings(&manifest["shared"], "shared") {
        generated.push_str(&format!("{relative:?},\n"));
    }
    generated.push_str("];\n");

    generated.push_str("pub(crate) static NODE_COMPAT_MODULE_ASSETS: &[(&str, &[&str])] = &[\n");
    for (module, file_list) in modules {
        let files = strings(file_list, &format!("modules.{module}"));
        generated.push_str(&format!("({module:?}, &["));
        for relative in files {
            generated.push_str(&format!("{relative:?},"));
        }
        generated.push_str("]),\n");
    }
    generated.push_str("];\n");
    fs::write(&index_path, generated)
        .unwrap_or_else(|error| panic!("cannot write embedded NodeCompat index: {error}"));

    println!(
        "cargo:rerun-if-changed={}",
        Path::new("../../packages/node-compat/runtime-files.json").display()
    );
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
        .unwrap_or_else(|error| panic!("cannot compress NodeCompat asset {path}: {error}"));
    let compressed = encoder
        .finish()
        .unwrap_or_else(|error| panic!("cannot finish NodeCompat asset {path}: {error}"));
    let compressed_len = compressed.len();
    output.extend_from_slice(&compressed);
    index.push((path, offset, compressed_len, bytes.len()));
}
