mod api;
mod api_manager;
pub(crate) mod custom_protocol;
mod event_handler;
pub(crate) mod fs_ops;
pub(crate) mod http_server;
pub(crate) mod logging;
pub(crate) mod main_exec;
mod menu;
mod node_bootstrap;
mod node_compat;
mod options;
pub(crate) mod os_native;
mod resource_manager;
mod shortcut_manager;
mod tray_manager;
mod utils;
mod window_manager;
pub(crate) mod windows_bootstrap;

use anyhow::{Result, anyhow, bail};
use directories::BaseDirs;
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::{cell::RefCell, collections::HashMap};
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, MutexGuard},
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

#[cfg(target_os = "windows")]
thread_local! {
    /// Menu accelerator tables are UI-thread-owned by Muda. Keep only their
    /// raw handles here so the Tao message hook never has to lock the window
    /// manager or a menu while TranslateAcceleratorW synchronously dispatches.
    static WINDOW_MENU_ACCELERATORS: RefCell<HashMap<isize, isize>> = RefCell::new(HashMap::new());
}

#[cfg(target_os = "windows")]
pub(crate) fn set_window_menu_accelerator(hwnd: isize, haccel: Option<isize>) {
    WINDOW_MENU_ACCELERATORS.with(|accelerators| {
        let mut accelerators = accelerators.borrow_mut();
        match haccel.filter(|haccel| *haccel != 0) {
            Some(haccel) => {
                accelerators.insert(hwnd, haccel);
            }
            None => {
                accelerators.remove(&hwnd);
            }
        }
    });
}

#[cfg(target_os = "windows")]
pub(crate) fn remove_window_menu_accelerator(hwnd: isize) {
    set_window_menu_accelerator(hwnd, None);
}

/// Translate a Windows menu accelerator before Tao dispatches the message.
/// The hook copies the HACCEL out of thread-local state before calling the
/// Win32 API because TranslateAcceleratorW synchronously invokes the target
/// window procedure, which can re-enter Niva's event handling.
#[cfg(target_os = "windows")]
pub(crate) fn translate_menu_accelerator(message: *const std::ffi::c_void) -> bool {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, HACCEL, MSG, TranslateAcceleratorW},
    };

    if message.is_null() {
        return false;
    }

    // Tao supplies a pointer to the MSG currently being dispatched.
    let message = unsafe { &*message.cast::<MSG>() };
    if message.hwnd.0.is_null() {
        return false;
    }

    // Keyboard messages can target the focused WebView child HWND. Muda's
    // accelerator table belongs to the top-level window, so resolve its root.
    let target = unsafe { GetAncestor(message.hwnd, GA_ROOT) };
    if target.0.is_null() {
        return false;
    }
    let target_id = target.0 as isize;
    let haccel = WINDOW_MENU_ACCELERATORS
        .with(|accelerators| accelerators.borrow().get(&target_id).copied());
    let Some(haccel) = haccel else {
        return false;
    };

    // The copied HACCEL remains valid while its Muda menu is retained by the
    // NivaWindow; menu attach/detach and this callback all run on the UI thread.
    unsafe { TranslateAcceleratorW(HWND(target.0), HACCEL(haccel as _), message) != 0 }
}

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
    _custom_protocol: custom_protocol::CustomProtocolDispatcher,

    event_loop_proxy: EventLoopProxy<NivaEvent>, // Event loop proxy.
}

impl NivaApp {
    pub fn new(event_loop: &mut NivaEventLoop) -> Result<Arc<NivaApp>> {
        let arguments = NivaArguments::new()?;

        let resource_manager: Arc<dyn ResourceManager> = match &arguments.resource {
            Some(dir) => FileSystemResource::new(dir)?,
            None => AppResourceManager::new()?,
        };

        let launch_info = NivaLaunchInfo::new(arguments, resource_manager.clone())?;
        let log_path = logging::init(&launch_info.data_dir)?;
        crate::log!(format!("Niva log file: {}", log_path.display()));

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
            _custom_protocol: custom_protocol,

            event_loop_proxy: event_loop.create_proxy(),
        });

        // bind app to window manager
        lock!(window_manager)?.bind_app(app.clone());
        api_manager.bind_app(app.clone());
        lock!(tray_manager)?.bind_app(app.clone());

        // start the loopback HTTP + WebSocket server before any window exists
        let server = NivaHttpServer::start(&app)?;
        crate::log!(format!("HTTP server listening on {}", server.base_url()));
        *lock!(app._http)? = Some(server);

        // forward menu / tray / hotkey events (outside tao) to windows
        EventHandler::install_external_handlers(app.clone());

        Ok(app)
    }

    pub fn http_slot(self: &Arc<Self>) -> HttpServerSlot {
        self._http.clone()
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
        // WebView2 is checked before the first WebView is created. This may
        // show a native prompt and invoke Microsoft's signed online bootstrapper.
        windows_bootstrap::ensure_runtime()?;

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
    pub config: Option<PathBuf>,
    pub resource: Option<PathBuf>,
    pub debug_entry: Option<String>,
    pub build_mode: bool,
}

impl NivaArguments {
    pub fn new() -> Result<Self> {
        Self::parse(std::env::args().skip(1))
    }

    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut arguments = arguments.into_iter().peekable();
        let mut parsed = Self {
            debug_devtools: false,
            config: None,
            resource: None,
            debug_entry: None,
            build_mode: false,
        };

        while let Some(argument) = arguments.next() {
            let Some(option) = argument.strip_prefix("--") else {
                continue;
            };
            let (name, inline_value) = option
                .split_once('=')
                .map_or((option, None), |(name, value)| (name, Some(value)));

            let mut required_value = || -> Result<String> {
                let value = match inline_value {
                    Some(value) => value.to_owned(),
                    None => arguments
                        .next()
                        .filter(|value| !value.starts_with("--"))
                        .ok_or_else(|| anyhow!("--{name} requires a value"))?,
                };
                anyhow::ensure!(!value.is_empty(), "--{name} requires a value");
                Ok(value)
            };

            match name {
                "config" => parsed.config = Some(PathBuf::from(required_value()?)),
                "resource" => parsed.resource = Some(PathBuf::from(required_value()?)),
                "debug-entry" => parsed.debug_entry = Some(required_value()?),
                "debug-devtools" => {
                    parsed.debug_devtools = match inline_value {
                        None | Some("true") => true,
                        Some("false") => false,
                        Some(value) => bail!("invalid --debug-devtools value {value:?}"),
                    };
                }
                "build" => parsed.build_mode = true,
                "debug-resource" | "debug-config" => {
                    bail!("--{name} was removed; use --resource or --config")
                }
                "stdio" => bail!("--stdio was removed; use the main-window process streams"),
                _ => {}
            }
        }

        Ok(parsed)
    }

    /// A debug page is used only when explicitly named on the command line,
    /// or when Devtools debug mode explicitly authorizes `debug.entry` from
    /// the selected config file. `--config` by itself is only a data source.
    pub(crate) fn explicit_debug_entry(&self, options: &NivaOptions) -> Option<String> {
        if self.build_mode {
            return None;
        }
        self.debug_entry.clone().or_else(|| {
            self.debug_devtools
                .then(|| options.debug.as_ref().and_then(|debug| debug.entry.clone()))
                .flatten()
                .filter(|entry| !entry.is_empty())
        })
    }

    pub(crate) fn has_explicit_debug_entry(&self, options: &NivaOptions) -> bool {
        self.explicit_debug_entry(options).is_some()
    }
}

#[derive(Debug)]
pub struct NivaLaunchInfo {
    pub name: String,         // Name of the project.
    pub uuid: String,         // UUID of the project.
    pub data_dir: PathBuf,    // Data directory of the project, keyed only by its full UUID.
    pub cache_dir: PathBuf,   // Cache directory of the project.
    pub temp_dir: PathBuf,    // This is where temporary files are stored such as
    pub options: NivaOptions, // Project options, read from niva.json.
    pub arguments: NivaArguments,
}

impl NivaLaunchInfo {
    pub fn new(
        arguments: NivaArguments,
        resource_manager: Arc<dyn ResourceManager>,
    ) -> Result<NivaLaunchInfo> {
        let mut options = {
            let base_content = if let Some(config) = &arguments.config {
                std::fs::read(config)?
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
        let uuid = canonical_uuid(&options.uuid)?;
        options.uuid = uuid.clone();

        let base_dirs = BaseDirs::new().ok_or(anyhow!("Failed to get user directories"))?;
        let temp_dir = std::env::temp_dir().join(&uuid);
        let data_dir = base_dirs.data_dir().join(&uuid);
        let cache_dir = base_dirs.cache_dir().join(&uuid);

        Ok(NivaLaunchInfo {
            name,
            uuid,
            data_dir,
            cache_dir,
            temp_dir,
            options,
            arguments,
        })
    }
}

fn canonical_uuid(uuid: &str) -> Result<String> {
    let bytes = uuid.as_bytes();
    anyhow::ensure!(bytes.len() == 36, "niva.json uuid must be a canonical UUID");
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            anyhow::ensure!(*byte == b'-', "niva.json uuid must be a canonical UUID");
        } else {
            anyhow::ensure!(
                byte.is_ascii_hexdigit(),
                "niva.json uuid must be a canonical UUID"
            );
        }
    }
    Ok(uuid.to_ascii_lowercase())
}

#[cfg(test)]
mod startup_tests {
    use super::{NivaArguments, canonical_uuid};
    use crate::app::options::NivaOptions;
    use serde_json::json;

    #[test]
    fn startup_arguments_accept_new_sources_and_reject_removed_aliases() {
        let args = NivaArguments::parse([
            "--config".into(),
            "project.json".into(),
            "--resource=dist".into(),
            "--debug-entry=http://localhost:3000".into(),
        ])
        .unwrap();
        assert_eq!(args.config.unwrap().to_string_lossy(), "project.json");
        assert_eq!(args.resource.unwrap().to_string_lossy(), "dist");
        assert_eq!(args.debug_entry.as_deref(), Some("http://localhost:3000"));
        assert!(NivaArguments::parse(["--debug-config=project.json".into()]).is_err());
        assert!(NivaArguments::parse(["--debug-resource=dist".into()]).is_err());
        assert!(NivaArguments::parse(["--stdio".into()]).is_err());
    }

    #[test]
    fn config_source_does_not_authorize_debug_entry() {
        let args = NivaArguments::parse(["--config=project.json".into()]).unwrap();
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "app",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51",
            "debug": { "entry": "http://localhost:3000" }
        }))
        .unwrap();
        assert!(args.explicit_debug_entry(&options).is_none());

        let devtools = NivaArguments::parse(["--debug-devtools=true".into()]).unwrap();
        assert_eq!(
            devtools.explicit_debug_entry(&options).as_deref(),
            Some("http://localhost:3000")
        );
    }

    #[test]
    fn uuid_is_validated_and_canonicalized_before_path_use() {
        assert_eq!(
            canonical_uuid("A51C1728-D174-42D4-8F57-7D296C966B51").unwrap(),
            "a51c1728-d174-42d4-8f57-7d296c966b51"
        );
        for invalid in ["short", "../../x", "zzzzzzzz-d174-42d4-8f57-7d296c966b51"] {
            assert!(canonical_uuid(invalid).is_err(), "{invalid}");
        }
    }
}
