mod api;
mod api_manager;
mod assets;
pub(crate) mod custom_protocol;
mod event_handler;
pub(crate) mod fs_ops;
pub(crate) mod http_server;
#[cfg(target_os = "macos")]
mod ipc_macos;
#[cfg(target_os = "windows")]
mod ipc_windows_frames;
pub(crate) mod main_exec;
mod menu;
mod node_bootstrap;
mod node_compat;
mod options;
pub(crate) mod os_native;
mod resource_manager;
mod shortcut_manager;
mod stdio;
mod tray_manager;
mod utils;
mod window_manager;

use anyhow::{Result, anyhow};
use directories::BaseDirs;
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, MutexGuard, OnceLock},
};

use tao::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget};

use crate::{lock, log_if_err};

use self::{
    api::register_api_instances,
    api_manager::ApiManager,
    event_handler::EventHandler,
    http_server::{HttpServerSlot, NivaHttpServer, app_server},
    options::NivaOptions,
    resource_manager::{AppResourceManager, FileSystemResource, ResourceManager},
    shortcut_manager::NivaShortcutManager,
    tray_manager::NivaTrayManager,
    utils::{ArcMut, arc_mut, merge_values},
    window_manager::{WindowManager, options::NivaWindowOptions},
};

pub type NivaEventLoop = EventLoop<NivaEvent>;
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
    _api: Arc<ApiManager>,
    _shortcut: ArcMut<NivaShortcutManager>,
    _tray: ArcMut<NivaTrayManager>,
    _http: HttpServerSlot, // Async HTTP + WebSocket server (populated post-Arc).
    _stdio: OnceLock<stdio::StdioBridge>,
    _custom_protocol: custom_protocol::CustomProtocolDispatcher,

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
        if let Some(macos_extra) = &launch_info.options.macos_extra {
            use self::options::NivaActivationPolicy;
            use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};

            if let Some(p) = macos_extra.activation_policy.clone() {
                let policy = match p {
                    NivaActivationPolicy::Regular => ActivationPolicy::Regular,
                    NivaActivationPolicy::Accessory => ActivationPolicy::Accessory,
                    NivaActivationPolicy::Prohibited => ActivationPolicy::Prohibited,
                };
                event_loop.set_activation_policy(policy);
            }

            // NOTE: tao 0.37 removed default menu creation (menu now via muda).
            // The `default_menu_creation` option is currently a no-op.
            let _ = macos_extra.default_menu_creation;

            if let Some(ignore) = macos_extra.activate_ignoring_other_apps {
                event_loop.set_activate_ignoring_other_apps(ignore);
            }
        };

        // create api manager and register api instances
        let mut api_manager = ApiManager::new(&launch_info.options);
        register_api_instances(&mut api_manager);
        let api_manager = Arc::new(api_manager);

        let window_manager = WindowManager::new(&launch_info);

        // build shortcuts (global-hotkey manager is not tied to the event loop)
        let shortcut_manager = NivaShortcutManager::new();

        let tray_manager = NivaTrayManager::new();
        let custom_protocol = custom_protocol::CustomProtocolDispatcher::new()?;

        let app = Arc::new(NivaApp {
            launch_info,

            _resource: resource_manager,
            _window: window_manager.clone(),
            _api: api_manager.clone(),
            _shortcut: shortcut_manager,
            _tray: tray_manager.clone(),
            _http: arc_mut(None),
            _stdio: OnceLock::new(),
            _custom_protocol: custom_protocol,

            event_loop_proxy: event_loop.create_proxy(),
        });

        if app.launch_info.arguments.stdio {
            let bridge = stdio::StdioBridge::start(Arc::downgrade(&app))?;
            app._stdio
                .set(bridge)
                .map_err(|_| anyhow!("stdio bridge already initialized"))?;
        }

        // bind app to window manager
        lock!(window_manager)?.bind_app(app.clone());
        api_manager.bind_app(app.clone());
        lock!(tray_manager)?.bind_app(app.clone());

        // start the loopback HTTP + WebSocket server before any window exists
        let server = NivaHttpServer::start(&app)?;
        eprintln!("[niva] http server listening on {}", server.base_url());
        *lock!(app._http)? = Some(server);

        // forward menu / tray / hotkey events (outside tao) to windows
        EventHandler::install_external_handlers(app.clone());

        Ok(app)
    }

    pub fn http_slot(self: &Arc<Self>) -> HttpServerSlot {
        self._http.clone()
    }

    pub(crate) fn stdio_bridge(&self) -> Option<&stdio::StdioBridge> {
        self._stdio.get()
    }

    /// Close the main window through the normal owner cleanup path, then
    /// terminate the event loop. Called when either side of the stdio pipe
    /// reaches EOF or can no longer write.
    pub(crate) fn request_stdio_exit(self: &Arc<Self>) {
        let app = self.clone();
        let event = NivaEvent::new(move |_, control_flow| {
            let close_result = (|| {
                let main_window = app.window()?.get_window(0)?;
                let closed = app.window()?.close_window_inner(main_window.window_id)?;
                WindowManager::cleanup_window(&app, &closed)
            })();
            *control_flow = ControlFlow::Exit;
            close_result
        });
        if self.event_loop_proxy.send_event(event).is_err() {
            eprintln!("[niva] unable to request stdio shutdown: event loop is closed");
        }
    }

    /// Port of the loopback server; window credentials live on each window.
    pub fn server_info(self: &Arc<Self>) -> Result<u16> {
        app_server(self)
    }

    pub fn resource(self: &Arc<Self>) -> Arc<dyn ResourceManager> {
        self._resource.clone()
    }

    pub fn window(self: &Arc<Self>) -> Result<MutexGuard<'_, WindowManager>> {
        lock!(self._window)
    }

    /// Handler map is frozen after init: lock-free clone, dispatch never
    /// blocks on this manager. (Registration takes &mut during startup only.)
    pub fn api(self: &Arc<Self>) -> Arc<ApiManager> {
        self._api.clone()
    }

    pub fn shortcut(self: &Arc<Self>) -> Result<MutexGuard<'_, NivaShortcutManager>> {
        lock!(self._shortcut)
    }

    pub fn tray(self: &Arc<Self>) -> Result<MutexGuard<'_, NivaTrayManager>> {
        lock!(self._tray)
    }

    pub fn run(self: Arc<NivaApp>, event_loop: NivaEventLoop) -> Result<()> {
        // create niva main window to launch application.
        let main_window_options: &NivaWindowOptions = &self.clone().launch_info.options.window;
        let main_window = self
            .window()?
            .open_window(main_window_options, &event_loop)?;

        let shortcuts_options = &self.launch_info.options.shortcuts.clone();
        if let Some(options) = shortcuts_options {
            log_if_err!(
                self.shortcut()?
                    .register_with_options(main_window.id, options)
            );
        }

        let tray_options = &self.launch_info.options.tray.clone();
        if let Some(options) = tray_options {
            log_if_err!(self.tray()?.create(main_window.id, options, &event_loop));
        }

        let event_handler = EventHandler::new(self);
        event_loop.run(move |event, target, control_flow| {
            event_handler.handle(event, target, control_flow);
        });
    }
}

#[derive(Debug)]
pub struct NivaArguments {
    pub debug_devtools: bool,
    pub stdio: bool,
    pub debug_config: Option<PathBuf>,
    pub debug_resource: Option<PathBuf>,
    pub debug_entry: Option<String>,
    pub build_mode: bool,
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
        let stdio = args_map
            .get("stdio")
            .map(|v| v.is_empty() || v == "true")
            .unwrap_or(false);
        utils::set_stdio_mode(stdio);
        let debug_resource = args_map.get("debug-resource").map(PathBuf::from);
        let debug_entry = args_map.get("debug-entry").map(|v| v.to_string());
        let debug_config = args_map.get("debug-config").map(PathBuf::from);
        let build_mode = args_map.contains_key("build");

        Self {
            debug_devtools,
            stdio,
            debug_config,
            debug_resource,
            debug_entry,
            build_mode,
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
        let mut options = {
            let base_content = if let Some(debug_config) = &arguments.debug_config {
                std::fs::read(debug_config)?
            } else {
                let base_path = "niva.json";
                resource_manager.load(base_path)?
            };

            let base_options: Value = serde_json::from_slice(&base_content)?;

            let platform = std::env::consts::OS;
            let platform_options = base_options.get(platform).cloned();

            let options = if let Some(platform_options) = platform_options {
                merge_values(base_options, platform_options)
            } else {
                base_options
            };
            serde_json::from_value::<NivaOptions>(options)?
        };

        if arguments.debug_devtools {
            options.window.devtools = Some(true);
        }

        let name = options.name.clone();
        let uuid = options.uuid.clone();
        let id_name = format!("{}_{}", name.to_lowercase(), &uuid[0..8]);

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
