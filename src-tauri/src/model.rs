use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Instant};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub favicon: Option<String>,
    pub loading: bool,
    pub audible: bool,
    pub muted: bool,
    pub discarded: bool,
    #[serde(skip)]
    pub back: bool,
    #[serde(skip)]
    pub forward: bool,
    #[serde(skip)]
    pub last_active: Option<Instant>,
    #[serde(skip)]
    pub blocked: u64,
    #[serde(skip)]
    pub upgrades: u64,
    #[serde(skip)]
    pub secure_context: Option<i64>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone)]
pub struct Selection {
    pub id: String,
    pub tab: u32,
    pub url: String,
    pub origin: String,
    pub at: Instant,
}
pub struct Pending {
    pub tab: u32,
    pub origin: String,
    pub username: String,
    pub password: zeroize::Zeroizing<String>,
    pub at: Instant,
}
pub struct WindowModel {
    pub closing: bool,
    pub native_hwnd: isize,
    pub vault_hwnd: Option<isize>,
    pub suggestion_label: Option<String>,
    pub suggestion_items: Vec<(String, String)>,
    pub suggestion_generation: u64,
    pub private: bool,
    pub tabs: Vec<Tab>,
    pub next: u32,
    pub active: u32,
    pub left: u32,
    pub right: Option<u32>,
    pub ratio: f64,
    pub bounds: Bounds,
    pub zoom: u32,
    pub fullscreen: bool,
    pub modes: HashMap<String, String>,
    pub scripts: HashMap<String, bool>,
    pub closed: Vec<(String, u64)>,
    pub selection: Option<Selection>,
    pub pending: Option<Pending>,
    pub selected: Option<(String, Instant)>,
    pub auth_busy: bool,
    pub auth_hwnd: Option<isize>,
    pub epoch: u64,
    pub last_vault: Instant,
    pub clipboard: Option<(zeroize::Zeroizing<String>, Instant)>,
}
impl WindowModel {
    pub fn new(_label: String, private: bool) -> Self {
        Self {
            closing: false,
            native_hwnd: 0,
            vault_hwnd: None,
            suggestion_label: None,
            suggestion_items: Vec::new(),
            suggestion_generation: 0,
            private,
            tabs: vec![],
            next: 1,
            active: 0,
            left: 0,
            right: None,
            ratio: 0.5,
            bounds: Bounds {
                x: 0.,
                y: 84.,
                width: 1200.,
                height: 700.,
            },
            zoom: 100,
            fullscreen: false,
            modes: HashMap::new(),
            scripts: HashMap::new(),
            closed: vec![],
            selection: None,
            pending: None,
            selected: None,
            auth_busy: false,
            auth_hwnd: None,
            epoch: 0,
            last_vault: Instant::now(),
            clipboard: None,
        }
    }
    pub fn tab(&self) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == self.active)
    }
    pub fn move_tab(&mut self, id: u32, right: bool) -> bool {
        if !self.tabs.iter().any(|t| t.id == id) {
            return false;
        }
        if right {
            if self.right == Some(id) {
            } else {
                let previous = self.right;
                if self.left == id {
                    if let Some(other) =
                        previous.or_else(|| self.tabs.iter().find(|t| t.id != id).map(|t| t.id))
                    {
                        self.left = other
                    } else {
                        return false;
                    }
                }
                self.right = Some(id)
            }
        } else if self.left != id {
            let previous = self.left;
            self.left = id;
            if self.right == Some(id) {
                self.right = Some(previous)
            }
        }
        self.active = id;
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.last_active = Some(Instant::now())
        }
        self.selection = None;
        true
    }
    pub fn close_tab(&mut self, id: u32) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };
        let removed = self.tabs.remove(index);
        self.closed.insert(0, (removed.url, crate::browsing_data::now_ms()));
        self.closed.truncate(20);
        if self.right == Some(id) {
            self.right = None
        }
        if self.left == id {
            if let Some(tab) = self.tabs.iter().find(|t| Some(t.id) != self.right) {
                self.left = tab.id
            } else {
                self.left = self.right.take().unwrap_or(self.tabs[0].id)
            }
        }
        if self.active == id || self.right.is_none() {
            self.active = self.left
        }
        self.selection = None;
        true
    }
    pub fn reorder(&mut self, id: u32, before: Option<u32>) {
        if before == Some(id) {
            return;
        }
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            let tab = self.tabs.remove(i);
            let j = before
                .and_then(|b| self.tabs.iter().position(|t| t.id == b))
                .unwrap_or(self.tabs.len());
            self.tabs.insert(j, tab)
        }
    }
    pub fn snapshot(&self, settings: &Value) -> Value {
        let t = self.tab();
        let host = t
            .and_then(|t| url::Url::parse(&t.url).ok())
            .map(|u| u.host_str().unwrap_or("").to_string())
            .unwrap_or_default();
        json!({"tabs":self.tabs,"privateMode":self.private,"activeTabId":self.active,"leftTabId":self.left,"rightTabId":self.right,"splitTabId":self.right,"splitRatio":self.ratio,"canGoBack":t.map(|t|t.back).unwrap_or(false),"canGoForward":t.map(|t|t.forward).unwrap_or(false),"shieldMode":self.modes.get(&host).cloned().unwrap_or_else(||match settings["privacy"]["trackingLevel"].as_str(){Some("off")=>"disabled",Some("strict")=>"aggressive",_=>"standard"}.into()),"scriptBlocking":self.scripts.get(&host).copied().unwrap_or(false),"stats":{"blocked":t.map(|t|t.blocked).unwrap_or(0),"httpsUpgrades":t.map(|t|t.upgrades).unwrap_or(0),"bandwidthSaved":t.map(|t|t.blocked*25000).unwrap_or(0)},"zoomPercent":self.zoom,"fullScreen":self.fullscreen,"memory":{"discardedTabs":self.tabs.iter().filter(|t|t.discarded).count(),"activeViews":self.tabs.iter().filter(|t|!t.discarded).count()}})
    }
}
pub struct Browser {
    pub startup_gpu_disabled: bool,
    pub windows: Mutex<HashMap<String, WindowModel>>,
    pub settings: Mutex<Value>,
    pub profile: PathBuf,
    pub downloads: Mutex<Vec<Value>>,
}
pub fn origin(raw: &str) -> Option<String> {
    let u = url::Url::parse(raw).ok()?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return None;
    }
    Some(u.origin().ascii_serialization())
}
pub fn address(raw: &str, settings: &Value) -> String {
    let input = raw.trim();
    if input.is_empty() {
        return settings["startup"]["homePage"]
            .as_str()
            .unwrap_or("https://duckduckgo.com/")
            .into();
    }
    if let Ok(u) = url::Url::parse(input) {
        if ["https", "http", "about", "file", "view-source"].contains(&u.scheme()) {
            return input.into();
        }
    }
    if input.starts_with("localhost") || input.starts_with("127.0.0.1") {
        return format!("http://{input}");
    }
    if !input.contains(char::is_whitespace) && input.contains('.') {
        return format!("https://{input}");
    }
    let q = url::form_urlencoded::byte_serialize(input.as_bytes()).collect::<String>();
    let prefix = match settings["search"]["engine"].as_str() {
        Some("google") => "https://www.google.com/search?q=",
        Some("bing") => "https://www.bing.com/search?q=",
        Some("brave") => "https://search.brave.com/search?q=",
        Some("ecosia") => "https://www.ecosia.org/search?q=",
        _ => "https://duckduckgo.com/?q=",
    };
    format!("{prefix}{q}")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> WindowModel {
        let mut w = WindowModel::new("window-1".into(), false);
        w.tabs = (1..=3)
            .map(|id| Tab {
                id,
                url: format!("https://{id}.test"),
                ..Default::default()
            })
            .collect();
        w.left = 1;
        w.active = 1;
        w
    }
    #[test]
    fn split_move_swap_close() {
        let mut w = setup();
        assert!(w.move_tab(2, true));
        assert_eq!(w.right, Some(2));
        w.move_tab(1, true);
        assert_eq!(w.left, 2);
        assert_eq!(w.right, Some(1));
        w.close_tab(2);
        assert_ne!(Some(w.left), w.right);
        assert_eq!(w.tabs.len(), 2)
    }
    #[test]
    fn close_two_panes() {
        let mut w = setup();
        w.tabs.pop();
        w.move_tab(2, true);
        w.close_tab(1);
        assert_eq!(w.active, 2);
        assert_eq!(w.right, None)
    }
    #[test]
    fn reorder_preserves_identity() {
        let mut w = setup();
        w.reorder(3, Some(1));
        assert_eq!(w.tabs[0].id, 3);
        assert_eq!(w.active, 1)
    }
    #[test]
    fn secure_origins() {
        assert!(origin("http://test.com").is_none());
        assert!(origin("https://name:pass@test.com").is_none());
        assert_eq!(origin("https://test.com/path").unwrap(), "https://test.com")
    }
}
