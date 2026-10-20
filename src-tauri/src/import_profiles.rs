use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Roots {
    pub local: PathBuf,
    pub roaming: PathBuf,
}
impl Roots {
    pub fn current() -> Result<Self, String> {
        Ok(Self {
            local: std::env::var_os("LOCALAPPDATA")
                .ok_or("Windows local profile directory is unavailable")?
                .into(),
            roaming: std::env::var_os("APPDATA")
                .ok_or("Windows roaming profile directory is unavailable")?
                .into(),
        })
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub channel: String,
    pub last_used: bool,
    pub path: PathBuf,
}
pub fn supported(source: &str) -> bool {
    ["edge", "chrome", "firefox", "opera"].contains(&source)
}
pub fn label(source: &str) -> &'static str {
    match source {
        "edge" => "Microsoft Edge",
        "chrome" => "Google Chrome",
        "firefox" => "Firefox",
        "opera" => "Opera",
        _ => "Browser",
    }
}
fn json_file(path: &Path) -> Option<Value> {
    if std::fs::metadata(path).ok()?.len() > 32 * 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
fn push(out: &mut Vec<Profile>, path: PathBuf, name: String, channel: &str, last_used: bool) {
    let Ok(path) = path.canonicalize() else {
        return;
    };
    if !path.is_dir() || out.iter().any(|p| p.path == path) {
        return;
    }
    out.push(Profile {
        id: path.to_string_lossy().into(),
        name,
        channel: channel.into(),
        last_used,
        path,
    });
}
fn chromium(root: PathBuf, channel: &str, out: &mut Vec<Profile>, opera: bool) {
    let state = json_file(&root.join("Local State")).unwrap_or(Value::Null);
    let last = state["profile"]["last_used"].as_str();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    let Ok(canonical) = root.canonicalize() else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name != "Default"
            && !name.starts_with("Profile ")
            && state["profile"]["info_cache"].get(&name).is_none()
        {
            continue;
        }
        let Ok(path) = entry.path().canonicalize() else {
            continue;
        };
        if !path.starts_with(&canonical) || !path.is_dir() {
            continue;
        }
        // Only profile metadata is read during detection, never saved credentials.
        let display = state["profile"]["info_cache"][&name]["name"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(&name)
            .to_string();
        push(out, path, display, channel, last == Some(name.as_str()));
    }
    if opera && (root.join("Bookmarks").is_file() || root.join("History").is_file()) {
        push(out, root, channel.into(), channel, true);
    }
}
pub fn ini(text: &str) -> Vec<(String, BTreeMap<String, String>)> {
    let mut sections = vec![];
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if line.starts_with(';') || line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            sections.push((name.to_string(), BTreeMap::new()));
        } else if let Some((key, value)) = line.split_once('=') {
            if let Some((_, values)) = sections.last_mut() {
                values.insert(key.trim().into(), value.trim().into());
            }
        }
    }
    sections
}
fn firefox(root: PathBuf, out: &mut Vec<Profile>) {
    let text = std::fs::read_to_string(root.join("profiles.ini")).unwrap_or_default();
    let installs = std::fs::read_to_string(root.join("installs.ini")).unwrap_or_default();
    let defaults = ini(&installs)
        .into_iter()
        .filter_map(|(_, s)| s.get("Default").cloned())
        .collect::<Vec<_>>();
    for (section, values) in ini(&text) {
        if !section.starts_with("Profile") {
            continue;
        }
        let Some(path) = values.get("Path") else {
            continue;
        };
        let full = if values.get("IsRelative").map(String::as_str) == Some("0") {
            PathBuf::from(path)
        } else {
            root.join(path)
        };
        let name = values
            .get("Name")
            .cloned()
            .unwrap_or_else(|| section.clone());
        push(
            out,
            full,
            name,
            "Firefox",
            defaults.contains(path) || values.get("Default").map(String::as_str) == Some("1"),
        );
    }
    if out.is_empty() {
        if let Ok(entries) = std::fs::read_dir(root.join("Profiles")) {
            for entry in entries.flatten() {
                if entry.path().join("places.sqlite").is_file() {
                    push(
                        out,
                        entry.path(),
                        entry.file_name().to_string_lossy().into(),
                        "Firefox",
                        false,
                    );
                }
            }
        }
    }
}
pub fn discover(roots: &Roots, source: &str) -> Result<Vec<Profile>, String> {
    if !supported(source) {
        return Err("Choose Edge, Chrome, Firefox, or Opera".into());
    }
    let mut out = vec![];
    match source {
        "edge" => {
            for (dir, name) in [
                ("Microsoft/Edge/User Data", "Edge"),
                ("Microsoft/Edge Beta/User Data", "Edge Beta"),
                ("Microsoft/Edge Dev/User Data", "Edge Dev"),
                ("Microsoft/Edge SxS/User Data", "Edge Canary"),
            ] {
                chromium(roots.local.join(dir), name, &mut out, false);
            }
        }
        "chrome" => {
            for (dir, name) in [
                ("Google/Chrome/User Data", "Chrome"),
                ("Google/Chrome Beta/User Data", "Chrome Beta"),
                ("Google/Chrome SxS/User Data", "Chrome Canary"),
            ] {
                chromium(roots.local.join(dir), name, &mut out, false);
            }
        }
        "opera" => {
            for name in [
                "Opera Stable",
                "Opera GX Stable",
                "Opera Developer",
                "Opera Beta",
            ] {
                chromium(
                    roots.roaming.join("Opera Software").join(name),
                    name,
                    &mut out,
                    true,
                );
            }
        }
        "firefox" => firefox(roots.roaming.join("Mozilla/Firefox"), &mut out),
        _ => unreachable!(),
    }
    out.sort_by(|a, b| {
        b.last_used
            .cmp(&a.last_used)
            .then(a.channel.cmp(&b.channel))
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });
    Ok(out)
}
pub fn select(profiles: Vec<Profile>, id: &str) -> Result<Profile, String> {
    profiles
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "That profile is no longer available. Detect profiles again.".into())
}
