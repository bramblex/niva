use niva_packager::{Target, build, sha256};
use serde_json::json;
use std::{fs, path::Path};

fn fixture(root: &Path) {
    fs::create_dir(root.join("resources")).unwrap();
    fs::write(root.join("resources/index.html"), "hello").unwrap();
    fs::write(
        root.join("niva.json"),
        r#"{"name":"Smoke","uuid":"test","version":"1.0.0"}"#,
    )
    .unwrap();
}
#[test]
fn bad_hash_and_missing_target_are_reported_independently_without_publishing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    fs::write(
        root.join("runtime"),
        [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1],
    )
    .unwrap();
    fs::write(root.join("manifest.json"),json!({"schemaVersion":1,"version":env!("CARGO_PKG_VERSION"),"runtimes":{"macos-aarch64":{"path":"runtime","version":env!("CARGO_PKG_VERSION"),"sha256":"0".repeat(64)}}}).to_string()).unwrap();
    let report = build(
        &root.join("manifest.json"),
        &root.join("niva.json"),
        &root.join("resources"),
        &root.join("output"),
        &[Target::MacosAarch64, Target::WindowsX86_64],
        None,
    )
    .unwrap();
    assert_eq!(report.results.len(), 2);
    assert!(
        report.results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("SHA256 mismatch")
    );
    assert!(
        report.results[1]
            .error
            .as_ref()
            .unwrap()
            .contains("missing")
    );
    assert_eq!(fs::read_dir(root.join("output")).unwrap().count(), 0);
}
#[test]
fn existing_artifact_is_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let runtime = [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
    fs::write(root.join("runtime"), runtime).unwrap();
    fs::write(root.join("manifest.json"),json!({"schemaVersion":1,"version":env!("CARGO_PKG_VERSION"),"runtimes":{"macos-aarch64":{"path":"runtime","version":env!("CARGO_PKG_VERSION"),"sha256":sha256(&runtime)}}}).to_string()).unwrap();
    fs::create_dir(root.join("output")).unwrap();
    let destination = root.join("output/Smoke-macos-aarch64.zip");
    fs::write(&destination, b"existing-user-output").unwrap();
    let report = build(
        &root.join("manifest.json"),
        &root.join("niva.json"),
        &root.join("resources"),
        &root.join("output"),
        &[Target::MacosAarch64],
        None,
    )
    .unwrap();
    assert!(
        report.results[0]
            .error
            .as_ref()
            .unwrap()
            .contains("already exists")
    );
    assert_eq!(fs::read(&destination).unwrap(), b"existing-user-output");
    assert_eq!(fs::read_dir(root.join("output")).unwrap().count(), 1);
}
