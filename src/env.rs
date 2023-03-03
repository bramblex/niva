use serde::Deserialize;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use wry::application::dpi::{
    LogicalPosition, LogicalSize, Position as DpiPosition, Size as DpiSize,
};

#[derive(Deserialize, Debug)]
pub struct Size(pub f64, pub f64);

impl Size {
    pub fn to_dpi_size(&self) -> DpiSize {
        DpiSize::Logical(LogicalSize::new(self.0, self.1))
    }
}

#[derive(Deserialize, Debug)]
pub struct Position(pub f64, pub f64);

impl Position {
    pub fn to_dpi_position(&self) -> DpiPosition {
        DpiPosition::Logical(LogicalPosition::new(self.0, self.1))
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum MenuItemConfig {
    NativeItem(String),
    MenuItem(String, u16),
    SubMenu(String, Vec<MenuItemConfig>),
}

#[derive(Deserialize, Debug)]
pub struct MenuConfig(pub Vec<MenuItemConfig>);

#[derive(Deserialize, Debug)]
pub struct Config {
    // project config
    pub name: String,
    pub icon: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    pub website: Option<String>,
    pub website_label: Option<String>,

    // webview config
    pub entry: Option<String>,
    pub background_color: Option<(u8, u8, u8, u8)>,
    pub devtools: Option<bool>,

    // window config
    pub title: Option<String>,
    pub theme: Option<String>,
    pub size: Option<Size>,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,

    pub position: Option<Position>,

    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,

    pub fullscreen: Option<bool>,
    pub maximized: Option<bool>,
    pub visible: Option<bool>,
    pub transparent: Option<bool>,
    pub decorations: Option<bool>,

    pub always_on_top: Option<bool>,
    pub always_on_bottom: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,

    pub focused: Option<bool>,
    pub content_protection: Option<bool>,

    // window menu
    pub menu: Option<MenuConfig>,

    // runtime config
    pub workers: Option<usize>,
}

fn get_work_dir() -> Result<PathBuf> {
    // if work dir is specified in command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let cwd = std::env::current_dir()?;
        let custom_path = Path::new(args[1].as_str()).to_path_buf();
        let full_path = cwd.join(custom_path);

        // if custom_path a directory and exists, return it
        if full_path.is_dir() {
            return Ok(full_path);
        }

        // if custom_path is not a directory, return error
        let err = Error::new(
            ErrorKind::Other,
            "Custom path is not a directory or not exists",
        );
        return Err(err);
    }

    // if work dir is not specified in command line arguments,
    // return executable dir path as default work dir
    let executable_path = std::env::current_exe()?;
    // executable parent always exists
    let default_work_dir = executable_path.parent().unwrap().to_path_buf();
    Ok(default_work_dir)
}

fn get_config(work_dir: &Path) -> Result<Config> {
    let config_path = work_dir.join("tauri-lite.json");
    let content =
        std::fs::read_to_string(config_path).unwrap_or("{ \"name\": \"tauri-lite\" }".to_owned());

    let config = serde_json::from_str::<Config>(&content)?;
    Ok(config)
}

pub fn init() -> Result<(PathBuf, Config)> {
    let work_dir = get_work_dir()?;
    let config = get_config(&work_dir)?;
    std::env::set_current_dir(&work_dir)?;
    Ok((work_dir, config))
}
