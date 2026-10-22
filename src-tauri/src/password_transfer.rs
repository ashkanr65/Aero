//! Export handoff only: never opens another browser's credential database.
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use zeroize::{Zeroize, Zeroizing};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const LIFETIME: Duration = Duration::from_secs(10 * 60);
const STABLE_FOR: Duration = Duration::from_millis(1200);

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: SystemTime,
    created: SystemTime,
}
fn stamp(path: &Path) -> Option<Stamp> {
    let m = fs::symlink_metadata(path).ok()?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() == 0 || m.len() > MAX_BYTES {
        return None;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return None;
        } // reparse point
    }
    Some(Stamp {
        len: m.len(),
        modified: m.modified().ok()?,
        created: m.created().ok()?,
    })
}
fn csv_paths(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for (i, entry) in fs::read_dir(folder)
        .map_err(|_| "Downloads cannot be read. Use Choose exported CSV instead.")?
        .enumerate()
    {
        if i >= 20_000 {
            return Err(
                "Downloads has too many files to watch. Use Choose exported CSV instead.".into(),
            );
        }
        let path = entry
            .map_err(|_| "Downloads changed. Try again or choose the exported CSV.")?
            .path();
        if path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("csv"))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}
struct Candidate {
    id: String,
    path: PathBuf,
    stamp: Stamp,
    unchanged_since: Instant,
}
pub struct Session {
    pub id: String,
    source: String,
    folder: PathBuf,
    started: Instant,
    baseline: HashSet<PathBuf>,
    candidates: HashMap<PathBuf, Candidate>,
}
impl Session {
    pub fn begin(folder: PathBuf, source: &str) -> Result<Self, String> {
        if !crate::import_profiles::supported(source) {
            return Err("Choose a supported browser".into());
        }
        let baseline = csv_paths(&folder)?.into_iter().collect();
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            folder,
            started: Instant::now(),
            baseline,
            candidates: HashMap::new(),
        })
    }
    pub fn expired(&self) -> bool {
        self.started.elapsed() >= LIFETIME
    }
    pub fn status(&mut self) -> Result<Value, String> {
        if self.expired() {
            return Err(
                "Password transfer expired. Start again when you are ready to export.".into(),
            );
        }
        let paths = csv_paths(&self.folder)?;
        self.candidates.retain(|p, _| paths.contains(p));
        for path in paths {
            if self.baseline.contains(&path) {
                continue;
            }
            let Some(current) = stamp(&path) else {
                self.candidates.remove(&path);
                continue;
            };
            if let Some(c) = self.candidates.get_mut(&path) {
                if c.stamp != current {
                    c.stamp = current;
                    c.id = uuid::Uuid::new_v4().to_string();
                    c.unchanged_since = Instant::now();
                }
            } else if self.candidates.len() < 20 {
                self.candidates.insert(
                    path.clone(),
                    Candidate {
                        id: uuid::Uuid::new_v4().to_string(),
                        path,
                        stamp: current,
                        unchanged_since: Instant::now(),
                    },
                );
            }
        }
        let mut files = self.candidates.values().filter(|c| c.unchanged_since.elapsed() >= STABLE_FOR).map(|c| json!({"id":c.id,"name":c.path.file_name().unwrap_or_default().to_string_lossy(),"bytes":c.stamp.len})).collect::<Vec<_>>();
        files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        Ok(
            json!({"active":true,"id":self.id,"source":crate::import_profiles::label(&self.source),"folder":self.folder,"files":files}),
        )
    }
    pub fn selected(&self, session: &str, candidate: &str) -> Result<PathBuf, String> {
        if self.id != session || self.expired() {
            return Err("This transfer is no longer active. Start again.".into());
        }
        let c = self
            .candidates
            .values()
            .find(|c| c.id == candidate)
            .ok_or("Choose a detected export again")?;
        if c.unchanged_since.elapsed() < STABLE_FOR || stamp(&c.path).as_ref() != Some(&c.stamp) {
            return Err(
                "The export changed. Wait for it to finish saving, then choose it again.".into(),
            );
        }
        Ok(c.path.clone())
    }
}

pub fn read_export(path: &Path) -> Result<Zeroizing<Vec<u8>>, String> {
    if stamp(path).is_none() {
        return Err("Choose a regular, nonempty CSV smaller than 8 MiB.".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(
        |_| "The export cannot be opened. Finish saving it in the source browser and retry.",
    )?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if file
            .metadata()
            .map_err(|_| "CSV unavailable")?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err("Choose a regular exported CSV file.".into());
        }
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "The export could not be read")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("CSV exceeds 8 MiB".into());
    }
    Ok(bytes)
}
pub fn wipe(value: &mut Value) {
    match value {
        Value::String(s) => s.zeroize(),
        Value::Array(a) => a.iter_mut().for_each(wipe),
        Value::Object(o) => o.values_mut().for_each(wipe),
        _ => {}
    }
}
pub struct Parsed {
    pub payload: Value,
    pub skipped: usize,
}
impl Drop for Parsed {
    fn drop(&mut self) {
        wipe(&mut self.payload);
    }
}
pub fn parse(bytes: &[u8]) -> Result<Parsed, String> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err("CSV exceeds 8 MiB".into());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "CSV must be UTF-8")?
        .trim_start_matches('\u{feff}');
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| "CSV headers are invalid")?
        .clone();
    let column = |names: &[&str]| {
        headers
            .iter()
            .position(|v| names.contains(&v.trim().to_lowercase().as_str()))
    };
    let site = column(&["url", "origin", "website", "login_uri"])
        .ok_or("CSV has no website column. Export passwords from the source browser first.")?;
    let user =
        column(&["username", "user", "login_username"]).ok_or("CSV has no username column")?;
    let secret = column(&["password", "login_password"]).ok_or("CSV has no password column")?;
    let mut parsed = Parsed {
        payload: json!({"entries":[]}),
        skipped: 0,
    };
    let entries = parsed.payload["entries"].as_array_mut().unwrap();
    for (i, row) in reader.records().enumerate() {
        if i >= 10_000 {
            return Err(
                "CSV has too many rows. Import at most 5,000 credentials at a time.".into(),
            );
        }
        let row = row.map_err(|_| "CSV is malformed; no passwords were imported")?;
        let u = row.get(user).unwrap_or("");
        let p = row.get(secret).unwrap_or("");
        if let Some(origin) = crate::model::origin(row.get(site).unwrap_or("")) {
            if !u.is_empty() && !p.is_empty() && u.len() <= 512 && p.len() <= 8192 {
                if entries.len() >= 5000 {
                    return Err("Import at most 5,000 credentials at a time.".into());
                }
                entries.push(json!({"origin":origin,"username":u,"password":p}));
                continue;
            }
        }
        parsed.skipped += 1;
    }
    if entries.is_empty() {
        return Err("No supported passwords were found. Only HTTPS sites with a username and password can be imported.".into());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests;
