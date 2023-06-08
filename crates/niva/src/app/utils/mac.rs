use std::path::PathBuf;

pub fn get_app_folder() -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent_dir) = exe_path.parent() {
            if let Some(parent_dir_name) = parent_dir.file_name() {
                if parent_dir_name == "MacOS" {
                    if let Some(app_dir) = parent_dir.parent() {
                        if let Some(extension) = app_dir.extension() {
                            if extension == "app" {
                                return Some(app_dir.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
