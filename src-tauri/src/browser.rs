use crate::model::*;
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Instant,
};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl,
    webview::{
        DownloadEvent, NewWindowResponse, PageLoadEvent, PermissionKind, PermissionResponse,
        WebviewBuilder,
    },
};
static NEXT_WINDOW: AtomicU32 = AtomicU32::new(1);
pub fn view_label(owner: &str, id: u32) -> String {
    format!("tab-{owner}-{id}")
}
pub fn shell_label(owner: &str) -> String {
    format!("shell-{owner}")
}
pub fn emit(app: &AppHandle, owner: &str) {
    let a = app.clone();
    let o = owner.to_string();
    tauri::async_runtime::spawn(async move {
        let state = a.state::<Browser>();
        let settings = state.settings.lock().unwrap().clone();
        let value = state
            .windows
            .lock()
            .unwrap()
            .get(&o)
            .map(|w| w.snapshot(&settings));
        if let Some(value) = value {
            let _ = a.emit_to(shell_label(&o), "browser:state", value);
        }
    });
}
pub fn layout(app: &AppHandle, owner: &str) {
    let state = app.state::<Browser>();
    let (tabs, left, right, b, ratio) = match state.windows.lock().unwrap().get(owner) {
        Some(w) => (w.tabs.clone(), w.left, w.right, w.bounds, w.ratio),
        None => return,
    };
    let efficient = state.settings.lock().unwrap()["performance"]["efficiencyMode"] == true;
    let split_width = ((b.width - 12.) * ratio).floor().max(1.);
    for t in tabs {
        if let Some(view) = app.get_webview(&view_label(owner, t.id)) {
            let visible = b.height > 0. && b.width > 0. && (t.id == left || right == Some(t.id));
            if visible {
                let (x, width) = if right.is_some() {
                    if t.id == left {
                        (b.x, split_width)
                    } else {
                        (
                            b.x + split_width + 12.,
                            (b.width - split_width - 12.).max(1.),
                        )
                    }
                } else {
                    (b.x, b.width)
                };
                let _ = view.set_position(LogicalPosition::new(x, b.y));
                let _ = view.set_size(LogicalSize::new(width.max(1.), b.height.max(1.)));
                crate::platform::efficiency(&view, false);
                let _ = view.show();
            } else {
                let _ = view.hide();
                if efficient && !t.audible {
                    crate::platform::efficiency(&view, true);
                }
            }
        }
    }
}
pub fn window(app: &AppHandle, private: bool) -> Result<String, String> {
    let id = NEXT_WINDOW.fetch_add(1, Ordering::Relaxed);
    let owner = format!("window-{id}");
    let mut window_builder = tauri::window::WindowBuilder::new(app, &owner)
        .title(if private {
            "Aero Browser — Private"
        } else {
            "Aero Browser"
        })
        .inner_size(1400., 900.)
        .min_inner_size(900., 620.)
        .decorations(false)
        .visible(false);
    if crate::smoke::enabled() {
        window_builder = window_builder.position(-10000., -10000.).focused(false);
    }
    let win = window_builder.build().map_err(|e| e.to_string())?;
    crate::session_lock::install(app, &owner);
    let state = app.state::<Browser>();
    state
        .windows
        .lock()
        .unwrap()
        .insert(owner.clone(), WindowModel::new(owner.clone(), private));
    let hwnd = win.hwnd().map_err(|e| e.to_string())?.0 as isize;
    if let Some(model) = state.windows.lock().unwrap().get_mut(&owner) {
        model.native_hwnd = hwnd;
    }
    let mut shell = WebviewBuilder::new(shell_label(&owner), WebviewUrl::App("index.html".into()))
        .use_https_scheme(true)
        .incognito(private)
        .data_directory(state.profile.join(if private {
            format!("Shell-Private-{owner}")
        } else {
            "Shell".into()
        }))
        .on_navigation(|url| trusted_url(url, "index.html"));
    if state.startup_gpu_disabled {
        shell=shell.additional_browser_args("--force-webrtc-ip-handling-policy=default_public_interface_only --disable-background-networking --autoplay-policy=no-user-gesture-required --disable-gpu --disable-gpu-compositing --disable-accelerated-video-decode --disable-accelerated-video-encode");
    }
    let shell_view = win
        .add_child(
            shell,
            LogicalPosition::new(0., 0.),
            LogicalSize::new(1400., 900.),
        )
        .map_err(|e| e.to_string())?;
    crate::platform::install(&shell_view, app.clone(), owner.clone(), 0);
    let app_event = app.clone();
    let name = owner.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::Resized(size) => {
            let a = app_event.clone();
            let w = name.clone();
            let size = *size;
            tauri::async_runtime::spawn(async move {
                if let Some(shell) = a.get_webview(&shell_label(&w)) {
                    let _ = shell.set_size(size);
                }
                layout(&a, &w);
            });
        }
        tauri::WindowEvent::Moved(_) => {
            let a = app_event.clone();
            let w = name.clone();
            tauri::async_runtime::spawn(async move {
                let ids = a
                    .state::<Browser>()
                    .windows
                    .lock()
                    .unwrap()
                    .get(&w)
                    .map(|m| m.tabs.iter().map(|t| t.id).collect::<Vec<_>>())
                    .unwrap_or_default();
                for id in ids {
                    if let Some(view) = a.get_webview(&view_label(&w, id)) {
                        let _ = crate::platform::action(&view, "notify-position", 0.);
                    }
                }
            });
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            let state = app_event.state::<Browser>();
            let clear_on_exit = state.settings.lock().unwrap()["privacy"]["clearOnExit"] == true;
            let should_clear = {
                let mut all = state.windows.lock().unwrap();
                let normal_count = all.values().filter(|w| !w.private && !w.closing).count();
                if let Some(w) = all.get_mut(&name) {
                    if clear_on_exit && !w.private && normal_count <= 1 && !w.closing {
                        w.closing = true;
                        true
                    } else { false }
                } else { false }
            };
            if should_clear {
                api.prevent_close();
                let a = app_event.clone();
                let o = name.clone();
                tauri::async_runtime::spawn(async move {
                    crate::vault::lock(&a, &o);
                    let result = async {
                        if let Ok(view) = active_view(&a, &o) { crate::platform::clear_data(&view).await?; }
                        Ok::<(), String>(())
                    }.await;
                    if let Err(error) = result {
                        if let Some(w) = a.state::<Browser>().windows.lock().unwrap().get_mut(&o) { w.closing = false; }
                        rfd::AsyncMessageDialog::new().set_title("Aero could not clear browsing data")
                            .set_description(format!("{error}. Aero stayed open so you can retry."))
                            .set_buttons(rfd::MessageButtons::Ok).show().await;
                    } else if let Some(window) = a.get_window(&o) { let _ = window.destroy(); }
                });
            }
        }
        tauri::WindowEvent::Destroyed => {
            crate::vault::closed(&app_event, &name);
        }
        _ => {}
    });
    let home = state.settings.lock().unwrap()["startup"]["homePage"]
        .as_str()
        .unwrap_or("https://duckduckgo.com/")
        .to_string();
    create_tab(app, &owner, &home, true)?;
    let _ = app.emit_to(shell_label(&owner), "browser:private-mode", private);
    // Creating an additional WebView2 host can transiently move native focus even
    // when its window is hidden. Finish creating the suggestion host before the
    // browser becomes interactive, so typing never triggers host creation.
    let startup_app = app.clone();
    let startup_owner = owner.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = crate::suggestions::prepare(&startup_app, &startup_owner) {
            eprintln!("Suggestion host initialization failed: {error}");
        }
        if let Some(window) = startup_app.get_window(&startup_owner) { let _ = window.show(); }
    });
    Ok(owner)
}
pub fn trusted_url(url: &url::Url, page: &str) -> bool {
    ((url.scheme() == "https" && url.host_str() == Some("tauri.localhost") && url.port().is_none())
        || (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (cfg!(debug_assertions)
            && url.scheme() == "http"
            && ["localhost", "127.0.0.1"].contains(&url.host_str().unwrap_or(""))
            && url.port() == Some(5174)))
        && (url.path() == format!("/{page}") || (page == "index.html" && url.path() == "/"))
}
pub fn create_tab(app: &AppHandle, owner: &str, raw: &str, activate: bool) -> Result<u32, String> {
    let state = app.state::<Browser>();
    let settings = state.settings.lock().unwrap().clone();
    let url = address(raw, &settings);
    let (id, private) = {
        let mut all = state.windows.lock().unwrap();
        let w = all.get_mut(owner).ok_or("Browser window closed")?;
        if w.tabs.len() >= 60 {
            return Err("Aero supports up to 60 tabs. Close a tab first.".into());
        }
        let id = w.next;
        w.next += 1;
        w.tabs.push(Tab {
            id,
            title: "New tab".into(),
            url: url.clone(),
            loading: true,
            last_active: Some(Instant::now()),
            ..Default::default()
        });
        if w.left == 0 {
            w.left = id;
            w.active = id;
        } else if activate {
            if w.right == Some(w.active) {
                w.right = Some(id)
            } else {
                w.left = id
            }
            w.active = id;
        }
        w.selection = None;
        (id, w.private)
    };
    build_view(app, owner, id, &url, private, &settings)?;
    layout(app, owner);
    emit(app, owner);
    Ok(id)
}
pub fn build_view(
    app: &AppHandle,
    owner: &str,
    id: u32,
    url: &str,
    private: bool,
    settings: &Value,
) -> Result<(), String> {
    let win = app.get_window(owner).ok_or("Browser window closed")?;
    let a_title = app.clone();
    let o_title = owner.to_string();
    let a_page = app.clone();
    let o_page = owner.to_string();
    let a_navigation = app.clone();
    let o_navigation = owner.to_string();
    let a_new = app.clone();
    let o_new = owner.to_string();
    let a_download = app.clone();
    let o_download = owner.to_string();
    let a_permissions = app.clone();
    let directory = app.state::<Browser>().profile.join(if private {
        format!("Private-{owner}")
    } else {
        "Session Data".into()
    });
    let mut builder=WebviewBuilder::new(view_label(owner,id),WebviewUrl::External(url.parse().map_err(|_|"Invalid webpage address")?)).data_directory(directory).incognito(private).zoom_hotkeys_enabled(false).additional_browser_args("--force-webrtc-ip-handling-policy=default_public_interface_only --disable-background-networking --autoplay-policy=no-user-gesture-required")
 .on_navigation(move|url|{let settings=a_navigation.state::<Browser>().settings.lock().unwrap().clone();if settings["privacy"]["upgradeHttps"]==true&&url.scheme()=="http"&&!["localhost","127.0.0.1"].contains(&url.host_str().unwrap_or("")){let mut secure=url.clone();let _=secure.set_scheme("https");let a=a_navigation.clone();let o=o_navigation.clone();tauri::async_runtime::spawn(async move{if let Some(view)=a.get_webview(&view_label(&o,id)){let _=view.navigate(secure);}if let Some(w)=a.state::<Browser>().windows.lock().unwrap().get_mut(&o){if let Some(t)=w.tabs.iter_mut().find(|t|t.id==id){t.upgrades+=1;}}emit(&a,&o);});return false}true})
 .on_document_title_changed(move|_,title|{let state=a_title.state::<Browser>();if let Some(w)=state.windows.lock().unwrap().get_mut(&o_title){if let Some(t)=w.tabs.iter_mut().find(|t|t.id==id){t.title=title;}}emit(&a_title,&o_title);})
 .on_page_load(move|view,event|{let state=a_page.state::<Browser>();if !state.windows.lock().unwrap().contains_key(&o_page){return}if let Some(w)=state.windows.lock().unwrap().get_mut(&o_page){if let Some(t)=w.tabs.iter_mut().find(|t|t.id==id){t.loading=event.event()==PageLoadEvent::Started;t.url=event.url().to_string();}if event.event()==PageLoadEvent::Started{w.selection=None;if let Some(tab)=w.tabs.iter_mut().find(|t|t.id==id){tab.secure_context=None;}}}if event.event()==PageLoadEvent::Finished{crate::platform::update_spellcheck(&view);let config=state.settings.lock().unwrap().clone();let tools=include_str!("page-tools.js").replace("hideAdElements: true",&format!("hideAdElements: {}",config["shields"]["hideAdElements"]==true)).replace("blockYoutubeVideoAds: true",&format!("blockYoutubeVideoAds: {}",config["shields"]["blockYoutubeVideoAds"]==true));let _=view.eval(tools);if config["privacy"]["fingerprintProtection"]==true{let _=view.eval(include_str!("fingerprint.js"));}let a=a_page.clone();let o=o_page.clone();let v=view.clone();tauri::async_runtime::spawn(async move{let _=crate::submission::observe(&a,&o,id,&v).await;});}emit(&a_page,&o_page);})
 .on_new_window(move|url,_|{let a=a_new.clone();let o=o_new.clone();tauri::async_runtime::spawn(async move{let _=create_tab(&a,&o,url.as_str(),true);});NewWindowResponse::Deny})
 .on_download(move|_,event|{let state=a_download.state::<Browser>();match event{DownloadEvent::Requested{url,destination}=>{let settings=state.settings.lock().unwrap().clone();if settings["downloads"]["askWhereToSave"].as_bool()==Some(true){if let Some(path)=rfd::FileDialog::new().set_file_name(destination.file_name().and_then(|s|s.to_str()).unwrap_or("download")).save_file(){*destination=path}else{return false}}else if let Some(dir)=settings["downloads"]["directory"].as_str().filter(|s|!s.is_empty()){*destination=std::path::PathBuf::from(dir).join(destination.file_name().unwrap_or_default());}state.downloads.lock().unwrap().insert(0,json!({"id":uuid::Uuid::new_v4().to_string(),"filename":destination.file_name().unwrap_or_default().to_string_lossy(),"state":"progressing","receivedBytes":0,"totalBytes":0,"startedAt":crate::browsing_data::now_ms(),"privateOwner":if private{Some(o_download.clone())}else{None},"savePath":destination,"url":url.as_str()}));},DownloadEvent::Finished{url,path,success}=>{for record in state.downloads.lock().unwrap().iter_mut(){if record["url"]==url.as_str()&&crate::browsing_data::same_scope(record,if private{Some(o_download.as_str())}else{None}){record["state"]=json!(if success{"completed"}else{"interrupted"});if let Some(ref path)=path{record["savePath"]=json!(path);record["receivedBytes"]=json!(std::fs::metadata(path).map(|m|m.len()).unwrap_or(0));}}}},_=>{}}let names=state.windows.lock().unwrap().keys().cloned().collect::<Vec<_>>();for name in names{let values=crate::browsing_data::downloads_for(&a_download,&name);let _=a_download.emit_to(shell_label(&name),"browser:downloads",&values);}true})
 .on_permission_request(move|_,kind|{let s=a_permissions.state::<Browser>().settings.lock().unwrap().clone();let allow=match kind{PermissionKind::Microphone=>s["permissions"]["microphone"].as_bool(),PermissionKind::Camera=>s["permissions"]["camera"].as_bool(),PermissionKind::Geolocation=>s["permissions"]["geolocation"].as_bool(),PermissionKind::Notifications=>s["permissions"]["notifications"].as_bool(),_=>Some(false)};if allow==Some(true){PermissionResponse::Allow}else{PermissionResponse::Deny}});
    if app.state::<Browser>().startup_gpu_disabled {
        builder=builder.additional_browser_args("--disable-gpu --disable-gpu-compositing --disable-accelerated-video-decode --disable-accelerated-video-encode");
    }
    let view = win
        .add_child(
            builder,
            LogicalPosition::new(0., 84.),
            LogicalSize::new(1200., 700.),
        )
        .map_err(|e| e.to_string())?;
    crate::platform::install(&view, app.clone(), owner.to_string(), id);
    crate::platform::configure(&view, settings.clone());
    crate::shields::install(&view, app.clone(), owner.to_string(), id);
    Ok(())
}
pub fn active_view(app: &AppHandle, owner: &str) -> Result<tauri::Webview, String> {
    let id = app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .ok_or("Browser window closed")?
        .active;
    app.get_webview(&view_label(owner, id))
        .ok_or("The tab is sleeping or unavailable".into())
}
pub fn restore(app: &AppHandle, owner: &str, id: u32) -> Result<(), String> {
    let state = app.state::<Browser>();
    let settings = state.settings.lock().unwrap().clone();
    let saved = {
        let mut all = state.windows.lock().unwrap();
        let w = all.get_mut(owner).ok_or("Browser window closed")?;
        let private = w.private;
        let tab = w.tabs.iter_mut().find(|t| t.id == id).ok_or("Tab closed")?;
        if !tab.discarded {
            return Ok(());
        }
        tab.discarded = false;
        tab.loading = true;
        (tab.url.clone(), tab.muted, private)
    };
    build_view(app, owner, id, &saved.0, saved.2, &settings)?;
    if saved.1 {
        if let Some(view) = app.get_webview(&view_label(owner, id)) {
            let _ = crate::platform::action(&view, "mute", 1.);
        }
    }
    Ok(())
}
pub fn discard(app: &AppHandle, owner: &str, id: u32) {
    let state = app.state::<Browser>();
    let can = {
        let mut all = state.windows.lock().unwrap();
        let Some(w) = all.get_mut(owner) else { return };
        if w.left == id || w.right == Some(id) {
            return;
        }
        if let Some(t) = w.tabs.iter_mut().find(|t| t.id == id) {
            if t.audible {
                return;
            }
            t.discarded = true;
            t.loading = false;
            true
        } else {
            false
        }
    };
    if can {
        if let Some(view) = app.get_webview(&view_label(owner, id)) {
            let _ = view.close();
        }
        emit(app, owner);
    }
}
pub async fn command(
    app: &AppHandle,
    owner: &str,
    command: &str,
    payload: Value,
) -> Result<Value, String> {
    let state = app.state::<Browser>();
    let win = app.get_window(owner).ok_or("Browser window closed")?;
    let settings = state.settings.lock().unwrap().clone();
    let active = {
        state
            .windows
            .lock()
            .unwrap()
            .get(owner)
            .ok_or("Browser window closed")?
            .active
    };
    let id = payload.as_u64().unwrap_or(active as u64) as u32;
    match command {
        "new-tab" => {
            create_tab(
                app,
                owner,
                payload.as_str().unwrap_or(
                    settings["startup"]["homePage"]
                        .as_str()
                        .unwrap_or("https://duckduckgo.com/"),
                ),
                true,
            )?;
        }
        "close-tab" => {
            let closed = state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .close_tab(id);
            if closed {
                if let Some(v) = app.get_webview(&view_label(owner, id)) {
                    v.close().map_err(|e| e.to_string())?;
                }
            } else {
                let v = active_view(app, owner)?;
                v.navigate(
                    address(settings["startup"]["homePage"].as_str().unwrap_or("https://duckduckgo.com/"), &settings).parse().map_err(|_|"Invalid homepage")?,
                )
                .map_err(|e| e.to_string())?;
            }
        }
        "activate-tab" | "move-tab-to-pane" => {
            let (id, right) = if command == "activate-tab" {
                let all = state.windows.lock().unwrap();
                let w = all.get(owner).unwrap();
                (
                    id,
                    w.right == Some(id) || (w.right == Some(w.active) && w.left != id),
                )
            } else {
                (
                    payload["id"].as_u64().ok_or("Invalid tab")? as u32,
                    payload["pane"] == "right",
                )
            };
            let need = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .map(|w| w.tabs.len() == 1 && right && w.left == id)
                .unwrap_or(false);
            if need {
                create_tab(app, owner, "https://duckduckgo.com/", false)?;
            }
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .move_tab(id, right);
            restore(app, owner, id)?;
        }
        "activate-pane" => {
            let mut all = state.windows.lock().unwrap();
            let w = all.get_mut(owner).unwrap();
            w.active = if payload == "right" {
                w.right.unwrap_or(w.left)
            } else {
                w.left
            };
            w.selection = None;
        }
        "toggle-split" => {
            let right = state.windows.lock().unwrap().get(owner).unwrap().right;
            if right.is_some() {
                let mut all = state.windows.lock().unwrap();
                let w = all.get_mut(owner).unwrap();
                w.right = None;
                w.active = w.left;
            } else {
                let next = state
                    .windows
                    .lock()
                    .unwrap()
                    .get(owner)
                    .unwrap()
                    .tabs
                    .iter()
                    .find(|t| t.id != active)
                    .map(|t| t.id);
                let next = match next {
                    Some(id) => id,
                    None => create_tab(app, owner, "https://duckduckgo.com/", false)?,
                };
                state.windows.lock().unwrap().get_mut(owner).unwrap().right = Some(next);
                restore(app, owner, next)?;
            }
        }
        "swap-panes" => {
            let mut all = state.windows.lock().unwrap();
            let w = all.get_mut(owner).unwrap();
            if let Some(right) = w.right {
                w.right = Some(w.left);
                w.left = right;
            }
        }
        "set-split-ratio" => {
            if let Some(ratio) = payload.as_f64().filter(|v| v.is_finite()) {
                state.windows.lock().unwrap().get_mut(owner).unwrap().ratio = ratio.clamp(0.2, 0.8);
            }
        }
        "reorder-tab" => {
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .reorder(
                    payload["id"].as_u64().ok_or("Invalid tab")? as u32,
                    payload["beforeId"].as_u64().map(|n| n as u32),
                );
        }
        "navigate" | "home" => {
            let target = address(
                if command == "home" {
                    settings["startup"]["homePage"].as_str().unwrap_or("")
                } else {
                    payload.as_str().unwrap_or("")
                },
                &settings,
            );
            active_view(app, owner)?
                .navigate(target.parse().map_err(|_| "Invalid address")?)
                .map_err(|e| e.to_string())?;
        }
        "back" | "forward" | "reload" | "stop" => {
            crate::platform::action(&active_view(app, owner)?, command, 0.)?;
        }
        "hard-reload" => {
            crate::platform::cdp(
                &active_view(app, owner)?,
                "Page.reload",
                json!({"ignoreCache":true}),
            )
            .await?;
        }
        "toggle-mute" => {
            let muted = {
                let mut all = state.windows.lock().unwrap();
                let t = all
                    .get_mut(owner)
                    .unwrap()
                    .tabs
                    .iter_mut()
                    .find(|t| t.id == id)
                    .ok_or("Tab closed")?;
                t.muted = !t.muted;
                t.muted
            };
            if let Some(v) = app.get_webview(&view_label(owner, id)) {
                crate::platform::action(&v, "mute", if muted { 1. } else { 0. })?;
            }
        }
        "zoom-in" | "zoom-out" | "zoom-reset" => {
            let zoom = {
                let mut all = state.windows.lock().unwrap();
                let w = all.get_mut(owner).unwrap();
                w.zoom = match command {
                    "zoom-in" => (w.zoom + 10).min(200),
                    "zoom-out" => w.zoom.saturating_sub(10).max(50),
                    _ => 100,
                };
                w.zoom
            };
            crate::platform::action(&active_view(app, owner)?, "zoom", zoom as f64 / 100.)?;
        }
        "minimize" => {
            crate::vault::lock(app, owner);
            win.minimize().map_err(|e| e.to_string())?;
        }
        "maximize" => {
            if win.is_maximized().unwrap_or(false) {
                win.unmaximize()
            } else {
                win.maximize()
            }
            .map_err(|e| e.to_string())?;
        }
        "close-window" => {
            win.close().map_err(|e| e.to_string())?;
        }
        "toggle-fullscreen" => {
            let full = !win.is_fullscreen().unwrap_or(false);
            win.set_fullscreen(full).map_err(|e| e.to_string())?;
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .fullscreen = full;
        }
        "new-window" | "new-private-window" => {
            window(app, command == "new-private-window")?;
        }
        "discard-tab" => discard(app, owner, id),
        "discard-other-tabs" => {
            let ids = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .unwrap()
                .tabs
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>();
            for id in ids {
                discard(app, owner, id)
            }
        }
        "reopen-tab" => {
            let url = state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .closed
                .first()
                .cloned();
            if let Some(url) = url {
                state
                    .windows
                    .lock()
                    .unwrap()
                    .get_mut(owner)
                    .unwrap()
                    .closed
                    .remove(0);
                create_tab(app, owner, &url.0, true)?;
            }
        }
        "duplicate-tab" => {
            let url = active_view(app, owner)?.url().map_err(|e| e.to_string())?;
            create_tab(app, owner, url.as_str(), true)?;
        }
        "next-tab" | "previous-tab" => {
            let next = {
                let all = state.windows.lock().unwrap();
                let w = all.get(owner).unwrap();
                let pos = w.tabs.iter().position(|t| t.id == active).unwrap_or(0);
                let offset = if command == "next-tab" {
                    1
                } else {
                    w.tabs.len() - 1
                };
                w.tabs[(pos + offset) % w.tabs.len()].id
            };
            let right = {
                let all = state.windows.lock().unwrap();
                let w = all.get(owner).unwrap();
                w.right == Some(next) || (w.right == Some(w.active) && w.left != next)
            };
            state
                .windows
                .lock()
                .unwrap()
                .get_mut(owner)
                .unwrap()
                .move_tab(next, right);
            restore(app, owner, next)?;
        }
        "open-password-manager" => {
            crate::vault::open(app, owner)?;
        }
        "focus-address" => {
            app.get_webview(&shell_label(owner))
                .ok_or("Browser interface unavailable")?
                .set_focus()
                .map_err(|e| e.to_string())?;
            let _ = app.emit_to(shell_label(owner), "browser:shortcut", "focus-address");
            return Ok(Value::Null);
        }
        "find" => {
            let text = payload["text"].as_str().unwrap_or("");
            let code = format!(
                "window.find({},false,{},true)",
                json!(text),
                payload["forward"] == false
            );
            active_view(app, owner)?
                .eval(code)
                .map_err(|e| e.to_string())?;
        }
        "print" | "print-system" => {
            active_view(app, owner)?
                .eval("window.print()")
                .map_err(|e| e.to_string())?;
        }
        "view-source" => {
            let url = active_view(app, owner)?.url().map_err(|e| e.to_string())?;
            create_tab(app, owner, &format!("view-source:{url}"), true)?;
        }
        "open-file" => {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Web documents", &["html", "htm", "pdf", "txt", "svg"])
                .pick_file()
                .await
            {
                let url = url::Url::from_file_path(file.path()).map_err(|_| "Invalid file path")?;
                active_view(app, owner)?
                    .navigate(url)
                    .map_err(|e| e.to_string())?;
            }
        }
        "save-page" => {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("Aero-page.mhtml")
                .save_file()
                .await
            {
                let snapshot = crate::platform::cdp(
                    &active_view(app, owner)?,
                    "Page.captureSnapshot",
                    json!({"format":"mhtml"}),
                )
                .await?;
                file.write(
                    snapshot["data"]
                        .as_str()
                        .ok_or("The page could not be saved")?
                        .as_bytes(),
                )
                .await
                .map_err(|e| e.to_string())?;
            }
        }
        "capture-page" => {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("Aero-capture.png")
                .save_file()
                .await
            {
                use base64::Engine;
                let image = crate::platform::cdp(
                    &active_view(app, owner)?,
                    "Page.captureScreenshot",
                    json!({"format":"png"}),
                )
                .await?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(image["data"].as_str().ok_or("Capture failed")?)
                    .map_err(|_| "Capture failed")?;
                file.write(&bytes).await.map_err(|e| e.to_string())?;
            }
        }
        "read-aloud" | "reader-mode" => {
            active_view(app, owner)?
                .eval(if command == "read-aloud" {
                    "window.__aeroReadAloud?.()"
                } else {
                    "window.__aeroReader?.()"
                })
                .map_err(|e| e.to_string())?;
        }
        "paste-and-go" => {
            let text = crate::vault::clipboard_read()?;
            let target = address(&text, &settings);
            active_view(app, owner)?
                .navigate(target.parse().map_err(|_| "Invalid address")?)
                .map_err(|e| e.to_string())?;
        }
        "fire" => {
            let ids = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .unwrap()
                .tabs
                .iter()
                .map(|t| t.id)
                .collect::<Vec<_>>();
            for id in ids {
                if let Some(v) = app.get_webview(&view_label(owner, id)) {
                    crate::platform::clear_data(&v).await?;
                }
            }
            active_view(app, owner)?
                .navigate(
                    address(settings["startup"]["homePage"].as_str().unwrap_or("https://duckduckgo.com/"), &settings).parse().map_err(|_|"Invalid homepage")?,
                )
                .map_err(|e| e.to_string())?;
        }
        "set-shield-mode" | "set-script-blocking" => {
            let host = active_view(app, owner)?
                .url()
                .map_err(|e| e.to_string())?
                .host_str()
                .unwrap_or("")
                .to_string();
            let mut all = state.windows.lock().unwrap();
            let w = all.get_mut(owner).unwrap();
            if command == "set-shield-mode" {
                let mode = payload
                    .as_str()
                    .filter(|s| ["standard", "aggressive", "disabled"].contains(s))
                    .ok_or("Invalid shields setting")?;
                w.modes.insert(host, mode.into());
            } else {
                w.scripts.insert(host, payload.as_bool().unwrap_or(false));
            }
            drop(all);
            crate::platform::action(&active_view(app, owner)?, "reload", 0.)?;
        }
        "restart" => {
            if rfd::AsyncMessageDialog::new()
                .set_title("Restart Aero")
                .set_description("Restart Aero to apply graphics changes? Open tabs will close.")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                .await
                == rfd::MessageDialogResult::Yes
            {
                app.restart();
            }
        }
        "open-downloads-folder" => {
            let path = settings["downloads"]["directory"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| dirs::download_dir().unwrap_or_default());
            let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
        }
        "toggle-devtools" => {
            let view = active_view(app, owner)?;
            if view.is_devtools_open() {
                view.close_devtools()
            } else {
                view.open_devtools()
            }
        }
        "focus-toolbar" | "focus-favorites" => {
            let _ = app.emit_to(shell_label(owner), "browser:shortcut", command);
        }
        "focus-browser" => {
            active_view(app, owner)?
                .set_focus()
                .map_err(|e| e.to_string())?;
        }
        "save-all-favorites" => {
            let values = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .unwrap()
                .tabs
                .iter()
                .filter(|t| t.url.starts_with("https://") || t.url.starts_with("http://"))
                .map(|t| json!({"title":t.title,"url":t.url}))
                .collect::<Vec<_>>();
            let _ = app.emit_to(shell_label(owner), "browser:save-all-favorites", values);
        }
        "open-url-new-tab" => {
            let url = active_view(app, owner)?.url().map_err(|e| e.to_string())?;
            create_tab(app, owner, url.as_str(), true)?;
        }
        "select-tab" => {
            let position = payload.as_u64().unwrap_or(1) as usize;
            let selected = state
                .windows
                .lock()
                .unwrap()
                .get(owner)
                .and_then(|w| {
                    if position == 9 {
                        w.tabs.last()
                    } else {
                        w.tabs.get(position.saturating_sub(1))
                    }
                })
                .map(|t| t.id);
            if let Some(id) = selected {
                let right = {
                    let all = state.windows.lock().unwrap();
                    let w = all.get(owner).unwrap();
                    w.right == Some(id) || (w.right == Some(w.active) && w.left != id)
                };
                state
                    .windows
                    .lock()
                    .unwrap()
                    .get_mut(owner)
                    .unwrap()
                    .move_tab(id, right);
                restore(app, owner, id)?;
            }
        }
        "find-next" | "find-previous" => {
            let _ = app.emit_to(shell_label(owner), "browser:shortcut", command);
        }
        "paste-plain" => {
            let text = crate::vault::clipboard_read()?;
            crate::platform::cdp(
                &active_view(app, owner)?,
                "Input.insertText",
                json!({"text":text}),
            )
            .await?;
        }
        "help" => {
            create_tab(
                app,
                owner,
                "https://support.microsoft.com/en-us/edge/keyboard-shortcuts-in-microsoft-edge",
                true,
            )?;
        }
        "show-suggestions" | "hide-suggestions" => {
            crate::suggestions::request(app, owner, command, payload).await?;
            // Editing the address is UI-only: relaying browser state and resizing
            // site views here feeds focus/layout changes back into the editor.
            return Ok(Value::Null);
        }
        "ui:find"
        | "ui:settings"
        | "ui:downloads"
        | "ui:history"
        | "ui:add-favorite"
        | "ui:favorites"
        | "ui:toggle-sidebar"
        | "ui:toggle-favorites-bar"
        | "ui:tab-search"
        | "ui:clear-data"
        | "ui:focus-toolbar"
        | "ui:focus-favorites" => {
            let _ = app.emit_to(
                shell_label(owner),
                "browser:shortcut",
                command.trim_start_matches("ui:"),
            );
        }
        "set-shortcut-mapping" => {
            crate::settings::shortcuts(app, payload, false)?;
        }
        "reset-shortcut-mapping" => {
            crate::settings::shortcuts(app, Value::Null, true)?;
        }
        other => return Err(format!("Unknown browser command: {other}")),
    }
    layout(app, owner);
    emit(app, owner);
    if [
        "activate-tab",
        "activate-pane",
        "move-tab-to-pane",
        "next-tab",
        "previous-tab",
    ]
    .contains(&command)
    {
        if let Ok(view) = active_view(app, owner) {
            let _ = view.set_focus();
        }
    }
    Ok(Value::Null)
}
pub fn mapped_key(
    app: &AppHandle,
    key: u32,
    ctrl: bool,
    shift: bool,
    alt: bool,
) -> Option<(String, Value)> {
    if ctrl && !shift && !alt && (0x31..=0x39).contains(&key) {
        return Some(("select-tab".into(), json!(key - 0x30)));
    }
    if ctrl && !alt && [0xbb, 0x6b].contains(&key) {
        return Some(("zoom-in".into(), Value::Null));
    }
    if ctrl && !alt && [0xbd, 0x6d].contains(&key) {
        return Some(("zoom-out".into(), Value::Null));
    }
    let name = match key {
        0x09 => "Tab".into(),
        0x0d => "Enter".into(),
        0x1b => "Escape".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x27 => "Right".into(),
        0x2e => "Delete".into(),
        0xbb => "Plus".into(),
        0xbd => "Minus".into(),
        0x70..=0x7b => format!("F{}", key - 0x6f),
        0x30..=0x5a => (key as u8 as char).to_string(),
        _ => return key_command(key, ctrl, shift, alt),
    };
    let combo = format!(
        "{}{}{}{}",
        if ctrl { "Ctrl+" } else { "" },
        if shift { "Shift+" } else { "" },
        if alt { "Alt+" } else { "" },
        name
    );
    let settings = app.state::<Browser>().settings.lock().unwrap().clone();
    if let Some(mapping) = settings["shortcuts"]["mapping"].as_object() {
        for (action, value) in mapping {
            if value
                .as_str()
                .map(|v| v.eq_ignore_ascii_case(&combo))
                .unwrap_or(false)
            {
                let command = match action.as_str() {
                    "mute-tab" => "toggle-mute",
                    "fullscreen" => "toggle-fullscreen",
                    "reader" => "reader-mode",
                    "devtools" => "toggle-devtools",
                    "find" => "ui:find",
                    "settings" | "focus-settings" => "ui:settings",
                    "history" => "ui:history",
                    "downloads" => "ui:downloads",
                    "add-favorite" => "ui:add-favorite",
                    "favorites" => "ui:favorites",
                    "tab-search" => "ui:tab-search",
                    "clear-data" => "ui:clear-data",
                    "toggle-sidebar" => "ui:toggle-sidebar",
                    "toggle-favorites-bar" => "ui:toggle-favorites-bar",
                    "collections" => "ui:toggle-sidebar",
                    other => other,
                };
                return Some((command.into(), Value::Null));
            }
        }
    }
    key_command(key, ctrl, shift, alt)
}
pub fn key_command(key: u32, ctrl: bool, shift: bool, alt: bool) -> Option<(String, Value)> {
    let action = match (key, ctrl, shift, alt) {
        (0x54, true, true, false) => "reopen-tab",
        (0x54, true, false, false) => "new-tab",
        (0x57, true, _, false) => "close-tab",
        (0x4c, true, _, false) | (0x75, false, _, false) => "focus-address",
        (0x50, true, true, false) => "open-password-manager",
        (0x4e, true, true, false) => "new-private-window",
        (0x4e, true, false, false) => "new-window",
        (0x09, true, false, false) => "next-tab",
        (0x09, true, true, false) => "previous-tab",
        (0x4d, true, false, false) => "toggle-mute",
        (0x46, true, false, false) => "ui:find",
        (0x46, false, false, true) | (0x79, false, false, false) => "ui:settings",
        (0x44, true, false, false) => "ui:add-favorite",
        (0x48, true, false, false) => "ui:history",
        (0x4a, true, false, false) => "ui:downloads",
        (0x42, true, true, false) => "ui:toggle-favorites-bar",
        (0x50, true, false, false) => "print",
        (0x53, true, false, false) => "save-page",
        (0x4f, true, false, false) => "open-file",
        (0x53, true, true, false) => "capture-page",
        (0x74, false, false, false) | (0x52, true, false, false) => "reload",
        (0x74, false, true, false) | (0x52, true, true, false) => "hard-reload",
        (0x7a, false, _, false) => "toggle-fullscreen",
        (0x7b, false, _, false) => "toggle-devtools",
        (0x25, false, _, true) => "back",
        (0x27, false, _, true) => "forward",
        (0x24, false, _, true) => "home",
        _ => return None,
    };
    Some((action.into(), Value::Null))
}
