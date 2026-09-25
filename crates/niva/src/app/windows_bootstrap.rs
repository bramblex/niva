use anyhow::Result;

#[cfg(any(target_os = "windows", test))]
// `ipc_windows_frames::install` requires ICoreWebView2_4 and unconditionally
// casts every frame to ICoreWebView2Frame2. Frame2 was introduced in SDK
// 1.0.1108.44. Microsoft maps that SDK to Runtime 98 and documents the exact
// minimum as Runtime 98.0.1108.44; Frame7 is optional and does not raise it.
const MINIMUM_WEBVIEW2_VERSION: RuntimeVersion = RuntimeVersion([98, 0, 1108, 44]);
#[cfg(any(target_os = "windows", test))]
const MINIMUM_WEBVIEW2_VERSION_TEXT: &str = "98.0.1108.44";

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RuntimeVersion([u32; 4]);

#[cfg(any(target_os = "windows", test))]
impl RuntimeVersion {
    fn parse(value: &str) -> Option<Self> {
        let parts = value
            .split('.')
            .map(|part| part.parse::<u32>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .ok()?;
        let parts: [u32; 4] = parts.try_into().ok()?;
        Some(Self(parts))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn ensure_runtime() -> Result<()> {
    windows::ensure_runtime()
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn ensure_runtime() -> Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{
        ffi::OsStr,
        fs::OpenOptions,
        io::Write,
        os::windows::ffi::OsStrExt,
        path::PathBuf,
        process::{Command, Output},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use anyhow::{Context, Result, bail};
    use serde::Deserialize;
    use windows::{
        Win32::UI::WindowsAndMessaging::{
            IDYES, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_YESNO, MessageBoxW,
        },
        core::PCWSTR,
    };

    use super::{MINIMUM_WEBVIEW2_VERSION, MINIMUM_WEBVIEW2_VERSION_TEXT, RuntimeVersion};

    const INSTALLER_SCRIPT: &str = include_str!("../../assets/webview2-bootstrap.ps1");
    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeState {
        machine_version: Option<String>,
        user_version: Option<String>,
    }

    impl RuntimeState {
        fn machine(&self) -> Option<RuntimeVersion> {
            self.machine_version
                .as_deref()
                .and_then(RuntimeVersion::parse)
        }

        fn user(&self) -> Option<RuntimeVersion> {
            self.user_version.as_deref().and_then(RuntimeVersion::parse)
        }

        // WebView2 resolves a machine-wide runtime before a per-user runtime.
        fn effective(&self) -> Option<RuntimeVersion> {
            self.machine().or_else(|| self.user())
        }

        fn machine_upgrade_required(&self) -> bool {
            self.machine()
                .is_some_and(|version| version < MINIMUM_WEBVIEW2_VERSION)
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct InstallResult {
        status: String,
        exit_code: Option<i64>,
        error: Option<String>,
    }

    struct TemporaryScript(PathBuf);

    impl TemporaryScript {
        fn create() -> Result<Self> {
            let root = std::env::temp_dir();
            for _ in 0..16 {
                let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
                let nonce = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let path = root.join(format!(
                    "niva-webview2-{}-{nonce}-{sequence}.ps1",
                    std::process::id()
                ));
                match OpenOptions::new().write(true).create_new(true).open(&path) {
                    Ok(mut file) => {
                        file.write_all(INSTALLER_SCRIPT.as_bytes())?;
                        file.flush()?;
                        return Ok(Self(path));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(error).context("write WebView2 bootstrap script"),
                }
            }
            bail!("unable to create a unique WebView2 bootstrap script")
        }
    }

    impl Drop for TemporaryScript {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    pub(super) fn ensure_runtime() -> Result<()> {
        let script = TemporaryScript::create().map_err(|error| {
            let message = format!("Niva could not prepare its WebView2 Runtime check: {error:#}");
            show_error(&message);
            error
        })?;
        let installed = query_runtime(&script.0).map_err(|error| {
            let message = format!(
                "Niva could not check whether Microsoft Edge WebView2 Runtime {MINIMUM_WEBVIEW2_VERSION_TEXT} or later is installed. The Windows PowerShell check failed: {error:#}"
            );
            show_error(&message);
            error
        })?;
        if installed
            .effective()
            .is_some_and(|version| version >= MINIMUM_WEBVIEW2_VERSION)
        {
            crate::niva_log!(
                crate::app::logging::Level::Info,
                "WebView2 Runtime meets minimum version {MINIMUM_WEBVIEW2_VERSION_TEXT}"
            );
            return Ok(());
        }

        let machine_upgrade = installed.machine_upgrade_required();
        let prompt = if machine_upgrade {
            format!(
                "This app needs Microsoft Edge WebView2 Runtime {MINIMUM_WEBVIEW2_VERSION_TEXT} or later.\n\nThe machine-wide runtime is too old. Continue to download Microsoft's official installer and allow Windows to request administrator permission for that installer?"
            )
        } else {
            format!(
                "This app needs Microsoft Edge WebView2 Runtime {MINIMUM_WEBVIEW2_VERSION_TEXT} or later.\n\nContinue to download Microsoft's official installer and install it for the current user?"
            )
        };
        if !confirm_install(&prompt) {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "WebView2 installation declined by user"
            );
            bail!("WebView2 Runtime installation was declined")
        }

        let mut elevated = machine_upgrade;
        let mut result = install_with_feedback(&script.0, elevated)?;
        if result.status == "failed" && !elevated && is_access_denied(result.exit_code) {
            if !confirm_install(
                "The per-user WebView2 installation was denied. Retry Microsoft's installer with administrator permission? Only the installer process will be elevated.",
            ) {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "WebView2 administrator retry declined by user"
                );
                bail!("WebView2 Runtime installation requires administrator permission")
            }
            elevated = true;
            result = install_with_feedback(&script.0, elevated)?;
        }

        crate::niva_log!(
            crate::app::logging::Level::Info,
            "WebView2 installer returned status={} exitCode={:?} elevated={elevated}",
            result.status,
            result.exit_code
        );
        if let Some(error) = result.error.as_deref() {
            crate::niva_log!(crate::app::logging::Level::Error, "WebView2: {error}");
        }

        // Always check the registry again; a successful child exit alone does
        // not prove that the runtime is discoverable by the WebView2 loader.
        let after = query_runtime(&script.0).map_err(|error| {
            let message = format!(
                "Niva could not verify the Microsoft Edge WebView2 Runtime installation: {error:#}"
            );
            show_error(&message);
            error
        })?;
        if after
            .effective()
            .is_some_and(|version| version >= MINIMUM_WEBVIEW2_VERSION)
        {
            crate::niva_log!(
                crate::app::logging::Level::Info,
                "WebView2 Runtime installation verified"
            );
            return Ok(());
        }

        let message = format!(
            "Microsoft's installer finished with status '{}' and exit code {:?}, but WebView2 Runtime {MINIMUM_WEBVIEW2_VERSION_TEXT} or later was not found. Check your network or device policy, then retry.",
            result.status, result.exit_code
        );
        show_error(&message);
        bail!(message)
    }

    fn query_runtime(script: &std::path::Path) -> Result<RuntimeState> {
        let output = run_script(script, "check", false)?;
        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parse WebView2 check output: {}", clipped_output(&output)))
    }

    fn install_runtime(script: &std::path::Path, elevated: bool) -> Result<InstallResult> {
        let output = run_script(script, "install", elevated)?;
        serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "parse WebView2 installer output (status {:?}): {}",
                output.status.code(),
                clipped_output(&output)
            )
        })
    }

    fn install_with_feedback(script: &std::path::Path, elevated: bool) -> Result<InstallResult> {
        install_runtime(script, elevated).map_err(|error| {
            let message =
                format!("Niva could not run Microsoft's WebView2 Runtime installer: {error:#}");
            show_error(&message);
            error
        })
    }

    fn run_script(script: &std::path::Path, action: &str, elevated: bool) -> Result<Output> {
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
        command.arg(script).args(["-Action", action]);
        if elevated {
            command.arg("-Elevated");
        }
        let output = command
            .output()
            .context("start the Windows PowerShell WebView2 bootstrap script")?;
        if !output.status.success() {
            bail!(
                "WebView2 bootstrap script failed with status {:?}: {}",
                output.status.code(),
                clipped_output(&output)
            );
        }
        Ok(output)
    }

    fn is_access_denied(code: Option<i64>) -> bool {
        matches!(code, Some(5 | 2147942405 | -2147024891))
    }

    fn clipped_output(output: &Output) -> String {
        let mut text = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if text.is_empty() {
            text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        }
        if text.len() > 1024 {
            let mut end = 1024;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            text.push_str("…");
        }
        text
    }

    fn confirm_install(message: &str) -> bool {
        let caption = wide("Niva — WebView2 Runtime required");
        let message = wide(message);
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(message.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
            ) == IDYES
        }
    }

    fn show_error(message: &str) {
        let caption = wide("Niva could not start");
        let message = wide(message);
        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(message.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{MINIMUM_WEBVIEW2_VERSION, MINIMUM_WEBVIEW2_VERSION_TEXT, RuntimeVersion};

    #[test]
    fn runtime_floor_matches_the_unconditional_frame2_requirement() {
        let minimum = RuntimeVersion::parse(MINIMUM_WEBVIEW2_VERSION_TEXT).unwrap();
        assert_eq!(minimum, MINIMUM_WEBVIEW2_VERSION);
        assert!(RuntimeVersion::parse("98.0.1108.43").unwrap() < minimum);
        assert!(RuntimeVersion::parse("98.0.1108.45").unwrap() > minimum);
        assert!(RuntimeVersion::parse("153.0.4234.32").unwrap() > minimum);
    }

    #[test]
    fn rejects_non_four_part_or_non_numeric_versions() {
        for value in ["86.0.616", "86.x.616.0", "", "1.2.3.4.5"] {
            assert!(RuntimeVersion::parse(value).is_none(), "{value}");
        }
    }
}
