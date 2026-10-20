use crate::{
    browser,
    model::{Browser, Selection},
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
pub fn clipboard_read() -> Result<String, String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .map_err(|_| "Windows clipboard is unavailable".into())
}
fn clipboard_write(value: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(value))
        .map_err(|_| "Windows clipboard is unavailable".into())
}
fn clear_if(expected: &str) {
    if let Ok(current) = clipboard_read() {
        if current == expected {
            let _ = clipboard_write(String::new());
        }
    }
}
pub fn handle(app: &AppHandle, owner: &str) -> Result<isize, String> {
    let state = app.state::<Browser>();
    let all = state.windows.lock().unwrap();
    let w = all.get(owner).ok_or("Browser window closed")?;
    let hwnd = w.vault_hwnd.unwrap_or(w.native_hwnd);
    if hwnd == 0 {
        return Err("Windows authentication could not attach to its window".into());
    }
    Ok(hwnd)
}
pub fn open(app: &AppHandle, owner: &str) -> Result<(), String> {
    if app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .ok_or("Browser window closed")?
        .private
    {
        return Err("Password Manager is unavailable in private windows".into());
    }
    let label = format!("vault-{owner}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_focus();
        return Ok(());
    }
    let app_event = app.clone();
    let name = owner.to_string();
    let window =
        WebviewWindowBuilder::new(app, &label, WebviewUrl::App("password-manager.html".into()))
            .additional_browser_args(if app.state::<Browser>().startup_gpu_disabled{"--disable-gpu --disable-gpu-compositing --disable-accelerated-video-decode --disable-accelerated-video-encode"}else{""})
            .use_https_scheme(true)
            .title("Aero Password Manager")
            .inner_size(840., 700.)
            .min_inner_size(620., 500.)
            .skip_taskbar(true)
            .on_navigation(|url| browser::trusted_url(url, "password-manager.html"))
            .build()
            .map_err(|e| e.to_string())?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    if let Some(w) = app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get_mut(owner)
    {
        w.vault_hwnd = Some(hwnd);
    }
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            let state = app_event.state::<Browser>();
            if let Some(w) = state.windows.lock().unwrap().get_mut(&name) {
                w.vault_hwnd = None;
                w.epoch += 1;
                w.auth_busy = false;
                w.selected = None;
                if let Some(hwnd) = w.auth_hwnd.take() {
                    crate::password_vault::cancel(hwnd);
                }
            }
        }
    });
    Ok(())
}
pub fn lock(app: &AppHandle, owner: &str) {
    // Never acquire the window model mutex twice in one expression: Rust
    // temporaries can keep the first guard alive through an or_else callback.
    let hwnd = {
        let state = app.state::<Browser>();
        let all = state.windows.lock().unwrap();
        all.get(owner).map(|w| w.auth_hwnd.or(w.vault_hwnd).unwrap_or(w.native_hwnd))
    };
    let expected = {
        let state = app.state::<Browser>();
        let mut all = state.windows.lock().unwrap();
        let Some(w) = all.get_mut(owner) else { return };
        w.epoch += 1;
        w.auth_busy = false;
        w.auth_hwnd = None;
        w.selected = None;
        w.selection = None;
        w.pending = None;
        w.clipboard.take()
    };
    if let Some(hwnd) = hwnd {
        crate::password_vault::cancel(hwnd);
    }
    if let Some((text, _)) = expected {
        clear_if(&text);
    }
    if let Some(vault) = app.get_webview_window(&format!("vault-{owner}")) {
        let _ = vault.emit("password-manager:lock", ());
        let _ = vault.close();
    }
    let _ = app.emit_to(browser::shell_label(owner), "password-manager:locked", ());
}
async fn native_operation(
    app: &AppHandle,
    owner: &str,
    operation: &str,
    payload: Value,
    require_vault: bool,
) -> Result<Value, String> {
    if require_vault
        && app
            .state::<Browser>()
            .windows
            .lock()
            .unwrap()
            .get(owner)
            .and_then(|w| w.vault_hwnd)
            .is_none()
    {
        return Err("The vault window closed before this operation".into());
    }
    let hwnd = handle(app, owner)?;
    let epoch = {
        let state = app.state::<Browser>();
        let mut all = state.windows.lock().unwrap();
        let w = all.get_mut(owner).ok_or("Browser window closed")?;
        if w.private {
            return Err("Password Manager is unavailable in private windows".into());
        }
        if w.auth_busy {
            return Err("Another Windows authentication request is in progress".into());
        }
        w.auth_busy = true;
        w.auth_hwnd = Some(hwnd);
        w.last_vault = Instant::now();
        w.epoch
    };
    let native_epoch = crate::password_vault::epoch(hwnd)?;
    let backup = operation == "backup";
    let operation = operation.to_string();
    let input = zeroize::Zeroizing::new(payload.to_string());
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::password_vault::operation(&operation, &input, hwnd, native_epoch)
    })
    .await
    .map_err(|_| "The native vault operation failed")?;
    let valid = {
        let state = app.state::<Browser>();
        let mut all = state.windows.lock().unwrap();
        if let Some(w) = all.get_mut(owner) {
            if w.epoch == epoch {
                w.auth_busy = false;
                w.auth_hwnd = None;
                w.last_vault = Instant::now();
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if !valid {
        return Err("The vault locked or closed during authentication".into());
    }
    let text = result?;
    if backup {
        return Ok(Value::String(text));
    }
    serde_json::from_str(&text).map_err(|_| "The vault returned an invalid response".into())
}
async fn native(
    app: &AppHandle,
    owner: &str,
    operation: &str,
    payload: Value,
) -> Result<Value, String> {
    native_operation(app, owner, operation, payload, true).await
}
async fn native_shell(
    app: &AppHandle,
    owner: &str,
    operation: &str,
    payload: Value,
) -> Result<Value, String> {
    native_operation(app, owner, operation, payload, false).await
}
pub async fn request(
    app: &AppHandle,
    owner: &str,
    method: &str,
    payload: Value,
) -> Result<Value, String> {
    let state = app.state::<Browser>();
    if state
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .ok_or("Browser window closed")?
        .private
    {
        return Err("Password Manager is unavailable in private windows".into());
    }
    match method {
        "password-manager:availability" => {
            let available =
                tauri::async_runtime::spawn_blocking(crate::password_vault::availability)
                    .await
                    .map_err(|_| "Windows Hello status could not be checked")??;
            Ok(json!({"available":available}))
        }
        "password-manager:lock" => {
            lock(app, owner);
            Ok(json!(true))
        }
        "password-manager:activity" => {
            if let Some(w) = state.windows.lock().unwrap().get_mut(owner) {
                w.last_vault = Instant::now();
            }
            Ok(Value::Null)
        }
        "password-manager:list" => native(app, owner, "list", json!({})).await,
        "password-manager:save" => {
            if payload["password"]
                .as_str()
                .map(|s| s.is_empty() || s.len() > 8192)
                .unwrap_or(true)
                || payload["username"]
                    .as_str()
                    .map(|s| s.is_empty() || s.len() > 512)
                    .unwrap_or(true)
            {
                return Err("Username or password is empty or too long".into());
            }
            native(app, owner, "save", payload).await
        }
        "password-manager:delete" => {
            native(
                app,
                owner,
                "delete",
                json!({"id":payload.as_str().ok_or("Choose a credential")?}),
            )
            .await
        }
        "password-manager:select" => {
            let id = payload
                .as_str()
                .filter(|s| s.len() <= 128 && !s.is_empty())
                .ok_or("Invalid credential")?
                .to_string();
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .selected = Some((id, Instant::now()));
            Ok(json!(true))
        }
        "password-manager:reveal" | "password-manager:copy" => {
            let selected = state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .selected
                .take()
                .filter(|(_, at)| at.elapsed() < Duration::from_secs(60))
                .ok_or("Select a credential again")?;
            let mut credential = native(app, owner, "reveal", json!({"id":selected.0})).await?;
            if method.ends_with("copy") {
                let secret = zeroize::Zeroizing::new(
                    credential["password"]
                        .as_str()
                        .ok_or("Credential unavailable")?
                        .to_string(),
                );
                credential["password"] = Value::Null;
                clipboard_write(secret.to_string())?;
                state
                    .windows
                    .lock()
                    .unwrap()
                    .get_mut(owner)
                    .unwrap()
                    .clipboard = Some((secret, Instant::now()));
                Ok(json!(true))
            } else {
                Ok(credential)
            }
        }
        "password-manager:backup" => {
            let result = native(app, owner, "backup", json!({})).await?;
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("Aero-password-vault.aerovault")
                .add_filter("Encrypted Aero vault", &["aerovault"])
                .save_file()
                .await
            else {
                return Ok(json!(false));
            };
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(result.as_str().ok_or("Backup failed")?)
                .map_err(|_| "Backup failed")?;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(file.path())
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(&bytes)
                })
                .map_err(|_| "Choose a new backup filename; existing backups are preserved")?;
            Ok(json!(true))
        }
        "password-manager:restore" => {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Encrypted Aero vault", &["aerovault"])
                .pick_file()
                .await
            else {
                return Ok(json!(false));
            };
            if state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .and_then(|w| w.vault_hwnd)
                .is_none()
            {
                return Err("Vault closed during file selection".into());
            }
            let size = std::fs::metadata(file.path())
                .map_err(|_| "Backup is unavailable")?
                .len();
            if size > 16 * 1024 * 1024 {
                return Err("Backup is too large".into());
            }
            use base64::Engine;
            let bytes = file.read().await;
            native(
                app,
                owner,
                "restore",
                json!(base64::engine::general_purpose::STANDARD.encode(bytes)),
            )
            .await?;
            Ok(json!(true))
        }
        "password-manager:import-csv" => {
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("CSV password export", &["csv"])
                .pick_file()
                .await
            else {
                return Ok(json!({"imported":0,"skipped":0,"message":"Import cancelled."}));
            };
            if std::fs::metadata(file.path())
                .map_err(|_| "CSV unavailable")?
                .len()
                > 8 * 1024 * 1024
            {
                return Err("CSV exceeds 8 MiB".into());
            }
            let text = zeroize::Zeroizing::new(
                String::from_utf8(file.read().await).map_err(|_| "CSV must be UTF-8")?,
            );
            let mut csv = csv::Reader::from_reader(text.as_bytes());
            let headers = csv
                .headers()
                .map_err(|_| "CSV headers are invalid")?
                .clone();
            let index = |names: &[&str]| {
                headers
                    .iter()
                    .position(|v| names.contains(&v.trim().to_lowercase().as_str()))
            };
            let website = index(&["url", "origin", "website", "login_uri"])
                .ok_or("CSV has no website column")?;
            let user = index(&["username", "user", "login_username"])
                .ok_or("CSV has no username column")?;
            let password =
                index(&["password", "login_password"]).ok_or("CSV has no password column")?;
            let mut entries = vec![];
            let mut skipped = 0;
            for row in csv.records() {
                let row = row.map_err(|_| "CSV is malformed")?;
                if let Some(origin) = crate::model::origin(row.get(website).unwrap_or("")) {
                    let u = row.get(user).unwrap_or("");
                    let p = row.get(password).unwrap_or("");
                    if !u.is_empty() && !p.is_empty() && u.len() <= 512 && p.len() <= 8192 {
                        entries.push(json!({"origin":origin,"username":u,"password":p}));
                        continue;
                    }
                }
                skipped += 1;
            }
            let result = native(app, owner, "import", json!({"entries":entries})).await?;
            Ok(
                json!({"imported":result["imported"],"skipped":skipped,"message":"Passwords imported into the local encrypted vault."}),
            )
        }
        "password-manager:select-for-fill" => {
            let id = payload.as_str().ok_or("Select a credential")?;
            let tab = {
                state
                    .windows
                    .lock()
                    .unwrap()
                    .get(owner)
                    .unwrap()
                    .tab()
                    .cloned()
                    .ok_or("No active page")?
            };
            let origin = crate::model::origin(&tab.url).ok_or("Select an HTTPS page first")?;
            let metadata = native(app, owner, "list", json!({})).await?;
            if !metadata
                .as_array()
                .ok_or("Vault unavailable")?
                .iter()
                .any(|v| v["id"] == id && v["origin"] == origin)
            {
                return Err("That credential does not belong to this HTTPS origin".into());
            }
            {
                let mut all = state.windows.lock().unwrap();
                let w = all.get_mut(owner).unwrap();
                if !w
                    .tab()
                    .map(|t| t.id == tab.id && t.url == tab.url)
                    .unwrap_or(false)
                {
                    return Err("The destination changed during authentication".into());
                }
                w.selection = Some(Selection {
                    id: id.into(),
                    tab: tab.id,
                    url: tab.url,
                    origin: origin.clone(),
                    at: Instant::now(),
                });
            }
            let _ = app.emit_to(
                browser::shell_label(owner),
                "password-manager:fill-ready",
                json!({"origin":origin}),
            );
            if let Some(v) = app.get_webview_window(&format!("vault-{owner}")) {
                let _ = v.close();
            }
            Ok(json!(true))
        }
        "password-manager:fill" => {
            let selected = {
                state
                    .windows
                    .lock()
                    .unwrap()
                    .get_mut(owner)
                    .unwrap()
                    .selection
                    .take()
                    .filter(|s| s.at.elapsed() < Duration::from_secs(30))
                    .ok_or("Select the credential again on the intended page")?
            };
            let view = browser::active_view(app, owner)?;
            if view.label() != browser::view_label(owner, selected.tab)
                || view.url().map_err(|_| "The tab closed")?.as_str() != selected.url
            {
                return Err("The destination changed before authentication".into());
            }
            let mut credential =
                native_shell(app, owner, "reveal", json!({"id":selected.id})).await?;
            let current = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .and_then(|w| w.tab())
                .cloned()
                .ok_or("The destination window closed during authentication")?;
            if current.id != selected.tab || current.url != selected.url {
                return Err(
                    "The active destination changed during authentication; nothing was filled"
                        .into(),
                );
            }
            if view.url().map_err(|_| "Tab closed")?.as_str() != selected.url
                || credential["origin"] != selected.origin
            {
                return Err(
                    "The destination changed during authentication; nothing was filled".into(),
                );
            }
            let input=zeroize::Zeroizing::new(json!({"url":selected.url,"origin":selected.origin,"username":credential["username"],"password":credential["password"]}).to_string());
            credential["password"] = Value::Null;
            let script = zeroize::Zeroizing::new(format!(
                "({})({})",
                include_str!("fill.js"),
                input.as_str()
            ));
            if crate::platform::isolated(&view, script.to_string()).await? != true {
                return Err(
                    "The approved visible HTTPS form was unavailable; nothing was filled".into(),
                );
            }
            Ok(json!(true))
        }
        "password-manager:save-submission" => {
            let pending = state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .pending
                .take()
                .filter(|p| p.at.elapsed() < Duration::from_secs(60))
                .ok_or("No explicit login submission is available")?;
            let current = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .and_then(|w| w.tab())
                .cloned()
                .ok_or("The submitted page closed")?;
            if current.id != pending.tab
                || crate::model::origin(&current.url).as_deref() != Some(pending.origin.as_str())
            {
                return Err("The submitted site changed; nothing was saved".into());
            }
            if rfd::AsyncMessageDialog::new()
                .set_title("Save or update password in Aero?")
                .set_description(format!(
                    "Save the submitted sign-in for {} in the encrypted local vault?",
                    pending.origin
                ))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                .await
                != rfd::MessageDialogResult::Yes
            {
                return Ok(json!(false));
            }
            let result=native(app,owner,"save",json!({"origin":pending.origin,"username":pending.username,"password":pending.password.as_str()})).await?;
            Ok(result)
        }
        "password-manager:discard-submission" => {
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .pending = None;
            Ok(json!(true))
        }
        _ => Err("Unknown password manager operation".into()),
    }
}
pub fn tick(app: &AppHandle) {
    let state = app.state::<Browser>();
    let mut expired = vec![];
    let mut clear = vec![];
    {
        let mut all = state.windows.lock().unwrap();
        for (owner, w) in all.iter_mut() {
            if w.last_vault.elapsed() > Duration::from_secs(300)
                && (w.auth_busy || w.vault_hwnd.is_some())
            {
                expired.push(owner.clone());
            }
            if w.clipboard
                .as_ref()
                .map(|(_, at)| at.elapsed() > Duration::from_secs(30))
                .unwrap_or(false)
            {
                if let Some((text, _)) = w.clipboard.take() {
                    clear.push(text);
                }
            }
            if w.pending
                .as_ref()
                .map(|p| p.at.elapsed() > Duration::from_secs(60))
                .unwrap_or(false)
            {
                w.pending = None;
            }
        }
    }
    for owner in expired {
        lock(app, &owner)
    }
    for text in clear {
        clear_if(&text);
    }
}

pub fn revoke(app: &AppHandle, owner: &str) {
    let state = app.state::<Browser>();
    let mut all = state.windows.lock().unwrap();
    if let Some(w) = all.get_mut(owner) {
        w.epoch += 1;
        w.auth_busy = false;
        w.selected = None;
        w.selection = None;
        w.pending = None;
        if let Some(hwnd) = w.auth_hwnd.take() {
            crate::password_vault::cancel(hwnd);
        }
    }
}
pub fn closed(app: &AppHandle, owner: &str) {
    let removed = app.state::<Browser>().windows.lock().unwrap().remove(owner);
    if let Some(mut w) = removed {
        if let Some(hwnd) = w.auth_hwnd.take() {
            crate::password_vault::cancel(hwnd);
        }
        let a = app.clone();
        let owner = owner.to_string();
        tauri::async_runtime::spawn(async move {
            if let Some((text, _)) = w.clipboard.take() {
                clear_if(&text);
            }
            if let Some(label) = w.suggestion_label.take() {
                if let Some(popup) = a.get_webview_window(&label) {
                    let _ = popup.close();
                }
            }
            if let Some(vault) = a.get_webview_window(&format!("vault-{owner}")) {
                let _ = vault.close();
            }
        });
    }
}
