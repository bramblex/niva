#[cfg(target_os = "windows")]
extern crate winres;

#[cfg(target_os = "windows")]
fn main() {
    let res = winres::WindowsResource::new();
    res.set_icon("src/assets/icon.ico");
    res.compile().unwrap();
    build_version::write_version_file().expect("Failed to write version.rs file");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    build_version::write_version_file().expect("Failed to write version.rs file");
}
