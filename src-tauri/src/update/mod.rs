pub mod core;
use core::{Config, Record, Store};
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tauri::Manager;

const VERSION: &str = env!("CARGO_PKG_VERSION");
static STATUS: OnceLock<Mutex<Value>> = OnceLock::new();
fn config() -> Result<Config, String> {
    serde_json::from_str(include_str!("../../updater.json"))
        .map_err(|_| "Update configuration is invalid".into())
}
fn status_value(state: &str, message: &str, next: Option<&str>) -> Value {
    json!({"version":VERSION,"state":state,"message":message,"nextVersion":next})
}
fn set_status(state: &str, message: &str, next: Option<&str>) {
    if let Ok(mut s) = STATUS
        .get_or_init(|| Mutex::new(status_value("idle", "Automatic updates are on", None)))
        .lock()
    {
        *s = status_value(state, message, next);
    }
}
pub fn status() -> Value {
    STATUS
        .get_or_init(|| Mutex::new(status_value("idle", "Automatic updates are on", None)))
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| {
            status_value(
                "unavailable",
                "Updates will be checked on the next start",
                None,
            )
        })
}
fn disabled() -> bool {
    cfg!(debug_assertions) || std::env::var_os("AERO_RUST_TEST_PROFILE").is_some()
}

// A cached payload inherits the original launch path, so portable data never moves into its cache.
pub fn launch_executable() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|_| "Application path is unavailable")?
        .canonicalize()
        .map_err(|_| "Application path is unavailable")?;
    if let Some(root) = std::env::var_os("AERO_LAUNCHER_PATH") {
        if let Ok(root) = PathBuf::from(root).canonicalize() {
            if let Some(parent) = root.parent() {
                let versions = parent.join(".aero-updates").join("versions");
                if exe.starts_with(&versions)
                    && exe.file_name().is_some_and(|s| s == "Aero.exe")
                    && exe.parent().and_then(Path::parent) == Some(versions.as_path())
                    && core::ordinary(&root, false)
                {
                    return Ok(root);
                }
            }
        }
    }
    Ok(exe)
}
fn store() -> Result<Store, String> {
    let exe = launch_executable()?;
    Store::new(
        exe.parent().ok_or("Application directory is unavailable")?,
        config()?,
    )
}
// No network requests here: startup selects an already verified, fully downloaded payload.
pub fn bootstrap() -> bool {
    if disabled() {
        return false;
    }
    let Ok(store) = store() else { return false };
    let Ok(original) = launch_executable() else {
        return false;
    };
    if let Ok(current) = std::env::current_exe().and_then(|p| p.canonicalize()) {
        if current != original
            && current
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                != Some(VERSION)
        {
            return true;
        }
    }
    store.launch_cached(
        VERSION,
        &original,
        &std::env::args_os().skip(1).collect::<Vec<_>>(),
    )
}
pub fn healthy() {
    if disabled() {
        return;
    }
    let Ok(store) = store() else { return };
    if let Ok(nonce) = std::env::var("AERO_UPDATE_HEALTH") {
        if nonce.len() == 32 && nonce.bytes().all(|b| b.is_ascii_hexdigit()) {
            let _ = core::atomic(
                &store.root.join(format!("health-{nonce}")),
                VERSION.as_bytes(),
            );
        }
    }
}
fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(format!("AeroUpdater/{VERSION}"))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            let host = url.host_str().unwrap_or("");
            if attempt.previous().len() < 5
                && url.scheme() == "https"
                && (host == "github.com" || host.ends_with(".githubusercontent.com"))
            {
                attempt.follow()
            } else {
                attempt.error("Untrusted update redirect")
            }
        }))
        .build()
        .map_err(|_| "Update connection could not be created".into())
}
fn fetch(client: &reqwest::blocking::Client, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("Cache-Control", "no-cache")
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| "The release service could not be reached")?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("Update metadata is too large".into());
    }
    let mut result = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut result)
        .map_err(|_| "The release response was interrupted")?;
    if result.len() as u64 > limit {
        return Err("Update metadata is too large".into());
    }
    Ok(result)
}
fn check() -> Result<(), String> {
    let store = store()?;
    let _lock = store.lock()?;
    store.clean(VERSION);
    let client = client()?;
    let base = format!(
        "https://github.com/{}/releases/latest/download",
        store.config.repository
    );
    let manifest = String::from_utf8(fetch(
        &client,
        &format!("{base}/latest.json"),
        core::MAX_MANIFEST,
    )?)
    .map_err(|_| "Invalid update metadata")?;
    let signature = String::from_utf8(fetch(&client, &format!("{base}/latest.json.sig"), 128)?)
        .map_err(|_| "Invalid update signature")?;
    let record = Record {
        manifest,
        signature,
    };
    let manifest = core::verify(&store.config, &record)?;
    if !store.should_stage(&record, VERSION)? {
        if let Some(r) = store.read_record("active.json") {
            let m = core::verify(&store.config, &r)?;
            if core::version(&m.version)? > core::version(VERSION)?
                && store.failed().as_deref() != Some(&m.version)
                && store.cached(&r).is_ok()
            {
                set_status(
                    "ready",
                    "Update ready. It will open automatically the next time you start Aero.",
                    Some(&m.version),
                );
                return Ok(());
            }
        }
        if store.failed().is_some() {
            set_status(
                "fallback",
                "Aero kept the working version after an update could not start. Future updates remain automatic.",
                None,
            );
        } else {
            set_status(
                "current",
                "Aero is up to date. Future updates install automatically.",
                None,
            );
        }
        return Ok(());
    }
    set_status(
        "downloading",
        "Downloading an update in the background. Keep browsing normally.",
        Some(&manifest.version),
    );
    let response = client
        .get(&manifest.url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| "The update could not be downloaded")?;
    if response
        .content_length()
        .is_some_and(|n| n != manifest.size)
    {
        return Err("Update size does not match the signed release".into());
    }
    if store.stage(&record, VERSION, response)? {
        set_status(
            "ready",
            "Update ready. It will open automatically the next time you start Aero.",
            Some(&manifest.version),
        );
    }
    Ok(())
}
pub fn start(app: &tauri::AppHandle) {
    if disabled() {
        set_status(
            "development",
            "Automatic updates are disabled in development and test builds",
            None,
        );
        return;
    }
    healthy();
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(10));
        loop {
            set_status(
                "checking",
                "Checking for updates quietly in the background",
                None,
            );
            let result = check();
            if result.is_err() {
                set_status(
                    "retry",
                    "Updates could not be checked. Aero will retry automatically; you can keep browsing.",
                    None,
                );
            }
            let _ = handle.emit_update_status();
            std::thread::sleep(if result.is_ok() {
                Duration::from_secs(6 * 60 * 60)
            } else {
                Duration::from_secs(15 * 60)
            });
        }
    });
}
trait EmitStatus {
    fn emit_update_status(&self) -> Result<(), String>;
}
impl EmitStatus for tauri::AppHandle {
    fn emit_update_status(&self) -> Result<(), String> {
        use tauri::Emitter;
        for owner in self
            .state::<crate::model::Browser>()
            .windows
            .lock()
            .unwrap()
            .keys()
        {
            let _ = self.emit_to(
                crate::browser::shell_label(owner),
                "browser:update-status",
                status(),
            );
        }
        Ok(())
    }
}
