use tauri::{AppHandle, Manager, Webview};
use webview2_com::{Microsoft::Web::WebView2::Win32::*, WebResourceRequestedEventHandler};
use windows::core::PWSTR;
fn hostname(raw: &str) -> String {
    url::Url::parse(raw)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default()
}
fn matches(host: &str, list: &[&str]) -> bool {
    list.iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}
pub fn blocked(
    url: &str,
    source: &str,
    resource: i32,
    settings: &serde_json::Value,
    mode: &str,
    scripts: bool,
) -> bool {
    if settings["privacy"]["trackingLevel"] == "off" || mode == "disabled" {
        return false;
    }
    if (scripts || settings["shields"]["blockScripts"] == true) && resource == 3 {
        return true;
    }
    let host = hostname(url);
    let category = if matches(
        &host,
        &[
            "doubleclick.net",
            "googlesyndication.com",
            "googleadservices.com",
            "adservice.google.com",
            "ads-twitter.com",
            "an.yandex.ru",
            "adtrafficquality.google",
        ],
    ) {
        Some("blockAdNetworks")
    } else if matches(
        &host,
        &[
            "google-analytics.com",
            "googletagmanager.com",
            "mc.yandex.ru",
            "mc.yandex.com",
            "metrika.yandex.ru",
            "metrika.yandex.com",
        ],
    ) {
        Some("blockAnalytics")
    } else if matches(
        &host,
        &[
            "hotjar.com",
            "hotjar.io",
            "mouseflow.com",
            "fullstory.com",
            "smartlook.com",
            "logrocket.com",
            "clarity.ms",
        ],
    ) {
        Some("blockSessionReplay")
    } else if matches(
        &host,
        &[
            "sentry.io",
            "sentry-cdn.com",
            "bugsnag.com",
            "rollbar.com",
            "raygun.com",
            "appsignal.com",
            "datadoghq.com",
        ],
    ) {
        Some("blockErrorMonitoring")
    } else {
        None
    };
    if let Some(category) = category {
        return settings["shields"][category] == true;
    }
    if hostname(source).ends_with("youtube.com")
        && settings["shields"]["blockYoutubeVideoAds"] == true
        && (url.contains("/pagead/")
            || url.contains("/api/stats/ads")
            || url.contains("/ptracking")
            || url.contains("/ad_break"))
    {
        return true;
    }
    mode == "aggressive"
        && ["/ads/", "/advert/", "/tracking/", "/telemetry/"]
            .iter()
            .any(|s| url.contains(s))
}
pub fn install(view: &Webview, app: AppHandle, owner: String, id: u32) {
    let _ = view.with_webview(move |native| {
        let environment = native.environment();
        let Ok(core) = (unsafe { native.controller().CoreWebView2() }) else {
            return;
        };
        let mut token = 0;
        let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let request = unsafe { args.Request()? };
                let mut raw = PWSTR::null();
                unsafe {
                    request.Uri(&mut raw)?;
                }
                let url = unsafe { raw.to_string() }?;
                unsafe {
                    windows::Win32::System::Com::CoTaskMemFree(Some(raw.0.cast()));
                }
                if let Some(html) = crate::smoke::fixture(&url) {
                    unsafe {
                        let stream =
                            windows::Win32::UI::Shell::SHCreateMemStream(Some(html.as_bytes()));
                        let response = environment.CreateWebResourceResponse(
                            stream.as_ref(),
                            200,
                            windows::core::w!("OK"),
                            windows::core::w!("Content-Type: text/html; charset=utf-8"),
                        )?;
                        args.SetResponse(&response)?;
                    }
                    return Ok(());
                }
                let mut resource = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                unsafe {
                    args.ResourceContext(&mut resource)?;
                }
                let state = app.state::<crate::model::Browser>();
                let settings = state.settings.lock().unwrap().clone();
                let (source, mode, scripts) = {
                    let all = state.windows.lock().unwrap();
                    let Some(w) = all.get(&owner) else {
                        return Ok(());
                    };
                    let source = w
                        .tabs
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.url.clone())
                        .unwrap_or_default();
                    let host = hostname(&source);
                    let mode = w.modes.get(&host).cloned().unwrap_or_else(|| {
                        if settings["privacy"]["trackingLevel"] == "strict" {
                            "aggressive"
                        } else {
                            "standard"
                        }
                        .into()
                    });
                    (source, mode, w.scripts.get(&host).copied().unwrap_or(false))
                };
                let headers = unsafe { request.Headers()? };
                if settings["privacy"]["blockThirdPartyCookies"] == true {
                    let first = hostname(&source);
                    let third = hostname(&url);
                    if first != third
                        && !first.ends_with(&format!(".{third}"))
                        && !third.ends_with(&format!(".{first}"))
                    {
                        unsafe {
                            let _ = headers.RemoveHeader(windows::core::w!("Cookie"));
                        }
                    }
                }
                if settings["privacy"]["upgradeHttps"] == true
                    && url.starts_with("http://")
                    && !["localhost", "127.0.0.1"].contains(&hostname(&url).as_str())
                {
                    let secure = format!("https://{}", &url[7..]);
                    let wide = secure.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
                    unsafe {
                        let _ = request.SetUri(windows::core::PCWSTR(wide.as_ptr()));
                    }
                }
                unsafe {
                    if settings["privacy"]["globalPrivacyControl"] == true {
                        headers.SetHeader(windows::core::w!("Sec-GPC"), windows::core::w!("1"))?;
                    }
                    if settings["privacy"]["doNotTrack"] == true {
                        headers.SetHeader(windows::core::w!("DNT"), windows::core::w!("1"))?;
                    }
                }
                let full_filter = settings["privacy"]["trackingLevel"] != "off"
                    && mode != "disabled"
                    && [
                        "blockAdNetworks",
                        "blockAnalytics",
                        "blockSessionReplay",
                        "blockErrorMonitoring",
                    ]
                    .iter()
                    .all(|key| settings["shields"][*key] == true)
                    && crate::filters::check(&state.profile, &url, &source, resource.0);
                if blocked(&url, &source, resource.0, &settings, &mode, scripts) || full_filter {
                    unsafe {
                        let response = environment.CreateWebResourceResponse(
                            None::<&windows::Win32::System::Com::IStream>,
                            403,
                            windows::core::w!("Blocked by Aero Shields"),
                            windows::core::w!("Content-Length: 0"),
                        )?;
                        args.SetResponse(&response)?;
                    }
                    if let Some(w) = state.windows.lock().unwrap().get_mut(&owner) {
                        if let Some(t) = w.tabs.iter_mut().find(|t| t.id == id) {
                            t.blocked += 1;
                        }
                    }
                    crate::browser::emit(&app, &owner);
                }
            }
            Ok(())
        }));
        unsafe {
            let _ = core.AddWebResourceRequestedFilter(
                windows::core::w!("*"),
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            );
            let _ = core.add_WebResourceRequested(&handler, &mut token);
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categories_and_whitelists() {
        let s = crate::settings::defaults();
        assert!(blocked(
            "https://doubleclick.net/ad",
            "https://example.test",
            3,
            &s,
            "standard",
            false
        ));
        assert!(!blocked(
            "https://example.test/index.js",
            "https://example.test",
            3,
            &s,
            "standard",
            false
        ));
        assert!(!blocked(
            "https://doubleclick.net/ad",
            "https://example.test",
            3,
            &s,
            "disabled",
            false
        ));
    }
}
