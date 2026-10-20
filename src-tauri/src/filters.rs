thread_local! { static EPOCH: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) }; }
use adblock::{
    engine::Engine,
    lists::{FilterSet, ParseOptions},
    request::Request,
};
use std::{cell::RefCell, path::Path};
static FILTER_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
thread_local! {static ENGINE:RefCell<Option<Engine>>=const{RefCell::new(None)};}
const FALLBACK: &str = "||doubleclick.net^\n||googlesyndication.com^\n||googleadservices.com^\n||google-analytics.com^\n||facebook.com/tr/*\n||hotjar.com^\n||sentry.io^";
pub fn check(profile: &Path, url: &str, source: &str, resource: i32) -> bool {
    ENGINE.with(|state| {
        let mut engine = state.borrow_mut();
        let generation = FILTER_EPOCH.load(std::sync::atomic::Ordering::Relaxed);
        if engine.is_none() || EPOCH.with(|epoch| epoch.get() != generation) {
            EPOCH.with(|epoch| epoch.set(generation));
            let mut set = FilterSet::new(true);
            set.add_filter_list(FALLBACK.to_string(), ParseOptions::default());
            let mut loaded = Engine::new_with_filter_set(set);
            if let Ok(bytes) = std::fs::read(profile.join("Shields/engine.bin")) {
                let _ = loaded.deserialize(&bytes);
            }
            *engine = Some(loaded);
        }
        let kind = match resource {
            1 => "document",
            2 => "stylesheet",
            3 => "script",
            4 => "image",
            5 => "media",
            6 => "font",
            7 | 8 => "xmlhttprequest",
            _ => "other",
        };
        Request::new(url, source, kind, "get")
            .map(|request| {
                engine
                    .as_ref()
                    .unwrap()
                    .check_network_request(&request)
                    .should_block()
            })
            .unwrap_or(false)
    })
}
pub fn refresh(profile: std::path::PathBuf) {
    std::thread::spawn(move || {
        let folder = profile.join("Shields");
        let cached = folder.join("engine.bin");
        if std::fs::metadata(&cached)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|t| t.as_secs() < 7 * 24 * 3600)
            .unwrap_or(false)
        {
            return;
        }
        let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
        else {
            return;
        };
        let mut lists = String::from(FALLBACK);
        for source in [
            "https://easylist.to/easylist/easylist.txt",
            "https://easylist.to/easylist/easyprivacy.txt",
        ] {
            let Ok(response) = client.get(source).send().and_then(|r| r.error_for_status()) else {
                return;
            };
            if response.content_length().unwrap_or(0) > 10 * 1024 * 1024 {
                return;
            }
            let Ok(text) = response.text() else { return };
            if text.len() > 10 * 1024 * 1024 || !text.starts_with('[') {
                return;
            }
            lists.push('\n');
            lists.push_str(&text);
        }
        let mut set = FilterSet::new(true);
        set.add_filter_list(lists, ParseOptions::default());
        let engine = Engine::new_with_filter_set(set);
        let bytes = engine.serialize();
        if std::fs::create_dir_all(&folder).is_ok() {
            let temp = folder.join("engine.pending");
            if std::fs::write(&temp, bytes).is_ok() {
                if crate::storage::replace(&temp, &cached).is_ok() {
                    FILTER_EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_engine_blocks_with_no_download() {
        let p = std::env::temp_dir().join("aero-no-such-filter-cache");
        assert!(check(
            &p,
            "https://doubleclick.net/ad.js",
            "https://example.test",
            3
        ));
        assert!(!check(
            &p,
            "https://example.test/app.js",
            "https://example.test",
            3
        ));
    }
}
