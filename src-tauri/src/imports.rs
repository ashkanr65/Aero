use crate::import_profiles::{self as profiles, Profile, Roots};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{collections::HashSet, path::Path};
pub const LIMIT: usize = 100_000;
fn web_url(value: &str) -> bool {
    url::Url::parse(value)
        .is_ok_and(|u| ["http", "https"].contains(&u.scheme()) && u.host_str().is_some())
}
fn bookmarks(value: &Value, output: &mut Vec<Value>, depth: usize) {
    if depth > 64 || output.len() > LIMIT {
        return;
    }
    if let Some(url) = value["url"].as_str().filter(|s| web_url(s)) {
        output.push(json!({"url":url,"title":value["name"].as_str().unwrap_or(url)}));
    }
    if let Some(children) = value["children"].as_array() {
        for child in children {
            bookmarks(child, output, depth + 1);
        }
    }
    if let Some(roots) = value["roots"].as_object() {
        for child in roots.values() {
            bookmarks(child, output, depth + 1);
        }
    }
}
fn query(db: &Connection, sql: &str, history: bool) -> Result<Vec<Value>, String> {
    let mut statement = db
        .prepare(sql)
        .map_err(|_| "This browser's history/bookmark database format is not supported")?;
    let mut rows = statement
        .query([])
        .map_err(|_| "The history/bookmark snapshot could not be read")?;
    let mut out = vec![];
    while let Some(row) = rows
        .next()
        .map_err(|_| "The history/bookmark snapshot is damaged")?
    {
        let (Ok(url), Ok(title)) = (row.get::<_, String>(0), row.get::<_, String>(1)) else {
            continue;
        };
        if !web_url(&url) {
            continue;
        }
        if history {
            let at = row.get::<_, i64>(2).unwrap_or(0);
            if at > 0 {
                out.push(json!({"url":url,"title":title,"visitedAt":at}));
            }
        } else {
            out.push(json!({"url":url,"title":title}));
        }
        if out.len() > LIMIT {
            break;
        }
    }
    Ok(out)
}
fn dedup(items: &mut Vec<Value>) {
    let mut seen = HashSet::new();
    items.retain(|v| seen.insert(v["url"].as_str().unwrap_or("").to_string()));
}
pub fn read(folder: &Path, source: &str) -> Result<Value, String> {
    if !profiles::supported(source) {
        return Err("Choose a supported browser".into());
    }
    let mut favorites = vec![];
    let mut history = vec![];
    let mut problems = vec![];
    let mut found = false;
    if source == "firefox" {
        let path = folder.join("places.sqlite");
        found = path.is_file();
        match crate::import_snapshot::read(&path) {
            Ok(db) => {
                match query(
                    &db,
                    "SELECT CAST(p.url AS TEXT),COALESCE(b.title,p.title,p.url) FROM moz_bookmarks b JOIN moz_places p ON p.id=b.fk WHERE b.type=1 LIMIT 100001",
                    false,
                ) {
                    Ok(v) => favorites = v,
                    Err(e) => problems.push(json!({"kind":"bookmarks","message":e})),
                }
                match query(
                    &db,
                    "SELECT CAST(url AS TEXT),COALESCE(title,url),COALESCE(last_visit_date,0)/1000 FROM moz_places WHERE last_visit_date>0 ORDER BY last_visit_date DESC LIMIT 100001",
                    true,
                ) {
                    Ok(v) => history = v,
                    Err(e) => problems.push(json!({"kind":"history","message":e})),
                }
            }
            Err(e) => problems.push(json!({"kind":"profile","message":e})),
        }
    } else {
        let path = folder.join("Bookmarks");
        if path.is_file() {
            found = true;
            let result = std::fs::metadata(&path)
                .map_err(|_| "Bookmarks cannot be accessed")
                .and_then(|m| {
                    if m.len() <= 32 * 1024 * 1024 {
                        Ok(())
                    } else {
                        Err("Bookmarks exceed the 32 MB import limit")
                    }
                })
                .and_then(|_| {
                    std::fs::read(&path).map_err(|_| "Windows denied access to bookmarks")
                });
            match result { Ok(bytes)=>match serde_json::from_slice::<Value>(&bytes) { Ok(tree)=>bookmarks(&tree,&mut favorites,0),Err(_)=>problems.push(json!({"kind":"bookmarks","message":"The bookmarks file is damaged or has an unsupported format"})) },Err(e)=>problems.push(json!({"kind":"bookmarks","message":e})) }
        }
        let path = folder.join("History");
        if path.is_file() {
            found = true;
            match crate::import_snapshot::read(&path) {
                Ok(db) => match query(
                    &db,
                    "SELECT CAST(url AS TEXT),COALESCE(title,url),(COALESCE(last_visit_time,0)/1000)-11644473600000 FROM urls WHERE last_visit_time>0 ORDER BY last_visit_time DESC LIMIT 100001",
                    true,
                ) {
                    Ok(v) => history = v,
                    Err(e) => problems.push(json!({"kind":"history","message":e})),
                },
                Err(e) => problems.push(json!({"kind":"history","message":e})),
            }
        }
    }
    if !found {
        return Err(format!(
            "No {} bookmarks or history were found in this profile. Try another detected profile or choose a profile folder.",
            profiles::label(source)
        ));
    }
    for (name, items) in [("Bookmarks", &mut favorites), ("History", &mut history)] {
        if items.len() > LIMIT {
            items.truncate(LIMIT);
            problems.push(json!({"kind":"limit","message":format!("{name}: showing the first {LIMIT} supported entries. Older history or additional bookmarks were not read.")}));
        }
        dedup(items);
    }
    let ready = !favorites.is_empty() || !history.is_empty() || problems.is_empty();
    Ok(
        json!({"source":source,"found":true,"stage":if ready{"ready"}else{"failed"},"profileId":folder.to_string_lossy(),"message":if ready{"Profile detected and read. Choose what to add to Aero."}else{"This profile could not be read. Details are shown below; try again after closing the source browser completely."},"bookmarksImported":0,"historyImported":0,"passwordsImported":0,"tabsImported":0,"bookmarks":favorites,"history":history,"tabs":[],"problems":problems}),
    )
}
fn read_profile(profile: Profile, source: &str) -> Result<Value, String> {
    let mut result = read(&profile.path, source)?;
    result["profileId"] = json!(profile.id);
    result["profileName"] = json!(profile.name);
    result["channel"] = json!(profile.channel);
    Ok(result)
}
pub fn detect(roots: &Roots, source: &str, selected: Option<&str>) -> Result<Value, String> {
    let found = profiles::discover(roots, source)?;
    if let Some(id) = selected {
        return read_profile(profiles::select(found, id)?, source);
    }
    if found.len() == 1 {
        return read_profile(found.into_iter().next().unwrap(), source);
    }
    if found.is_empty() {
        return Ok(
            json!({"source":source,"stage":"failed","found":false,"message":format!("No {} profile was detected. Open that browser once, then retry. For a custom installation, choose its profile folder.",profiles::label(source)),"profiles":[],"problems":[]}),
        );
    }
    Ok(
        json!({"source":source,"stage":"choose-profile","found":true,"message":"Choose the profile you want to import.","profiles":found,"problems":[]}),
    )
}
fn command_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{tail}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
    }
}
fn open_password_settings(
    roots: &Roots,
    source: &str,
    selected: Option<&str>,
) -> Result<Value, String> {
    let found = profiles::discover(roots, source)?;
    let profile = if let Some(id) = selected {
        Some(profiles::select(found, id)?)
    } else {
        found.into_iter().next()
    };
    let channel = profile.as_ref().map(|p| p.channel.as_str()).unwrap_or("");
    let (rel, page) = match source {
        "edge" => (
            format!(
                "Microsoft/{}/Application/msedge.exe",
                match channel {
                    "Edge Beta" => "Edge Beta",
                    "Edge Dev" => "Edge Dev",
                    "Edge Canary" => "Edge SxS",
                    _ => "Edge",
                }
            ),
            "edge://wallet/passwords",
        ),
        "chrome" => (
            format!(
                "Google/{}/Application/chrome.exe",
                match channel {
                    "Chrome Beta" => "Chrome Beta",
                    "Chrome Canary" => "Chrome SxS",
                    _ => "Chrome",
                }
            ),
            "chrome://password-manager/settings",
        ),
        "firefox" => ("Mozilla Firefox/firefox.exe".into(), "about:logins"),
        "opera" => (
            format!(
                "{}/launcher.exe",
                if channel.contains("GX") {
                    "Opera GX"
                } else {
                    "Opera"
                }
            ),
            "opera://password-manager/settings",
        ),
        _ => return Err("Choose a supported browser".into()),
    };
    let mut bases = vec![roots.local.clone(), roots.local.join("Programs")];
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(path) = std::env::var_os(var) {
            bases.push(path.into());
        }
    }
    let exe=bases.into_iter().map(|b|b.join(&rel)).find(|p|p.is_file()).ok_or_else(||format!("{} could not be located. Open it manually and choose Passwords, then Export passwords.",profiles::label(source)))?;
    let mut command = std::process::Command::new(exe);
    if let Some(p) = profile {
        if source == "firefox" {
            command.arg("-profile").arg(command_path(&p.path));
        } else if p
            .path
            .file_name()
            .is_some_and(|n| n == "Default" || n.to_string_lossy().starts_with("Profile "))
        {
            if let (Some(parent), Some(name)) = (p.path.parent(), p.path.file_name()) {
                command
                    .arg(format!("--user-data-dir={}", command_path(parent)))
                    .arg(format!("--profile-directory={}", name.to_string_lossy()));
            }
        } else if source == "opera" {
            command.arg(format!("--user-data-dir={}", command_path(&p.path)));
        }
    }
    command.arg(page).spawn().map_err(
        |_| "The source browser could not be opened. Open its Passwords settings manually.",
    )?;
    Ok(json!({"opened":true}))
}
pub async fn import(app: &tauri::AppHandle, payload: Value) -> Result<Value, String> {
    use tauri::Manager;
    let source = payload["source"]
        .as_str()
        .filter(|s| profiles::supported(s))
        .ok_or("Choose a supported browser")?
        .to_string();
    let roots = if crate::smoke::enabled() {
        let base = app
            .state::<crate::model::Browser>()
            .profile
            .join("ImportSources");
        Roots {
            local: base.join("Local"),
            roaming: base.join("Roaming"),
        }
    } else {
        Roots::current()?
    };
    let action = payload["action"].as_str().unwrap_or("discover").to_string();
    if action == "browse" {
        let Some(folder) = rfd::AsyncFileDialog::new()
            .set_title(format!(
                "Choose {} profile folder",
                profiles::label(&source)
            ))
            .pick_folder()
            .await
        else {
            return Ok(Value::Null);
        };
        let path = folder.path().to_path_buf();
        return tauri::async_runtime::spawn_blocking(move || read(&path, &source))
            .await
            .map_err(|_| "Profile import failed")?;
    }
    let selected = payload["profileId"].as_str().map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || match action.as_str() {
        "discover" | "read" => detect(&roots, &source, selected.as_deref()),
        "password-settings" if crate::smoke::enabled() => {
            Err("Source browser launch is disabled in synthetic tests".into())
        }
        "password-settings" => open_password_settings(&roots, &source, selected.as_deref()),
        _ => Err("Unknown import action".into()),
    })
    .await
    .map_err(|_| "Profile import failed")?
}

#[cfg(test)]
mod tests;
