use serde_json::{Value, json};
use std::sync::mpsc;
use tauri::{AppHandle, Manager, Webview};
use webview2_com::DevToolsProtocolEventReceivedEventHandler;
use webview2_com::FaviconChangedEventHandler;
use webview2_com::{
    AcceleratorKeyPressedEventHandler, CallDevToolsProtocolMethodCompletedHandler,
    FocusChangedEventHandler, HistoryChangedEventHandler,
    IsDocumentPlayingAudioChangedEventHandler, Microsoft::Web::WebView2::Win32::*,
};
use windows::core::{BOOL, Interface, PCWSTR, PWSTR};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub async fn cdp(view: &Webview, method: &str, params: Value) -> Result<Value, String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let method = wide(method);
    let params = wide(&params.to_string());
    view.with_webview(move |native| {
        let result = (|| -> windows::core::Result<()> {
            let core = unsafe { native.controller().CoreWebView2()? };
            let handler =
                CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |hr, value| {
                    let result = hr
                        .map_err(|_| "The browser operation failed".to_string())
                        .and_then(|_| {
                            serde_json::from_str(&value)
                                .map_err(|_| "The browser response was invalid".into())
                        });
                    let _ = send.send(result);
                    Ok(())
                }));
            unsafe {
                core.CallDevToolsProtocolMethod(
                    PCWSTR(method.as_ptr()),
                    PCWSTR(params.as_ptr()),
                    &handler,
                )
            }
        })();
        if result.is_err() { /* A closed webview drops the callback; timeout fails closed. */ }
    })
    .map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(15), receive)
        .await
        .map_err(|_| "The browser operation timed out".to_string())?
        .map_err(|_| "The webpage closed during this operation".to_string())?
}
pub async fn isolated(view: &Webview, code: String) -> Result<Value, String> {
    let tree = cdp(view, "Page.getFrameTree", json!({})).await?;
    let frame = tree["frameTree"]["frame"]["id"]
        .as_str()
        .ok_or("The main frame is unavailable")?;
    let context = cdp(
        view,
        "Page.createIsolatedWorld",
        json!({"frameId":frame,"worldName":"AeroSecureWorld","grantUniveralAccess":false}),
    )
    .await?;
    let result=cdp(view,"Runtime.evaluate",json!({"expression":code,"contextId":context["executionContextId"],"returnByValue":true,"awaitPromise":true})).await?;
    if result.get("exceptionDetails").is_some() {
        return Err("The page did not support this operation".into());
    }
    Ok(result["result"]["value"].clone())
}
pub fn action(view: &Webview, action: &str, value: f64) -> Result<(), String> {
    let (send, receive) = mpsc::channel();
    let action = action.to_string();
    view.with_webview(move |native| {
        let result = (|| -> windows::core::Result<()> {
            let controller = native.controller();
            let core = unsafe { controller.CoreWebView2()? };
            unsafe {
                match action.as_str() {
                    "back" => core.GoBack(),
                    "forward" => core.GoForward(),
                    "reload" => core.Reload(),
                    "stop" => core.Stop(),
                    "mute" => core.cast::<ICoreWebView2_8>()?.SetIsMuted(value != 0.),
                    "zoom" => controller.SetZoomFactor(value),
                    "notify-position" => controller.NotifyParentWindowPositionChanged(),
                    _ => Ok(()),
                }
            }
        })();
        let _ = send.send(result.map_err(|_| "The native browser operation failed".to_string()));
    })
    .map_err(|e| e.to_string())?;
    receive
        .recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|_| "The native browser did not respond".to_string())?
}
pub fn install(view: &Webview, app: AppHandle, owner: String, id: u32) {
    let _ = view.with_webview(move |native| {
        let controller = native.controller();
        let Ok(core) = (unsafe { controller.CoreWebView2() }) else {
            return;
        };
        if let Ok(settings) = unsafe { core.Settings() } {
            if let Ok(settings) = settings.cast::<ICoreWebView2Settings4>() {
                unsafe {
                    let _ = settings.SetIsPasswordAutosaveEnabled(false);
                    let _ = settings.SetIsGeneralAutofillEnabled(false);
                }
            }
        }
        let mut token = 0i64;
        let binding_app = app.clone();
        let binding_owner = owner.clone();
        if id != 0 {
            if let Ok(receiver) = unsafe {
                core.GetDevToolsProtocolEventReceiver(windows::core::w!("Runtime.bindingCalled"))
            } {
                let handler =
                    DevToolsProtocolEventReceivedEventHandler::create(Box::new(move |_, args| {
                        if let Some(args) = args {
                            let mut raw = PWSTR::null();
                            unsafe {
                                args.ParameterObjectAsJson(&mut raw)?;
                            }
                            let message = zeroize::Zeroizing::new(unsafe { raw.to_string() }?);
                            unsafe {
                                let length =
                                    windows::Win32::Globalization::lstrlenW(PCWSTR(raw.0)) as usize;
                                std::slice::from_raw_parts_mut(raw.0, length).fill(0);
                                windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast()));
                            }
                            crate::submission::receive(&binding_app, &binding_owner, id, &message);
                        }
                        Ok(())
                    }));
                unsafe {
                    let _ = receiver.add_DevToolsProtocolEventReceived(&handler, &mut token);
                }
            }
        }

        if id != 0 {
            if let Ok(favicon) = core.cast::<ICoreWebView2_15>() {
                let a = app.clone();
                let o = owner.clone();
                let handler = FaviconChangedEventHandler::create(Box::new(move |sender, _| {
                    if let Some(sender) = sender {
                        if let Ok(core) = sender.cast::<ICoreWebView2_15>() {
                            let mut raw = PWSTR::null();
                            unsafe {
                                core.FaviconUri(&mut raw)?;
                            }
                            let uri = unsafe { raw.to_string() }?;
                            unsafe {
                                windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast()));
                            }
                            if let Some(w) = a
                                .state::<crate::model::Browser>()
                                .windows
                                .lock()
                                .unwrap()
                                .get_mut(&o)
                            {
                                if let Some(t) = w.tabs.iter_mut().find(|t| t.id == id) {
                                    t.favicon = if uri.is_empty() { None } else { Some(uri) };
                                }
                            }
                            crate::browser::emit(&a, &o);
                        }
                    }
                    Ok(())
                }));
                unsafe {
                    let _ = favicon.add_FaviconChanged(&handler, &mut token);
                }
            }
        }
        let app_history = app.clone();
        let window_history = owner.clone();
        let history = HistoryChangedEventHandler::create(Box::new(move |core, _| {
            if let Some(core) = core {
                let mut back = BOOL(0);
                let mut forward = BOOL(0);
                unsafe {
                    let _ = core.CanGoBack(&mut back);
                    let _ = core.CanGoForward(&mut forward);
                }
                let mut source = PWSTR::null();
                let current = if unsafe { core.Source(&mut source) }.is_ok() {
                    let result = unsafe { source.to_string() }.ok();
                    unsafe {
                        windows::Win32::System::Com::CoTaskMemFree(Some(source.0.cast()));
                    }
                    result
                } else {
                    None
                };
                let state = app_history.state::<crate::model::Browser>();
                if let Some(w) = state.windows.lock().unwrap().get_mut(&window_history) {
                    if let Some(t) = w.tabs.iter_mut().find(|t| t.id == id) {
                        t.back = back.as_bool();
                        t.forward = forward.as_bool();
                        if let Some(url) = current {
                            if t.url != url {
                                t.url = url;
                                w.selection = None;
                            }
                        }
                    }
                }
                crate::browser::emit(&app_history, &window_history);
            }
            Ok(())
        }));
        unsafe {
            let _ = core.add_HistoryChanged(&history, &mut token);
        }
        if let Ok(audio) = core.cast::<ICoreWebView2_8>() {
            let app_audio = app.clone();
            let window_audio = owner.clone();
            let handler =
                IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |core, _| {
                    if let Some(core) = core {
                        if let Ok(audio) = core.cast::<ICoreWebView2_8>() {
                            let mut playing = BOOL(0);
                            let mut muted = BOOL(0);
                            unsafe {
                                let _ = audio.IsDocumentPlayingAudio(&mut playing);
                                let _ = audio.IsMuted(&mut muted);
                            }
                            let state = app_audio.state::<crate::model::Browser>();
                            if let Some(w) = state.windows.lock().unwrap().get_mut(&window_audio) {
                                if let Some(t) = w.tabs.iter_mut().find(|t| t.id == id) {
                                    t.audible = playing.as_bool();
                                    t.muted = muted.as_bool();
                                }
                            }
                            crate::browser::emit(&app_audio, &window_audio);
                        }
                    }
                    Ok(())
                }));
            unsafe {
                let _ = audio.add_IsDocumentPlayingAudioChanged(&handler, &mut token);
            }
        }
        let app_focus = app.clone();
        let window_focus = owner.clone();
        let focus = FocusChangedEventHandler::create(Box::new(move |_, _| {
            let state = app_focus.state::<crate::model::Browser>();
            if let Some(w) = state.windows.lock().unwrap().get_mut(&window_focus) {
                if w.left == id || w.right == Some(id) {
                    w.active = id;
                    w.selection = None;
                }
            }
            crate::browser::emit(&app_focus, &window_focus);
            Ok(())
        }));
        unsafe {
            let _ = controller.add_GotFocus(&focus, &mut token);
        }
        let keyboard = AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let mut kind = COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN;
                let mut key = 0;
                unsafe {
                    args.KeyEventKind(&mut kind)?;
                    args.VirtualKey(&mut key)?;
                }
                if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                    || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN
                {
                    let ctrl =
                        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x11) }
                            < 0;
                    let shift =
                        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x10) }
                            < 0;
                    let alt =
                        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x12) }
                            < 0;
                    if let Some((command, payload)) =
                        crate::browser::mapped_key(&app, key, ctrl, shift, alt)
                    {
                        unsafe {
                            args.SetHandled(true)?;
                        }
                        let a = app.clone();
                        let w = owner.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = crate::browser::command(&a, &w, &command, payload).await;
                        });
                    }
                }
            }
            Ok(())
        }));
        unsafe {
            let _ = controller.add_AcceleratorKeyPressed(&keyboard, &mut token);
        }
    });
}

pub fn configure(view: &Webview, settings: Value) {
    update_spellcheck(view);
    let _ = view.with_webview(move |native| {
        let controller = native.controller();
        let Ok(core) = (unsafe { controller.CoreWebView2() }) else {
            return;
        };
        let zoom = settings["accessibility"]["pageZoom"]
            .as_f64()
            .unwrap_or(100.)
            / 100.;
        unsafe {
            let _ = controller.SetZoomFactor(zoom);
        }
        if let Ok(base) = unsafe { core.Settings() } {
            if let Ok(value) = base.cast::<ICoreWebView2Settings2>() {
                let mut raw = PWSTR::null();
                if unsafe { value.UserAgent(&mut raw) }.is_ok() {
                    let text = unsafe { raw.to_string() }.unwrap_or_default();
                    unsafe {
                        windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast()));
                    }
                    let mut user_agent = text
                        .split_whitespace()
                        .filter(|word| !word.starts_with("Aero/"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if settings["appearance"]["aeroIdentity"] == true {
                        user_agent.push_str(" Aero/1.0");
                    }
                    let bytes = wide(&user_agent);
                    unsafe {
                        let _ = value.SetUserAgent(PCWSTR(bytes.as_ptr()));
                    }
                }
            }
        }
        if let Ok(core) = core.cast::<ICoreWebView2_13>() {
            if let Ok(profile) = unsafe { core.Profile() } {
                if let Ok(profile) = profile.cast::<ICoreWebView2Profile3>() {
                    let level = if settings["privacy"]["trackingLevel"] == "off" {
                        COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_NONE
                    } else if settings["privacy"]["trackingLevel"] == "strict" {
                        COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT
                    } else {
                        COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED
                    };
                    unsafe {
                        let _ = profile.SetPreferredTrackingPreventionLevel(level);
                    }
                }
            }
        }
    });
}

pub fn efficiency(view: &Webview, low: bool) {
    let _ = view.with_webview(move |native| {
        if let Ok(core) = unsafe { native.controller().CoreWebView2() } {
            if let Ok(core) = core.cast::<ICoreWebView2_19>() {
                unsafe {
                    let _ = core.SetMemoryUsageTargetLevel(if low {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                    } else {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                    });
                }
            }
        }
    });
}

/// Wait for the Windows profile operation to finish before destroying its view.
pub async fn clear_data(view: &Webview) -> Result<(), String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let send = std::sync::Arc::new(std::sync::Mutex::new(Some(send)));
    view.with_webview(move |native| {
        let completed = send.clone();
        let result = (|| -> windows::core::Result<()> {
            let profile = unsafe { native.controller().CoreWebView2()?.cast::<ICoreWebView2_13>()?.Profile()?.cast::<ICoreWebView2Profile2>()? };
            let handler = webview2_com::ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
                if let Some(send) = completed.lock().unwrap().take() {
                    let _ = send.send(result.map_err(|_| "Windows could not clear browsing data".to_string()));
                }
                Ok(())
            }));
            unsafe { profile.ClearBrowsingDataAll(&handler) }
        })();
        if result.is_err() {
            if let Some(send) = send.lock().unwrap().take() { let _ = send.send(Err("Browsing data could not be cleared".into())); }
        }
    }).map_err(|e|e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(30), receive).await
        .map_err(|_|"Clearing browsing data timed out".to_string())?
        .map_err(|_|"Browser closed while clearing data".to_string())?
}

pub fn update_spellcheck(view: &Webview) {
    let view = view.clone();
    tauri::async_runtime::spawn(async move {
        let enabled = view.app_handle().state::<crate::model::Browser>().settings.lock().unwrap()["content"]["spellcheck"] != false;
        let code = format!("({})({enabled})", include_str!("spellcheck.js"));
        let _ = isolated(&view, code).await;
    });
}

pub async fn clear_selected(view: &Webview, request: &crate::browsing_data::ClearRequest) -> Result<(), String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let send = std::sync::Arc::new(std::sync::Mutex::new(Some(send)));
    let mask = request.mask();
    let all_time = request.time_range == crate::browsing_data::TimeRange::AllTime;
    let start = request.start_time as f64 / 1000.;
    let end = request.end_time as f64 / 1000.;
    view.with_webview(move |native| {
        let completed = send.clone();
        let result = (|| -> windows::core::Result<()> {
            let profile = unsafe { native.controller().CoreWebView2()?.cast::<ICoreWebView2_13>()?.Profile()?.cast::<ICoreWebView2Profile2>()? };
            let handler = webview2_com::ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
                if let Some(send) = completed.lock().unwrap().take() { let _ = send.send(result.map_err(|_| "Windows could not clear the selected browsing data".to_string())); }
                Ok(())
            }));
            unsafe { if all_time { profile.ClearBrowsingData(mask, &handler) } else { profile.ClearBrowsingDataInTimeRange(mask, start, end, &handler) } }
        })();
        if result.is_err() { if let Some(send) = send.lock().unwrap().take() { let _ = send.send(Err("The selected browsing data could not be cleared".into())); } }
    }).map_err(|e|e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(30), receive).await
        .map_err(|_|"Clearing the selected browsing data timed out".to_string())?
        .map_err(|_|"Browser closed while clearing data".to_string())?
}
