use serde_json::{Value, json};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, Webview};
pub async fn observe(app: &AppHandle, owner: &str, id: u32, view: &Webview) -> Result<(), String> {
    let private = app
        .state::<crate::model::Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .ok_or("Window closed")?
        .private;
    if private {
        return Ok(());
    }
    let tree = crate::platform::cdp(view, "Page.getFrameTree", json!({})).await?;
    let frame = tree["frameTree"]["frame"]["id"]
        .as_str()
        .ok_or("Main frame unavailable")?;
    let context = crate::platform::cdp(
        view,
        "Page.createIsolatedWorld",
        json!({"frameId":frame,"worldName":"AeroSecureWorld","grantUniveralAccess":false}),
    )
    .await?;
    let context_id = context["executionContextId"]
        .as_i64()
        .ok_or("Secure context unavailable")?;
    {
        let state = app.state::<crate::model::Browser>();
        if let Some(w) = state.windows.lock().unwrap().get_mut(owner) {
            if let Some(tab) = w.tabs.iter_mut().find(|t| t.id == id) {
                tab.secure_context = Some(context_id);
            }
        }
    }
    crate::platform::cdp(
        view,
        "Runtime.addBinding",
        json!({"name":"__AeroLoginSubmission","executionContextId":context_id}),
    )
    .await?;
    crate::platform::cdp(view,"Runtime.evaluate",json!({"expression":include_str!("submission.js"),"contextId":context_id,"returnByValue":true})).await?;
    Ok(())
}
pub fn receive(app: &AppHandle, owner: &str, id: u32, message: &str) {
    let Ok(event) = serde_json::from_str::<Value>(message) else {
        return;
    };
    if event["name"] != "__AeroLoginSubmission" {
        return;
    }
    let Some(context) = event["executionContextId"].as_i64() else {
        return;
    };
    let Some(payload) = event["payload"].as_str().filter(|s| s.len() <= 16000) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return;
    };
    let Some(username) = value["username"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 512)
    else {
        return;
    };
    let Some(password) = value["password"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 8192)
    else {
        return;
    };
    let Some(origin) = value["origin"].as_str() else {
        return;
    };
    let valid = {
        let state = app.state::<crate::model::Browser>();
        let mut all = state.windows.lock().unwrap();
        let Some(w) = all.get_mut(owner) else { return };
        if w.private || w.active != id {
            return;
        }
        let Some(tab) = w
            .tabs
            .iter()
            .find(|t| t.id == id && t.secure_context == Some(context))
        else {
            return;
        };
        if crate::model::origin(&tab.url).as_deref() != Some(origin) {
            return;
        }
        w.pending = Some(crate::model::Pending {
            tab: id,
            origin: origin.into(),
            username: username.into(),
            password: zeroize::Zeroizing::new(password.into()),
            at: Instant::now(),
        });
        true
    };
    if valid {
        let a = app.clone();
        let destination = crate::browser::shell_label(owner);
        let offer = json!({"origin":origin,"username":username});
        tauri::async_runtime::spawn(async move {
            let _ = a.emit_to(destination, "password-manager:save-offer", offer);
        });
    }
}
