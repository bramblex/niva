#[cfg(target_os = "windows")]
extern crate winres;

#[cfg(target_os = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.compile().unwrap();
    build_version::write_version_file().expect("Failed to write version.rs file");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    build_version::write_version_file().expect("Failed to write version.rs file");
}
