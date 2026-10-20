use crate::{browser, model::Browser};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DataType { History, Downloads, Cookies, Cache, SiteData, Autofill }
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TimeRange { Today, LastWeek, LastMonth, LastYear, AllTime }
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearRequest {
    pub time_range: TimeRange,
    pub start_time: u64,
    pub end_time: u64,
    pub types: Vec<DataType>,
}
impl ClearRequest {
    pub fn validate(&self, now: u64) -> Result<(), String> {
        if self.types.is_empty() || self.types.len() > 6 || self.start_time > self.end_time || self.end_time > now.saturating_add(5000) {
            return Err("Choose valid data types and a valid time range".into());
        }
        if self.time_range == TimeRange::AllTime && self.start_time != 0 {
            return Err("All time must start at the beginning of the profile".into());
        }
        for (index, kind) in self.types.iter().enumerate() {
            if self.types[..index].contains(kind) { return Err("Duplicate browsing data types".into()); }
        }
        Ok(())
    }
    pub fn contains(&self, timestamp: u64) -> bool { timestamp >= self.start_time && timestamp < self.end_time }
    pub fn mask(&self) -> webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_BROWSING_DATA_KINDS {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        let mut mask = COREWEBVIEW2_BROWSING_DATA_KINDS(0);
        for kind in &self.types {
            mask |= match kind {
                DataType::History => COREWEBVIEW2_BROWSING_DATA_KINDS_BROWSING_HISTORY,
                DataType::Downloads => COREWEBVIEW2_BROWSING_DATA_KINDS_DOWNLOAD_HISTORY,
                DataType::Cookies => COREWEBVIEW2_BROWSING_DATA_KINDS_COOKIES,
                DataType::Cache => COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
                DataType::SiteData => COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_DOM_STORAGE,
                DataType::Autofill => COREWEBVIEW2_BROWSING_DATA_KINDS_GENERAL_AUTOFILL,
            };
        }
        mask
    }
}
pub fn same_scope(record: &Value, private_owner: Option<&str>) -> bool {
    record["privateOwner"].as_str() == private_owner
}
pub fn downloads_for(app: &AppHandle, owner: &str) -> Vec<Value> {
    let state = app.state::<Browser>();
    let private = state.windows.lock().unwrap().get(owner).map(|w| w.private).unwrap_or(true);
    let private_owner = private.then_some(owner);
    state.downloads.lock().unwrap().iter().filter(|record| same_scope(record, private_owner)).cloned().collect()
}
pub async fn clear(app: &AppHandle, owner: &str, value: Value) -> Result<Value, String> {
    let request: ClearRequest = serde_json::from_value(value).map_err(|_| "Invalid browsing data selection")?;
    request.validate(now_ms())?;
    let state = app.state::<Browser>();
    let private = state.windows.lock().unwrap().get(owner).ok_or("Browser window closed")?.private;
    let view = browser::active_view(app, owner)?;
    crate::platform::clear_selected(&view, &request).await?;
    if request.types.contains(&DataType::Downloads) {
        let private_owner = private.then_some(owner);
        state.downloads.lock().unwrap().retain(|record| {
            !same_scope(record, private_owner) || record["state"] == "progressing" || !request.contains(record["startedAt"].as_u64().unwrap_or(0))
        });
    }
    let owners = {
        let mut all = state.windows.lock().unwrap();
        all.iter_mut().filter(|(name, w)| if private { name.as_str() == owner } else { !w.private }).map(|(name, w)| {
            if request.types.contains(&DataType::History) { w.closed.retain(|(_, at)| !request.contains(*at)); }
            name.clone()
        }).collect::<Vec<_>>()
    };
    for name in owners {
        let _ = app.emit_to(browser::shell_label(&name), "browser:data-cleared", &request);
        if request.types.contains(&DataType::Downloads) {
            let _ = app.emit_to(browser::shell_label(&name), "browser:downloads", downloads_for(app, &name));
        }
    }
    serde_json::to_value(request).map_err(|_| "The browsing data result was invalid".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ClearRequest { ClearRequest { time_range: TimeRange::LastWeek, start_time: 100, end_time: 200, types: vec![DataType::Cache] } }
    #[test]
    fn range_includes_start_and_excludes_end() { let r = request(); assert!(!r.contains(99)); assert!(r.contains(100)); assert!(r.contains(199)); assert!(!r.contains(200)); }
    #[test]
    fn validates_selection_and_range() {
        let mut r = request(); assert!(r.validate(200).is_ok()); r.types.clear(); assert!(r.validate(200).is_err());
        r.types = vec![DataType::History, DataType::History]; assert!(r.validate(200).is_err());
        r.types = vec![DataType::History]; r.start_time = 201; assert!(r.validate(200).is_err());
        r.start_time = 0; r.end_time = 6000; assert!(r.validate(200).is_err());
        r.end_time = 200; r.time_range = TimeRange::AllTime; r.start_time = 1; assert!(r.validate(200).is_err());
    }
    #[test]
    fn masks_do_not_include_vault_or_settings() {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        let mut r = request(); r.types = vec![DataType::History,DataType::Downloads,DataType::Cookies,DataType::Cache,DataType::SiteData,DataType::Autofill];
        let mask = r.mask(); assert_eq!(mask.0 & COREWEBVIEW2_BROWSING_DATA_KINDS_PASSWORD_AUTOSAVE.0, 0);
        assert_eq!(mask.0 & COREWEBVIEW2_BROWSING_DATA_KINDS_SETTINGS.0, 0);
        r.types = vec![DataType::Cache]; assert_eq!(r.mask(), COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE);
    }
    #[test]
    fn private_download_scope_is_separate() {
        let normal = serde_json::json!({"privateOwner":null}); let private = serde_json::json!({"privateOwner":"window-2"});
        assert!(same_scope(&normal,None)); assert!(!same_scope(&private,None)); assert!(!same_scope(&normal,Some("window-2")));
        assert!(same_scope(&private,Some("window-2"))); assert!(!same_scope(&private,Some("window-3")));
    }
}
