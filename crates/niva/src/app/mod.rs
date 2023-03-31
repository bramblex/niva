mod api;
mod api_manager;
mod event_handler;
mod options;
mod resource_manager;
mod shortcut_manager;
mod tray_manager;
mod utils;
mod window_manager;

use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, MutexGuard},
};

use tao::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget};

use crate::lock;

use self::{
    api::register_api_instances,
    api_manager::ApiManager,
    event_handler::EventHandler,
    options::NivaOptions,
    resource_manager::{AppResourceManager, FileSystemResource, ResourceManager},
    shortcut_manager::NivaShortcutManager,
    tray_manager::NivaTrayManager,
    utils::{arc, arc_mut, ArcMut},
    window_manager::{options::NivaWindowOptions, WindowManager},
};

pub type NivaId = u32;
pub type NivaEventLoop = EventLoop<NivaEvent>;
pub type NivaEventLoopProxy = EventLoopProxy<NivaEvent>;
pub type NivaWindowTarget = EventLoopWindowTarget<NivaEvent>;

pub type NivaCallback = Pin<Box<dyn Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send>>;
pub struct NivaEvent(NivaCallback);

impl Debug for NivaEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NivaEvent").finish()
    }
}

impl NivaEvent {
    pub fn new<F: Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send + 'static>(
        f: F,
    ) -> Self {
        Self(Box::pin(f))
    }
}

impl Deref for NivaEvent {
    type Target = Pin<Box<dyn Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct NivaApp {
    launch_info: NivaLaunchInfo, // NivaApp launch info, contains this command line arguments and niva.json project options.

    _resource: Arc<dyn ResourceManager>,
    _window: ArcMut<WindowManager>, // Window manager.
    _api: ArcMut<ApiManager>,
    _shortcut: ArcMut<NivaShortcutManager>,
    _tray: ArcMut<NivaTrayManager>,

    event_loop_proxy: EventLoopProxy<NivaEvent>, // Event loop proxy.
}

impl NivaApp {
    pub fn new(event_loop: &mut NivaEventLoop) -> Result<Arc<NivaApp>> {
        let arguments = NivaArguments::new();

        let resource_manager: Arc<dyn ResourceManager> = match &arguments.debug_resource {
            Some(dir) => FileSystemResource::new(dir)?,
            None => AppResourceManager::new()?,
        };

        let launch_info = NivaLaunchInfo::new(arguments, resource_manager.clone())?;

        #[cfg(target_os = "macos")]
        {
            use self::options::NivaActivationPolicy;
            use wry::application::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
            if let Some(p) = launch_info.options.activation.clone() {
                let policy = match p {
                    NivaActivationPolicy::Regular => ActivationPolicy::Regular,
                    NivaActivationPolicy::Accessory => ActivationPolicy::Accessory,
                    NivaActivationPolicy::Prohibited => ActivationPolicy::Prohibited,
                };
                event_loop.set_activation_policy(policy);
            }
        }

        // create api manager and register api instances
        let api_manager = ApiManager::new(&launch_info.options);
        {
            let mut api_manager = lock!(api_manager)?;
            register_api_instances(&mut api_manager);
        }

        let window_manager = WindowManager::new(&launch_info);

        // build shortcuts
        let shortcut_manager = NivaShortcutManager::new(&launch_info.options.shortcuts, event_loop);

        let tray_manager = NivaTrayManager::new();

        let app = Arc::new(NivaApp {
            launch_info,

            _resource: resource_manager,
            _window: window_manager.clone(),
            _api: api_manager.clone(),
            _shortcut: shortcut_manager,
            _tray: tray_manager.clone(),

            event_loop_proxy: event_loop.create_proxy(),
        });

        // bind app to window manager
        lock!(window_manager)?.bind_app(app.clone());
        lock!(api_manager)?.bind_app(app.clone());
        lock!(tray_manager)?.bind_app(app.clone());

        Ok(app)
    }

    pub fn resource<'a>(self: &'a Arc<Self>) -> Arc<dyn ResourceManager>{
        self._resource.clone()
    }

    pub fn window<'a>(self: &'a Arc<Self>) -> Result<MutexGuard<'a, WindowManager>> {
        lock!(self._window)
    }

    pub fn api<'a>(self: &'a Arc<Self>) -> Result<MutexGuard<'a, ApiManager>> {
        lock!(self._api)
    }

    pub fn shortcut<'a>(self: &'a Arc<Self>) -> Result<MutexGuard<'a, NivaShortcutManager>> {
        lock!(self._shortcut)
    }

    pub fn tray<'a>(self: &'a Arc<Self>) -> Result<MutexGuard<'a, NivaTrayManager>> {
        lock!(self._tray)
    }

    pub fn run(self: Arc<NivaApp>, event_loop: NivaEventLoop) -> Result<()> {
        // create niva main window to launch application.
        let main_window_options: &NivaWindowOptions = &self.clone().launch_info.options.window;
        let main_window = self
            .window()?
            .open_window(main_window_options, &event_loop)?;

        let tray_options = &self.launch_info.options.tray.clone();
        if let Some(options) = tray_options {
            let _ = self.tray()?.create(options, &event_loop)?;
        }

        let event_handler = EventHandler::new(self, main_window);
        event_loop.run(move |event, target, control_flow| {
            event_handler.handle(event, target, control_flow);
        });
    }
}

#[derive(Debug)]
pub struct NivaArguments {
    pub debug_devtools: bool,
    pub debug_resource: Option<PathBuf>,
    pub debug_entry: Option<String>,
}

impl NivaArguments {
    pub fn new() -> Self {
        let args = std::env::args().collect::<Vec<String>>();

        // parse args
        let mut args_map = HashMap::<String, String>::new();

        for arg in args.iter().skip(1) {
            if arg.starts_with("--") {
                let mut arg = arg.splitn(2, '=');
                let key = arg.next().unwrap().trim_start_matches("--");
                let value = arg.next().unwrap_or("");
                args_map.insert(key.to_string(), value.to_string());
            }
        }


        let debug_devtools = args_map
            .get("debug-devtools")
            .map(|v| v == "true")
            .unwrap_or(false);
        let debug_resource = args_map.get("debug-resource").map(PathBuf::from);
        let debug_entry = args_map.get("debug-entry").map(|v| v.to_string());

        Self {
            debug_devtools,
            debug_resource,
            debug_entry,
        }
    }
}

#[derive(Debug)]
pub struct NivaLaunchInfo {
    pub name: String,         // Name of the project.
    pub uuid: String,         // UUID of the project.
    pub id_name: String, // Name of the project with short UUID, Truncate the UUID to obtain the first eight characters. This is used to create data directory, cache directory and temporary directory.
    pub data_dir: PathBuf, // Data directory of the project. This iw where application local data is stored. such as extracted resources files.
    pub cache_dir: PathBuf, // Cache directory of the project.
    pub temp_dir: PathBuf, // This is where temporary files are stored such as
    pub options: NivaOptions, // Project options, read from niva.json.
    pub arguments: NivaArguments,
}

impl NivaLaunchInfo {
    pub fn new(
        arguments: NivaArguments,
        resource_manager: Arc<dyn ResourceManager>,
    ) -> Result<NivaLaunchInfo> {
        let content = resource_manager.load("niva.json".to_string())?;
        let options: NivaOptions = serde_json::from_slice(&content)?;

        let name = options.name.clone();
        let uuid = options.uuid.clone();
        let id_name = format!("{}_{}", name, &uuid[0..8]);

        let base_dirs = BaseDirs::new().ok_or(anyhow!("Failed to get user directories"))?;
        let temp_dir = std::env::temp_dir().join(&id_name);
        let data_dir = base_dirs.data_dir().join(&id_name);
        let cache_dir = base_dirs.cache_dir().join(&id_name);

        Ok(NivaLaunchInfo {
            name,
            uuid,
            id_name,
            data_dir,
            cache_dir,
            temp_dir,
            options,
            arguments,
        })
    }
}
