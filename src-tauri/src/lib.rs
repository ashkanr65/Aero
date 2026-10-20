mod browsing_data;
mod browser;
mod filters;
mod model;
mod password_vault;
mod platform;
mod settings;
mod shields;
mod storage;
mod vault;
use model::Browser;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager, Webview};
fn caller(webview: &Webview) -> Result<(String, bool), String> {
    let label = webview.label();
    let (owner, is_vault) = if let Some(owner) = label.strip_prefix("shell-") {
        (owner, false)
    } else if let Some(owner) = label.strip_prefix("vault-") {
        (owner, true)
    } else {
        return Err("Webpages cannot access Aero browser or vault commands".into());
    };
    let url = webview.url().map_err(|_| "The caller closed")?;
    if !browser::trusted_url(
        &url,
        if is_vault {
            "password-manager.html"
        } else {
            "index.html"
        },
    ) {
        return Err("This command requires the isolated Aero interface".into());
    }
    Ok((owner.into(), is_vault))
}
#[tauri::command]
async fn aero_request(
    webview: Webview,
    app: AppHandle,
    method: String,
    payload: Value,
) -> Result<Value, String> {
    let (owner, is_vault) = caller(&webview)?;
    let state = app.state::<Browser>();
    if is_vault {
        if !method.starts_with("password-manager:") {
            return Err("The vault cannot control browser windows".into());
        }
        return vault::request(&app, &owner, &method, payload).await;
    }
    if method.starts_with("password-manager:") {
        if ![
            "password-manager:availability",
            "password-manager:fill",
            "password-manager:save-submission",
            "password-manager:discard-submission",
        ]
        .contains(&method.as_str())
        {
            return Err("Use the isolated password manager for this operation".into());
        }
        return vault::request(&app, &owner, &method, payload).await;
    }
    match method.as_str() {
        "browser:get-state" => {
            let s = state.settings.lock().unwrap().clone();
            Ok(state
                .windows
                .lock()
                .unwrap()
                .get(&owner)
                .ok_or("Browser window closed")?
                .snapshot(&s))
        }
        "browser:get-private-mode" => Ok(json!(
            state
                .windows
                .lock()
                .unwrap()
                .get(&owner)
                .ok_or("Browser window closed")?
                .private
        )),
        "browser:get-settings" => Ok(state.settings.lock().unwrap().clone()),
        "browser:get-downloads" => Ok(json!(browsing_data::downloads_for(&app, &owner))),
        "browser:clear-data" => browsing_data::clear(&app, &owner, payload).await,
        "browser:set-settings" => settings::save(&app, payload),
        "browser:reset-settings" => settings::save(&app, settings::defaults()),
        "browser:set-bounds" => {
            let bounds: model::Bounds =
                serde_json::from_value(payload).map_err(|_| "Invalid bounds")?;
            if ![bounds.x, bounds.y, bounds.width, bounds.height]
                .iter()
                .all(|n| n.is_finite() && *n >= 0. && *n < 20000.)
            {
                return Err("Invalid browser viewport".into());
            }
            if let Some(w) = state.windows.lock().unwrap().get_mut(&owner) {
                w.bounds = bounds;
            }
            browser::layout(&app, &owner);
            Ok(Value::Null)
        }
        "browser:get-shortcut-mapping" => {
            Ok(state.settings.lock().unwrap()["shortcuts"]["mapping"].clone())
        }
        "browser:set-shortcut-mapping" => settings::shortcuts(&app, payload, false),
        "browser:reset-shortcut-mapping" => settings::shortcuts(&app, Value::Null, true),
        "browser:choose-download-directory" => Ok(json!(
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|f| f.path().to_string_lossy().to_string())
                .unwrap_or_default()
        )),
        "browser:open-password-manager" => {
            vault::open(&app, &owner)?;
            Ok(json!(true))
        }
        "browser:command" => {
            browser::command(
                &app,
                &owner,
                payload["command"]
                    .as_str()
                    .ok_or("Invalid browser command")?,
                payload["payload"].clone(),
            )
            .await
        }
        "browser:import-from-source" => {
            if state.windows.lock().unwrap().get(&owner).is_none_or(|w|w.private) {
                return Err("Import browser data from a regular Aero window".into());
            }
            crate::imports::import(&app,payload).await
        },
        _ => Err("Unknown Aero operation".into()),
    }
}
mod imports;
mod import_profiles;
mod import_snapshot;
mod session_lock;
mod smoke;
mod submission;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![aero_request])
        .setup(|app| {
            let profile = if let Some(path) = std::env::var_os("AERO_RUST_TEST_PROFILE") {
                std::path::PathBuf::from(path)
            } else {
                let executable = std::env::current_exe()?;
                if executable
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.contains("Portable"))
                    .unwrap_or(false)
                {
                    executable
                        .parent()
                        .ok_or("Executable directory unavailable")?
                        .join("AeroData")
                } else {
                    app.path().app_local_data_dir()?
                }
            };
            std::fs::create_dir_all(&profile)?;
            let loaded = std::fs::read(profile.join("settings.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .map(|v| settings::normalized(&v))
                .unwrap_or_else(settings::defaults);
            if smoke::enabled() {
                let mut value = loaded.clone();
                value["startup"]["homePage"] = json!("https://aero-synthetic.test/login");
                app.manage(Browser {
                    startup_gpu_disabled: loaded["appearance"]["disableGpu"] == true,
                    windows: std::sync::Mutex::new(std::collections::HashMap::new()),
                    settings: std::sync::Mutex::new(value),
                    profile: profile.clone(),
                    downloads: std::sync::Mutex::new(vec![]),
                });
            } else {
                app.manage(Browser {
                    startup_gpu_disabled: loaded["appearance"]["disableGpu"] == true,
                    windows: std::sync::Mutex::new(std::collections::HashMap::new()),
                    settings: std::sync::Mutex::new(loaded),
                    profile,
                    downloads: std::sync::Mutex::new(vec![]),
                });
            }
            browser::window(app.handle(), false).map_err(std::io::Error::other)?;
            if smoke::enabled() {
                smoke::start(app.handle().clone());
            }
            if !smoke::enabled() {
                filters::refresh(app.state::<Browser>().profile.clone());
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    vault::tick(&handle);
                    let state = handle.state::<Browser>();
                    let settings = state.settings.lock().unwrap().clone();
                    if settings["performance"]["sleepingTabs"] == true {
                        let minutes = settings["performance"]["sleepAfterMinutes"]
                            .as_u64()
                            .unwrap_or(15);
                        let sleeping = {
                            let all = state.windows.lock().unwrap();
                            all.iter()
                                .flat_map(|(owner, w)| {
                                    w.tabs
                                        .iter()
                                        .filter(|t| {
                                            !t.discarded
                                                && !t.audible
                                                && t.id != w.left
                                                && Some(t.id) != w.right
                                                && t.last_active
                                                    .map(|at| at.elapsed().as_secs() > minutes * 60)
                                                    .unwrap_or(false)
                                        })
                                        .map(|t| (owner.clone(), t.id))
                                        .collect::<Vec<_>>()
                                })
                                .collect::<Vec<_>>()
                        };
                        for (owner, id) in sleeping {
                            browser::discard(&handle, &owner, id);
                        }
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Aero could not start");
}
mod suggestions;
