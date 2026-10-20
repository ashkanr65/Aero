use serde_json::Value;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

// Tauri's window visibility/resize path may activate an owned WebView2 window.
// A suggestion list must never participate in the keyboard focus chain.
fn hide(window: &WebviewWindow) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    window
        .run_on_main_thread(move || unsafe {
            let _ = SetWindowPos(
                HWND(hwnd as *mut _),
                None,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE | SWP_HIDEWINDOW,
            );
        })
        .map_err(|e| e.to_string())
}

// Serialize popup creation without holding the model lock during native calls.
static UPDATES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn render(app: &AppHandle, owner: &str) -> Option<String> {
    let state = app.state::<crate::model::Browser>();
    let windows = state.windows.lock().unwrap();
    let w = windows.get(owner)?;
    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let rows = w.suggestion_items.iter().enumerate().map(|(i, (title, url))| format!(
        "<a href='aero-suggestion://select/{}/{i}'><strong>{}</strong><small>{}</small></a>",
        w.suggestion_generation, escape(title), escape(url)
    )).collect::<Vec<_>>().join("");
    let html = format!(
        "<style>body{{margin:0;padding:5px;background:#fff;color:#172033;font:13px Segoe UI}}a{{display:flex;gap:12px;padding:10px;border-radius:6px;color:inherit;text-decoration:none}}a:hover{{background:#e9f2ff}}strong{{max-width:40%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}small{{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}@media(prefers-color-scheme:dark){{body{{background:#25272c;color:#f5f5f5}}a:hover{{background:#353a43}}}}</style>{rows}"
    );
    Some(format!(
        "if(document.body)document.body.innerHTML={}",
        serde_json::json!(html)
    ))
}

pub fn prepare(app: &AppHandle, owner: &str) -> Result<(), String> {
    popup(app, owner).map(|_| ())
}

fn popup(app: &AppHandle, owner: &str) -> Result<WebviewWindow, String> {
    let label = format!("suggestions-{owner}");
    if let Some(window) = app.get_webview_window(&label) {
        return Ok(window);
    }
    let parent = app.get_window(owner).ok_or("Browser window closed")?;
    let callback_app = app.clone();
    let callback_owner = owner.to_string();
    let callback_label = label.clone();
    let load_owner = owner.to_string();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("blank.html".into()))
        .use_https_scheme(true)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .focusable(false)
        .on_page_load(move |view, event| {
            if matches!(event.event(), tauri::webview::PageLoadEvent::Finished) {
                if let Some(script) = render(view.app_handle(), &load_owner) {
                    let _ = view.eval(script);
                }
            }
        })
        .on_navigation(move |url| {
            if url.scheme() == "aero-suggestion" {
                let mut parts = url.path().trim_start_matches('/').split('/');
                let selection = parts
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .zip(parts.next().and_then(|s| s.parse::<usize>().ok()));
                let target = {
                    let state = callback_app.state::<crate::model::Browser>();
                    let windows = state.windows.lock().unwrap();
                    windows.get(&callback_owner).and_then(|w| {
                        let (generation, index) = selection?;
                        (generation == w.suggestion_generation)
                            .then(|| w.suggestion_items.get(index).map(|(_, url)| url.clone()))
                            .flatten()
                    })
                };
                if let Some(target) = target {
                    let a = callback_app.clone();
                    let o = callback_owner.clone();
                    let l = callback_label.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(w) = a.get_webview_window(&l) {
                            let _ = hide(&w);
                        }
                        let settings = a
                            .state::<crate::model::Browser>()
                            .settings
                            .lock()
                            .unwrap()
                            .clone();
                        if let Ok(view) = crate::browser::active_view(&a, &o) {
                            if let Ok(url) = crate::model::address(&target, &settings).parse() {
                                let _ = view.navigate(url);
                                let _ = view.set_focus();
                            }
                        }
                    });
                }
                return false;
            }
            crate::browser::trusted_url(url, "blank.html")
        })
        .build()
        .map_err(|e| e.to_string())?;
    let registered = {
        let state = app.state::<crate::model::Browser>();
        if let Some(w) = state.windows.lock().unwrap().get_mut(owner) {
            w.suggestion_label = Some(label);
            true
        } else {
            false
        }
    };
    if !registered {
        let _ = window.close();
        return Err("Browser window closed during startup".into());
    }
    if let Ok(hwnd) = window.hwnd() {
        let main = parent.hwnd().map_err(|e| e.to_string())?;
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::GWLP_HWNDPARENT,
                main.0 as isize,
            );
        }
    }
    Ok(window)
}

pub async fn request(
    app: &AppHandle,
    owner: &str,
    command: &str,
    payload: Value,
) -> Result<(), String> {
    let _update = UPDATES.lock().await;
    let items = if command == "hide-suggestions" {
        vec![]
    } else {
        payload["items"]
            .as_array()
            .map(|v| {
                v.iter()
                    .take(5)
                    .filter_map(|v| {
                        Some((
                            v["title"].as_str()?.to_string(),
                            v["url"].as_str()?.to_string(),
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let label = format!("suggestions-{owner}");
    {
        let state = app.state::<crate::model::Browser>();
        let mut windows = state.windows.lock().unwrap();
        let w = windows.get_mut(owner).ok_or("Browser window closed")?;
        w.suggestion_generation += 1;
        w.suggestion_items = items.clone();
    }
    if items.is_empty() {
        if let Some(window) = app.get_webview_window(&label) {
            hide(&window)?;
        }
        return Ok(());
    }
    let parent = app.get_window(owner).ok_or("Browser window closed")?;
    // Host creation is a startup operation; never retry it from an input event.
    let Some(window) = app.get_webview_window(&label) else {
        return Ok(());
    };
    if let Some(script) = render(app, owner) {
        window.eval(script).map_err(|e| e.to_string())?;
    }
    let bounds = &payload["bounds"];
    let position = parent.inner_position().map_err(|e| e.to_string())?;
    let scale = parent.scale_factor().unwrap_or(1.);
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let x = position.x + (bounds["x"].as_f64().unwrap_or(0.) * scale) as i32;
    let y = position.y + (bounds["y"].as_f64().unwrap_or(0.) * scale) as i32;
    let width =
        (bounds["width"].as_f64().unwrap_or(400.).clamp(320., 1000.) * scale).round() as i32;
    let height = ((items.len() as f64 * 38. + 18.) * scale).round() as i32;
    window
        .run_on_main_thread(move || unsafe {
            let _ = SetWindowPos(
                HWND(hwnd as *mut _),
                Some(HWND_TOP),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
            );
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
