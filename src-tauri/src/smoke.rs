use crate::{browser, model::Browser};
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
pub fn enabled() -> bool {
    cfg!(debug_assertions) && std::env::args().any(|s| s == "--smoke-test")
}
pub fn fixture(url: &str) -> Option<String> {
    if !enabled() {
        return None;
    }
    let u = url::Url::parse(url).ok()?;
    if !["aero-synthetic.test", "other-synthetic.test"].contains(&u.host_str().unwrap_or("")) {
        return None;
    }
    if u.path().ends_with("favicon.ico") {
        return None;
    }
    Some(format!(
        r#"<!doctype html><html><head><title>Synthetic {}</title></head><body><h1>Synthetic login test</h1><form action="/submitted" method="post"><input name="username" autocomplete="username" value="synthetic-user"><input type="password" name="password" value="synthetic-only-password"><button type="submit">Sign in</button></form><iframe src="https://other-synthetic.test/frame"></iframe><script>document.querySelector('form').addEventListener('submit',e=>e.preventDefault());</script></body></html>"#,
        u.path()
    ))
}
async fn eval(view: &tauri::Webview, expression: &str) -> Result<Value, String> {
    let result = crate::platform::cdp(
        view,
        "Runtime.evaluate",
        json!({"expression":expression,"returnByValue":true,"awaitPromise":true}),
    )
    .await?;
    if result.get("exceptionDetails").is_some() {
        return Err(format!(
            "Synthetic page evaluation failed: {}",
            result["exceptionDetails"]["exception"]["description"]
        ));
    }
    Ok(result["result"]["value"].clone())
}
async fn ready(view: &tauri::Webview, expression: &str) -> Result<(), String> {
    for _ in 0..80 {
        if eval(view, expression).await.unwrap_or(Value::Null) == true {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(format!("Timed out waiting for {expression}"))
}
async fn run(app: &AppHandle) -> Result<Value, String> {
    let owner = "window-1";
    let shell = app
        .get_webview(&browser::shell_label(owner))
        .ok_or("Shell unavailable")?;
    ready(
        &shell,
        "!!document.querySelector('[title=\"Toggle vertical tabs\"]') && !!window.aero",
    )
    .await?;
    let original = eval(&shell, "window.aero.getState()").await?;
    if original["tabs"].as_array().map(|s| s.len()) != Some(1) {
        return Err("Initial Rust tab state is invalid".into());
    }
    address_checks(app, &shell, owner).await?;
    eval(
        &shell,
        "document.querySelector('[title=\"Toggle vertical tabs\"]').click(); true",
    )
    .await?;
    ready(
        &shell,
        "document.querySelector('header button[draggable]')?.getBoundingClientRect().width > 200",
    )
    .await?;
    let width = eval(
        &shell,
        "document.querySelector('header button[draggable]').getBoundingClientRect().width",
    )
    .await?;
    if !(235.0..=241.0).contains(&width.as_f64().unwrap_or(0.)) {
        return Err("Horizontal tab sizing differs from Aero".into());
    }
    let second = browser::create_tab(app, owner, "https://aero-synthetic.test/second", false)?;
    browser::command(
        app,
        owner,
        "move-tab-to-pane",
        json!({"id":second,"pane":"right"}),
    )
    .await?;
    ready(
        &shell,
        "!!document.querySelector('[aria-label=\"Resize split panes\"]')",
    )
    .await?;
    browser::command(app, owner, "set-split-ratio", json!(0.65)).await?;
    browser::command(app, owner, "swap-panes", Value::Null).await?;
    let value = eval(&shell, "window.aero.getState()").await?;
    if value["leftTabId"] != second || value["rightTabId"] != 1 {
        return Err("Split migration or swapping differs from Aero".into());
    }
    eval(
        &shell,
        "document.querySelector('[title=\"Settings and more (Alt+F)\"]').click();true",
    )
    .await?;
    ready(
        &shell,
        "document.body.textContent.includes('Aero Settings')",
    )
    .await?;
    let caption=eval(&shell,"(()=>{const b=document.querySelector('[aria-label=\"Close\"]'),r=b.getBoundingClientRect();return document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)===b})()").await?;
    if caption != true {
        return Err("Settings cover the window controls".into());
    }
    settings_checks(app, &shell, owner).await?;
    eval(
        &shell,
        "document.querySelector('[aria-label=\"Close settings\"]').click();true",
    )
    .await?;
    browser::command(app, owner, "activate-tab", json!(second)).await?;
    let view = browser::active_view(app, owner)?;
    crate::platform::action(&view, "reload", 0.)?;
    ready(
        &view,
        "document.querySelector('h1')?.textContent === 'Synthetic login test'",
    )
    .await?;
    for _ in 0..40 {
        let observed = app
            .state::<Browser>()
            .windows
            .lock()
            .unwrap()
            .get(owner)
            .and_then(|w| w.tab())
            .and_then(|t| t.secure_context)
            .is_some();
        if observed {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let rect = eval(
        &view,
        "document.querySelector('button[type=submit]').getBoundingClientRect().toJSON()",
    )
    .await?;
    let x = rect["x"].as_f64().unwrap() + rect["width"].as_f64().unwrap() / 2.;
    let y = rect["y"].as_f64().unwrap() + rect["height"].as_f64().unwrap() / 2.;
    crate::platform::cdp(
        &view,
        "Input.dispatchMouseEvent",
        json!({"type":"mousePressed","x":x,"y":y,"button":"left","clickCount":1}),
    )
    .await?;
    crate::platform::cdp(
        &view,
        "Input.dispatchMouseEvent",
        json!({"type":"mouseReleased","x":x,"y":y,"button":"left","clickCount":1}),
    )
    .await?;
    for _ in 0..30 {
        if app
            .state::<Browser>()
            .windows
            .lock()
            .unwrap()
            .get(owner)
            .unwrap()
            .pending
            .is_some()
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .unwrap()
        .pending
        .is_none()
    {
        return Err("Explicit trusted login submission did not produce a save offer".into());
    }
    ready(
        &shell,
        "document.body.textContent.includes('Save or update password for')",
    )
    .await?;
    eval(
        &shell,
        "[...document.querySelectorAll('button')].find(b=>b.textContent==='Not now').click();true",
    )
    .await?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    eval(&view,"document.querySelector('form').dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));true").await?;
    if app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .unwrap()
        .pending
        .is_some()
    {
        return Err("A scripted login submission produced a save offer".into());
    }
    let denied=eval(&view,"(async()=>{try{await window.__TAURI_INTERNALS__.invoke('aero_request',{method:'password-manager:list',payload:null});return false}catch{return true}})()").await?;
    if denied != true {
        return Err("A webpage could access vault IPC".into());
    }
    eval(&view,"window.audio=new AudioContext();window.osc=audio.createOscillator();osc.connect(audio.destination);osc.start();audio.resume();true").await?;
    for _ in 0..30 {
        let audible = app
            .state::<Browser>()
            .windows
            .lock()
            .unwrap()
            .get(owner)
            .and_then(|w| w.tab())
            .map(|t| t.audible)
            .unwrap_or(false);
        if audible {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if !app
        .state::<Browser>()
        .windows
        .lock()
        .unwrap()
        .get(owner)
        .unwrap()
        .tab()
        .unwrap()
        .audible
    {
        return Err(format!(
            "Native Web Audio indicator did not update: {}",
            eval(
                &view,
                "({state:audio.state,url:location.href,visibility:document.visibilityState})"
            )
            .await
            .unwrap_or(Value::Null)
        ));
    }
    browser::command(app, owner, "toggle-mute", json!(second)).await?;
    eval(&view, "osc.stop();audio.close();true").await?;
    // Test isolated DOM writes with synthetic values; never unlock or open a real vault.
    eval(&view,"const d=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value');Object.defineProperty(HTMLInputElement.prototype,'value',{get:d.get,set(v){window.intercepted=v;d.set.call(this,v)}});true").await?;
    let url = view.url().map_err(|e| e.to_string())?.to_string();
    let code = format!(
        "({})({})",
        include_str!("fill.js"),
        json!({"url":url,"origin":"https://aero-synthetic.test","username":"synthetic-fill-user","password":"synthetic-fill-password"})
    );
    if crate::platform::isolated(&view, code).await? != true {
        return Err("Isolated-world autofill failed".into());
    }
    let result=eval(&view,"({password:document.querySelector('[type=password]').value,intercepted:window.intercepted})").await?;
    if result["password"] != "synthetic-fill-password" || result.get("intercepted").is_some() {
        return Err("Webpage overrides intercepted isolated autofill".into());
    }
    eval(
        &view,
        "document.querySelector('form').style.opacity='0';true",
    )
    .await?;
    let code = format!(
        "({})({})",
        include_str!("fill.js"),
        json!({"url":url,"origin":"https://aero-synthetic.test","username":"synthetic-fill-user","password":"synthetic-fill-password"})
    );
    if crate::platform::isolated(&view, code).await? != false {
        return Err("Autofill accepted a hidden form".into());
    }
    let private = browser::window(app, true)?;
    let private_shell = app
        .get_webview(&browser::shell_label(&private))
        .ok_or("Private shell unavailable")?;
    ready(&private_shell, "!!window.aero").await?;
    if eval(&private_shell, "window.aero.getState()").await?["privateMode"] != true {
        return Err("Private browsing state did not reach the UI".into());
    }
    if crate::vault::open(app, &private).is_ok() {
        return Err("Private window could open a vault".into());
    }
    if eval(&private_shell,"window.aero.importSummary({source:'edge'}).then(()=>false,()=>true)").await? != true {
        return Err("Private window could import persistent browser data".into());
    }
    let private_view = browser::active_view(app, &private)?;
    crate::platform::action(&private_view,"reload",0.)?;
    ready(&private_view, "document.querySelector('h1')?.textContent==='Synthetic login test'").await?;
    eval(&private_view,"localStorage.setItem('aeroPrivateScope','keep');true").await?;
    eval(&view,"localStorage.setItem('aeroNormalScope','clear');true").await?;
    crate::browsing_data::clear(app,owner,json!({"timeRange":"all-time","startTime":0,"endTime":crate::browsing_data::now_ms()+1000,"types":["site-data"]})).await?;
    if eval(&private_view,"localStorage.getItem('aeroPrivateScope')==='keep'").await? != true
        || eval(&view,"localStorage.getItem('aeroNormalScope')===null").await? != true {
        return Err("Normal cleanup affected a private browsing profile".into());
    }
    let settings = eval(&shell, "window.aero.getSettings()").await?;
    if settings["appearance"]["verticalTabs"] != false {
        return Err("UI setting did not persist to Rust".into());
    }
    let mut spell_settings = app.state::<Browser>().settings.lock().unwrap().clone();
    spell_settings["content"]["spellcheck"] = json!(false);
    crate::settings::save(app, spell_settings.clone())?;
    ready(&view, "document.querySelector('[name=username]').spellcheck === false").await?;
    spell_settings["content"]["spellcheck"] = json!(true);
    crate::settings::save(app, spell_settings)?;
    ready(&view, "document.querySelector('[name=username]').spellcheck === true").await?;
    eval(&view, "localStorage.setItem('aeroSynthetic','value');true").await?;
    crate::platform::clear_data(&view).await?;
    if eval(&view, "localStorage.getItem('aeroSynthetic')").await? != Value::Null {
        return Err("Windows profile clearing retained synthetic site storage".into());
    }
    let mut clear_settings = app.state::<Browser>().settings.lock().unwrap().clone();
    clear_settings["privacy"]["clearOnExit"] = json!(true);
    crate::settings::save(app, clear_settings)?;
    // Locking with no authentication in flight must not recursively acquire
    // the model mutex. Exercise the same path as the caption minimize action.
    browser::command(app, owner, "minimize", Value::Null).await?;
    let window = app.get_window(owner).ok_or("Browser window unavailable")?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_position(tauri::LogicalPosition::new(-10000., -10000.)).map_err(|e| e.to_string())?;
    if app.state::<Browser>().windows.lock().unwrap().get(owner).unwrap().pending.is_some() {
        return Err("Minimizing did not revoke pending credentials".into());
    }
    Ok(
        json!({"passed":true,"checks":["shell RPC","shared Aero UI","multiword address typing","native address shortcut focus","persistent suggestion window","suggestion feedback prevention","rapid address refocus","Escape dismissal","typing without matches","suggestion selection navigation","address Enter navigation","horizontal widths","split migration","split resize","pane swap","settings caption controls","webpage IPC rejection","native Web Audio","mute","isolated autofill","prototype override resistance","hidden field rejection","private UI","private vault denial","settings persistence","trusted submission save offer","scripted submission rejection","minimize and vault locking","native browsing data clearing","clear on exit","spellcheck preference","password manager privacy layout","restore defaults privacy placement","import settings frame","automatic browser profile detection","profile chooser","selective browser import","import retry deduplication","imported history timestamps","partial browser import","password transfer guidance","stale import result cancellation","private import denial","clear data selection controls","native time-limited clearing","history time-range preservation","unselected data preservation","invalid clear request rejection","private profile cleanup isolation","minimum-width settings layout","theme selection styling"]}),
    )
}
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let result = run(&app).await;
        let profile = app.state::<Browser>().profile.clone();
        let value = match &result {
            Ok(value) => value.clone(),
            Err(error) => {
                let diagnostic = if let Some(shell) =
                    app.get_webview(&browser::shell_label("window-1"))
                {
                    eval(&shell,"({url:location.href,text:document.body?.innerText,api:typeof window.aero,viewport:[innerWidth,innerHeight],rect:document.querySelector('header button[draggable]')?.getBoundingClientRect().toJSON(),html:document.documentElement.outerHTML.slice(0,2000)})").await.unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                json!({"passed":false,"error":error,"diagnostic":diagnostic})
            }
        };
        let _ = std::fs::write(
            profile.join("smoke-report.json"),
            serde_json::to_vec_pretty(&value).unwrap(),
        );
        if result.is_ok() {
            // Exercise the real caption-close path rather than requesting process
            // exit while queued native minimize/restore events are still running.
            let owners = app.state::<Browser>().windows.lock().unwrap().keys().cloned().collect::<Vec<_>>();
            for owner in owners {
                if let Err(error) = browser::command(&app, &owner, "close-window", Value::Null).await {
                    eprintln!("Synthetic close failed: {error}");
                    app.exit(1);
                    return;
                }
            }
        } else {
            app.exit(1);
        }
    });
}

async fn address_checks(app: &AppHandle, shell: &tauri::Webview, owner: &str) -> Result<(), String> {
    use tauri::Emitter;
    for _ in 0..80 {
        if app.get_window(owner).is_some_and(|w|w.is_visible().unwrap_or(false)) { break; }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    // Move native focus within the test application's UI thread without making
    // its offscreen window the foreground application or taking the user's typing.
    shell.set_focus().map_err(|e|e.to_string())?;
    ready(shell, "document.hasFocus()").await?;
    // Wait for the shell's asynchronous event subscriptions, not just API creation.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    app.emit_to(browser::shell_label(owner), "browser:save-all-favorites",json!([{"title":"synthetic multiword search","url":"https://aero-synthetic.test/address-selected"}])).map_err(|e|e.to_string())?;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let rect=eval(shell, r#"(()=>{window.__addressCommands=[];window.__addressFocus=[];for(const name of ['focus','blur'])window.addEventListener(name,e=>window.__addressFocus.push({name,target:e.target?.tagName,time:performance.now()}),true);const command=window.aero.command;window.aero.command=(name,payload)=>{window.__addressCommands.push(name);return command(name,payload)};return document.querySelector('[aria-label="Address or search"]').getBoundingClientRect().toJSON()})()"#).await?;
    let x=rect["x"].as_f64().ok_or("Address rectangle unavailable")?+30.;
    let y=rect["y"].as_f64().ok_or("Address rectangle unavailable")?+rect["height"].as_f64().unwrap_or(32.)/2.;
    for event in ["mousePressed","mouseReleased"] {
        crate::platform::cdp(shell,"Input.dispatchMouseEvent",json!({"type":event,"x":x,"y":y,"button":"left","clickCount":1})).await?;
    }
    let mut popup_hwnd = None;
    for text in ["synthetic", " ", "multiword", " ", "search"] {
        crate::platform::cdp(shell,"Input.insertText",json!({"text":text})).await?;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        for _ in 0..40 {
            if app.get_webview_window(&format!("suggestions-{owner}")).is_some() { break; }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let popup=app.get_webview_window(&format!("suggestions-{owner}")).ok_or_else(||"Address suggestions did not open after typing".to_string())?;
        let hwnd=popup.hwnd().map_err(|e|e.to_string())?.0 as isize;
        if popup_hwnd.is_some_and(|previous|previous!=hwnd) { return Err("Typing recreated the suggestion window".into()); }
        popup_hwnd=Some(hwnd);
    }
    let value=eval(shell,r#"(()=>{const i=document.querySelector('[aria-label="Address or search"]');return {value:i.value,focused:document.activeElement===i,nativeFocus:document.hasFocus(),focusEvents:window.__addressFocus,commands:window.__addressCommands}})()"#).await?;
    std::fs::write(app.state::<Browser>().profile.join("address-focus-report.json"),serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if value["value"] != "synthetic multiword search" || value["focused"] != true || value["nativeFocus"] != true || value["focusEvents"].as_array().map(|a|a.len()).unwrap_or(1000)!=1 || value["commands"].as_array().map(|a|a.len()).unwrap_or(1000)>15 {
        return Err(format!("Address typing lost input/focus or entered a command loop: {value}"));
    }
    let popup=app.get_webview_window(&format!("suggestions-{owner}")).ok_or("Suggestions unavailable")?;
    let popup_view=app.get_webview(&format!("suggestions-{owner}")).ok_or("Suggestion webview unavailable")?;
    ready(&popup_view,"document.querySelector('a strong')?.textContent==='synthetic multiword search'").await?;
    // Repeated state notifications must not resend unchanged suggestion commands.
    let count=eval(shell,"window.__addressCommands.length").await?;
    for _ in 0..20 { browser::emit(app,owner); }
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    if eval(shell,"window.__addressCommands.length").await?!=count { return Err("Browser state notifications caused suggestion feedback".into()); }
    // A stale blur timer must not dismiss a refocused editor's suggestions.
    eval(shell,"(()=>{const i=document.querySelector('[aria-label=\"Address or search\"]');i.blur();i.focus();i.setSelectionRange(i.value.length,i.value.length);return true})()").await?;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    if !popup.is_visible().map_err(|e|e.to_string())? { return Err("Rapid refocus dismissed suggestions".into()); }
    crate::platform::cdp(shell,"Input.dispatchKeyEvent",json!({"type":"keyDown","key":"Escape","code":"Escape","windowsVirtualKeyCode":27})).await?;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    if popup.is_visible().map_err(|e|e.to_string())? { return Err("Escape retained suggestions".into()); }
    eval(shell,"(()=>{const i=document.querySelector('[aria-label=\"Address or search\"]');i.focus();i.select();return true})()").await?;
    crate::platform::cdp(shell,"Input.insertText",json!({"text":"no matching words at all"})).await?;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    if popup.is_visible().map_err(|e|e.to_string())? || eval(shell,"document.querySelector('[aria-label=\"Address or search\"]').value").await?!="no matching words at all" { return Err("Typing without suggestions lost input".into()); }
    eval(shell,"document.querySelector('[aria-label=\"Address or search\"]').select();true").await?;
    crate::platform::cdp(shell,"Input.insertText",json!({"text":"synthetic multiword"})).await?;
    ready(&popup_view,"document.querySelector('a strong')?.textContent==='synthetic multiword search'").await?;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    eval(&popup_view,"document.querySelector('a').click();true").await?;
    ready(shell,"window.aero.getState().then(s=>s.tabs.find(t=>t.id===s.activeTabId).url==='https://aero-synthetic.test/address-selected')").await?;
    eval(shell,"(()=>{const i=document.querySelector('[aria-label=\"Address or search\"]');i.focus();i.select();return true})()").await?;
    crate::platform::cdp(shell,"Input.insertText",json!({"text":"https://aero-synthetic.test/address-enter"})).await?;
    crate::platform::cdp(shell,"Input.dispatchKeyEvent",json!({"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13,"text":"\r"})).await?;
    ready(shell,"window.aero.getState().then(s=>s.tabs.find(t=>t.id===s.activeTabId).url==='https://aero-synthetic.test/address-enter')").await?;
    // Ctrl+L/F6 starts from a site view, so DOM focus alone is insufficient.
    browser::active_view(app,owner)?.set_focus().map_err(|e|e.to_string())?;
    browser::command(app,owner,"focus-address",Value::Null).await?;
    ready(shell,"(()=>{const i=document.querySelector('[aria-label=\"Address or search\"]');return document.hasFocus()&&document.activeElement===i&&i.selectionStart===0&&i.selectionEnd===i.value.length})()").await?;
    crate::platform::cdp(shell,"Input.insertText",json!({"text":"shortcut multiword search"})).await?;
    if eval(shell,"document.querySelector('[aria-label=\"Address or search\"]').value").await?!="shortcut multiword search" { return Err("Native address shortcut lost text".into()); }
    crate::platform::cdp(shell,"Input.dispatchKeyEvent",json!({"type":"keyDown","key":"Escape","code":"Escape","windowsVirtualKeyCode":27})).await?;
    Ok(())
}

async fn settings_checks(app: &AppHandle, shell: &tauri::Webview, owner: &str) -> Result<(), String> {
    if eval(shell,"document.querySelector('[data-aero-settings-section]').textContent.includes('Open Password Manager')").await? == true {
        return Err("Password Manager is still under Startup settings".into());
    }
    eval(shell,"[...document.querySelectorAll('.settings-nav')].find(b=>b.textContent==='Privacy & shields').click();true").await?;
    ready(shell,"!!document.querySelector('[data-aero-password-row]')").await?;
    let row = eval(shell,"(()=>{const row=document.querySelector('[data-aero-password-row]'),p=row.querySelector('small').getBoundingClientRect(),b=row.querySelector('button').getBoundingClientRect(),r=row.getBoundingClientRect();return {indent:p.x-r.x,inline:b.x>=p.right+12&&b.top<p.bottom&&p.top<b.bottom,reset:document.querySelector('[data-aero-settings-section]').textContent.includes('Restore defaults'),navReset:document.querySelector('nav').textContent.includes('Restore defaults')}})()").await?;
    if row["indent"].as_f64().unwrap_or(0.) < 16. || row["inline"] != true || row["reset"] != true || row["navReset"] != false {
        return Err(format!("Privacy settings layout is invalid: {row}"));
    }
    screenshot(app,shell,"settings-privacy.png").await?;
    let original_settings = app.state::<Browser>().settings.lock().unwrap().clone();
    let mut light_settings = original_settings.clone();
    light_settings["appearance"]["theme"] = json!("light");
    crate::settings::save(app,light_settings)?;
    let window = app.get_window(owner).ok_or("Window unavailable")?;
    window.set_size(tauri::LogicalSize::new(900.,700.)).map_err(|e|e.to_string())?;
    ready(shell,"innerWidth===900&&!document.documentElement.classList.contains('dark')").await?;
    if eval(shell,"(()=>{const row=document.querySelector('[data-aero-password-row]'),p=row.querySelector('small').getBoundingClientRect(),b=row.querySelector('button').getBoundingClientRect(),r=row.getBoundingClientRect();return p.x-r.x>=16&&b.x>=p.right+12&&b.top<p.bottom&&p.top<b.bottom})()").await? != true {
        return Err("Password Manager text/button stopped fitting at minimum window width".into());
    }
    if eval(shell,"getComputedStyle(document.querySelector('[data-aero-password-row]').closest('.settings-card')).backgroundColor==='rgb(255, 255, 255)'&&document.documentElement.style.colorScheme==='light'").await? != true {
        return Err("Light preference did not change actual rendered settings styles".into());
    }
    screenshot(app,shell,"settings-privacy-narrow-light.png").await?;
    window.set_size(tauri::LogicalSize::new(1400.,900.)).map_err(|e|e.to_string())?;
    crate::settings::save(app,original_settings)?;
    ready(shell,"innerWidth===1400").await?;

    eval(shell,"[...document.querySelectorAll('.settings-nav')].find(b=>b.textContent==='Import browser data').click();true").await?;
    ready(shell,"!!document.querySelector('[data-aero-settings-section=import]')").await?;
    if eval(shell,"(()=>{const items=[...document.querySelectorAll('.settings-nav')].map(b=>b.textContent);return items.indexOf('Import browser data')===items.indexOf('Performance')+1 && !!document.querySelector('[data-aero-settings-section=import] button') && document.querySelectorAll('nav').length===1})()").await? != true {
        return Err("Import did not stay in the settings frame below Performance".into());
    }
    screenshot(app,shell,"settings-import.png").await?;
    import_checks(app,shell).await?;
    eval(shell,"[...document.querySelectorAll('.settings-nav')].find(b=>b.textContent==='Privacy & shields').click();true").await?;
    ready(shell,"!!document.querySelector('[data-aero-clear-data]')").await?;
    eval(shell,"document.querySelector('[aria-label=\"Select all browsing data types\"]').click();true").await?;
    ready(shell,"[...document.querySelectorAll('[data-aero-clear-kind]')].every(b=>b.checked)").await?;
    eval(shell,"document.querySelector('[aria-label=\"Select all browsing data types\"]').click();true").await?;
    ready(shell,"[...document.querySelectorAll('[data-aero-clear-kind]')].every(b=>!b.checked)").await?;
    if eval(shell,"[...document.querySelectorAll('[data-aero-clear-data] button')].find(b=>b.textContent==='Clear selected data').disabled && document.querySelector('[aria-label=\"Clear browsing data time range\"]').options.length===5").await? != true {
        return Err("Cleanup empty selection or time presets are invalid".into());
    }
    let view = browser::active_view(app,owner)?;
    crate::platform::action(&view,"reload",0.)?;
    ready(&view,"document.querySelector('h1')?.textContent==='Synthetic login test'").await?;
    eval(&view,"localStorage.setItem('aeroSelective','keep');document.cookie='aeroSelectiveCookie=keep; Secure; SameSite=Lax; path=/';true").await?;
    // A past range must not silently clear recently written website storage.
    crate::browsing_data::clear(app,owner,json!({"timeRange":"last-year","startTime":1,"endTime":2,"types":["site-data","cookies"]})).await?;
    if eval(&view,"localStorage.getItem('aeroSelective')==='keep'&&document.cookie.includes('aeroSelectiveCookie=keep')").await? != true {
        return Err("Time-bounded native clearing removed data outside its range".into());
    }
    eval(shell,"(()=>{const now=Date.now(),midnight=new Date(now);midnight.setHours(0,0,0,0);localStorage.setItem('aero.history',JSON.stringify([{id:'synthetic-recent',title:'Synthetic recent',url:'https://aero-synthetic.test/recent',visitedAt:Math.floor((midnight.getTime()+now)/2)},{id:'synthetic-old',title:'Synthetic old',url:'https://aero-synthetic.test/old',visitedAt:now-400*86400000}]));document.querySelector('[data-aero-clear-kind=history]').click();return true})()").await?;
    eval(shell,"[...document.querySelectorAll('[data-aero-clear-data] button')].find(b=>b.textContent==='Clear selected data').click();true").await?;
    ready(shell,"document.querySelector('[data-aero-clear-data]').textContent.includes('Selected browsing data cleared.')").await?;
    if eval(shell,"(()=>{const h=JSON.parse(localStorage.getItem('aero.history'));return h.length===1&&h[0].id==='synthetic-old'})()").await? != true {
        return Err("History cleanup did not preserve entries outside Today".into());
    }
    if eval(&view,"localStorage.getItem('aeroSelective')==='keep'&&document.cookie.includes('aeroSelectiveCookie=keep')").await? != true {
        return Err("History-only cleanup removed unselected website data".into());
    }
    let now=crate::browsing_data::now_ms();
    crate::browsing_data::clear(app,owner,json!({"timeRange":"last-week","startTime":now.saturating_sub(60000),"endTime":now+1000,"types":["site-data"]})).await?;
    if eval(&view,"localStorage.getItem('aeroSelective')===null&&document.cookie.includes('aeroSelectiveCookie=keep')").await? != true {
        return Err("Website-storage selection did not preserve unselected cookies".into());
    }
    crate::browsing_data::clear(app,owner,json!({"timeRange":"all-time","startTime":0,"endTime":crate::browsing_data::now_ms()+1000,"types":["cookies"]})).await?;
    if eval(&view,"!document.cookie.includes('aeroSelectiveCookie=')").await? != true {
        return Err("Cookie selection did not clear synthetic cookies".into());
    }
    // Invalid payloads fail before calling Windows or changing app metadata.
    if crate::browsing_data::clear(app,owner,json!({"timeRange":"all-time","startTime":0,"endTime":now,"types":[]})).await.is_ok() {
        return Err("Empty clear-data selection was accepted".into());
    }
    Ok(())
}

async fn import_checks(app: &AppHandle, shell: &tauri::Webview) -> Result<(),String> {
    let base=app.state::<Browser>().profile.join("ImportSources/Local");
    let edge=base.join("Microsoft/Edge/User Data");
    for (folder,name) in [("Default","Personal"),("Profile 1","Work")] {
        let path=edge.join(folder);std::fs::create_dir_all(&path).map_err(|e|e.to_string())?;
        std::fs::write(path.join("Bookmarks"),json!({"roots":{"bookmark_bar":{"children":[{"name":"Synthetic imported bookmark","url":"https://aero-import.test/bookmark"}]}}}).to_string()).map_err(|e|e.to_string())?;
        let db=rusqlite::Connection::open(path.join("History")).map_err(|e|e.to_string())?;
        db.execute_batch("CREATE TABLE urls(url TEXT,title TEXT,last_visit_time INTEGER);INSERT INTO urls VALUES('https://aero-import.test/history','Synthetic imported history',13300000000000000);").map_err(|e|e.to_string())?;
        let _=name;
    }
    std::fs::write(edge.join("Local State"),json!({"profile":{"last_used":"Default","info_cache":{"Default":{"name":"Personal"},"Profile 1":{"name":"Work"}}}}).to_string()).map_err(|e|e.to_string())?;
    let chrome=base.join("Google/Chrome/User Data/Default");std::fs::create_dir_all(&chrome).map_err(|e|e.to_string())?;
    std::fs::write(chrome.join("Bookmarks"),json!({"roots":{"bookmark_bar":{"children":[{"name":"Synthetic recovery bookmark","url":"https://aero-import.test/recovery"}]}}}).to_string()).map_err(|e|e.to_string())?;
    std::fs::write(chrome.join("History"),"invalid sqlite").map_err(|e|e.to_string())?;
    eval(shell,"document.querySelector('[data-import-browser=edge]').click();true").await?;
    ready(shell,"document.querySelectorAll('[data-import-profile]').length===2").await?;
    let before=eval(shell,"localStorage.getItem('aero.history')").await?;
    eval(shell,"document.querySelector('[data-import-profile=Personal]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-import-apply]')").await?;
    if eval(shell,"document.querySelector('[data-aero-import]').textContent.includes('1 pages found')&&document.querySelector('[data-aero-import]').textContent.includes('1 found')").await? != true { return Err("Detected profile data did not reach the import UI".into()); }
    eval(shell,"document.querySelector('[data-import-kind=history]').click();document.querySelector('[data-import-apply]').click();true").await?;
    ready(shell,"document.querySelector('[data-aero-import]').textContent.includes('Added 1 bookmarks and 0 history pages')").await?;
    if eval(shell,"localStorage.getItem('aero.history')").await?!=before { return Err("Unchecked history was changed by import".into()); }
    if eval(shell,"JSON.parse(localStorage.getItem('aero.favorites')).some(x=>x.url==='https://aero-import.test/bookmark')").await?!=true { return Err("Selected bookmarks were not saved".into()); }
    eval(shell,"document.querySelector('[data-import-retry]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-import-apply]')").await?;
    eval(shell,"document.querySelector('[data-import-apply]').click();true").await?;
    ready(shell,"document.querySelector('[data-aero-import]').textContent.includes('Added 0 bookmarks and 1 history pages')").await?;
    if eval(shell,"JSON.parse(localStorage.getItem('aero.favorites')).filter(x=>x.url==='https://aero-import.test/bookmark').length===1&&JSON.parse(localStorage.getItem('aero.history')).some(x=>x.url==='https://aero-import.test/history'&&x.visitedAt===1655526400000)").await?!=true { return Err("Retry duplicated bookmarks or lost the imported timestamp".into()); }
    screenshot(app,shell,"import-complete.png").await?;
    eval(shell,"document.querySelector('[aria-label=\"Choose another browser\"]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-import-browser=chrome]')").await?;
    eval(shell,"document.querySelector('[data-import-browser=chrome]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-import-apply]')&&!!document.querySelector('[data-aero-import] [role=alert]')").await?;
    if eval(shell,"document.querySelector('[data-import-kind=history]').disabled&&!!document.querySelector('[data-import-password-guide]')&&!document.querySelector('[data-aero-import]').textContent.includes('Import complete')").await?!=true { return Err("Partial import or password guidance was misleading".into()); }
    screenshot(app,shell,"import-partial.png").await?;
    eval(shell,"document.querySelector('[data-import-open-passwords]').click();true").await?;
    ready(shell,"document.querySelector('[data-import-password-guide]').textContent.includes('disabled in synthetic tests')").await?;
    if eval(shell,"!!document.querySelector('[data-import-apply]')").await?!=true { return Err("Password help failure discarded the import preview".into()); }
    eval(shell,"document.querySelector('[aria-label=\"Choose another browser\"]').click();true").await?;
    // Close a UI operation before its promise completes; the old result must not reopen it.
    let db=rusqlite::Connection::open(edge.join("Default/History")).map_err(|e|e.to_string())?;
    db.execute_batch("PRAGMA locking_mode=EXCLUSIVE; BEGIN EXCLUSIVE; COMMIT;").map_err(|e|e.to_string())?;
    eval(shell,"document.querySelector('[data-import-browser=edge]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-import-profile=Personal]')").await?;
    eval(shell,"document.querySelector('[data-import-profile=Personal]').click();true").await?;
    ready(shell,"!!document.querySelector('[data-aero-import] .animate-spin')").await?;
    eval(shell,"document.querySelector('[aria-label=\"Choose another browser\"]').click();true").await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    if eval(shell,"!!document.querySelector('[data-import-browser=edge]')&&!document.querySelector('[data-import-apply]')").await?!=true { return Err("A stale import result replaced the browser picker".into()); }
    drop(db);
    Ok(())
}
async fn screenshot(app: &AppHandle, view: &tauri::Webview, name: &str) -> Result<(),String> {
    use base64::Engine;
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let value=crate::platform::cdp(view,"Page.captureScreenshot",json!({"format":"png"})).await?;
    let data=base64::engine::general_purpose::STANDARD.decode(value["data"].as_str().ok_or("Screenshot unavailable")?).map_err(|e|e.to_string())?;
    std::fs::write(app.state::<Browser>().profile.join(name),data).map_err(|e|e.to_string())
}
