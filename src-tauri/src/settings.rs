use crate::{browser, model::Browser};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
pub fn defaults() -> Value {
    serde_json::from_str(include_str!("default-settings.json")).expect("Bundled settings are valid")
}
pub fn normalized(input: &Value) -> Value {
    let mut settings = defaults();
    if let Some(root) = input.as_object() {
        for (section, value) in root {
            if let (Some(target), Some(value)) = (
                settings.get_mut(section).and_then(Value::as_object_mut),
                value.as_object(),
            ) {
                for (key, value) in value {
                    if let Some(default) = target.get(key) {
                        if default.is_boolean() && !value.is_boolean() {
                            continue;
                        }
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }
    let clamp = |v: &mut Value, low: u64, high: u64| {
        *v = json!(v.as_u64().unwrap_or(low).clamp(low, high));
    };
    clamp(&mut settings["layout"]["leftSidebarWidth"], 180, 400);
    clamp(&mut settings["layout"]["rightSidebarWidth"], 230, 500);
    clamp(&mut settings["accessibility"]["pageZoom"], 50, 200);
    settings
}
pub fn save(app: &AppHandle, value: Value) -> Result<Value, String> {
    let state = app.state::<Browser>();
    let next = normalized(&value);
    let previous = state.settings.lock().unwrap().clone();
    let bytes = serde_json::to_vec_pretty(&next).map_err(|_| "Invalid settings")?;
    std::fs::create_dir_all(&state.profile).map_err(|_| "Settings directory unavailable")?;
    let temp = state.profile.join("settings.pending.json");
    std::fs::write(&temp, bytes).map_err(|_| "Settings could not be saved")?;
    crate::storage::replace(&temp, &state.profile.join("settings.json"))?;
    *state.settings.lock().unwrap() = next.clone();
    let owners = state
        .windows
        .lock()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for owner in owners {
        let _ = app.emit_to(browser::shell_label(&owner), "browser:settings", &next);
        if next["appearance"]["disableGpu"] != previous["appearance"]["disableGpu"] {
            let _ = app.emit_to(
                browser::shell_label(&owner),
                "browser:gpu-setting-restart-required",
                &next["appearance"]["disableGpu"],
            );
        }
        let ids = state
            .windows
            .lock()
            .unwrap()
            .get(&owner)
            .map(|w| w.tabs.iter().map(|t| t.id).collect::<Vec<_>>())
            .unwrap_or_default();
        for id in ids {
            if let Some(view) = app.get_webview(&browser::view_label(&owner, id)) {
                crate::platform::configure(&view, next.clone());
            }
        }
        browser::layout(app, &owner);
        browser::emit(app, &owner);
    }
    Ok(next)
}
pub fn shortcuts(app: &AppHandle, mapping: Value, reset: bool) -> Result<Value, String> {
    let state = app.state::<Browser>();
    let mut value = state.settings.lock().unwrap().clone();
    value["shortcuts"]["mapping"] = if reset {
        defaults()["shortcuts"]["mapping"].clone()
    } else {
        mapping
    };
    let settings = save(app, value)?;
    let owners = state
        .windows
        .lock()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for owner in owners {
        let _ = app.emit_to(
            browser::shell_label(&owner),
            "browser:shortcut-mapping-changed",
            &settings["shortcuts"]["mapping"],
        );
    }
    Ok(settings["shortcuts"]["mapping"].clone())
}
