fn main() {
    // let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    // if profile == "debug" {
    //     // Debug 构建配置
    //     println!("Building in Debug mode");
    //     // 在这里执行针对 Debug 构建的操作
    // } else {
    //     // Release 构建配置
    //     println!("Building in Release mode");
    //     // 在这里执行针对 Release 构建的操作
    // }

    build_version::write_version_file().expect("Failed to write version.rs file");
}
