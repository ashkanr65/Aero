use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_FILE: u64 = 512 * 1024 * 1024;
fn memory_copy(source: &Connection) -> Result<Connection, String> {
    let mut target =
        Connection::open_in_memory().map_err(|_| "Cannot prepare a history snapshot")?;
    {
        let backup =
            Backup::new(source, &mut target).map_err(|_| "History snapshot is unavailable")?;
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(2) {
                return Err("The source browser is locking its history".into());
            }
            match backup
                .step(256)
                .map_err(|_| "History snapshot could not be read")?
            {
                StepResult::Done => break,
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(25))
                }
                _ => {}
            }
        }
    }
    target
        .pragma_update(None, "trusted_schema", false)
        .map_err(|_| "History snapshot is unsupported")?;
    target
        .pragma_update(None, "temp_store", "MEMORY")
        .map_err(|_| "Cannot prepare history sorting")?;
    Ok(target)
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("aero-import-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).map_err(|_| "Cannot create a temporary history snapshot")?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn side(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    s.into()
}
fn stamp(path: &Path) -> Result<Option<(u64, std::time::SystemTime)>, String> {
    match fs::metadata(path) {
        Ok(m) if m.len() <= MAX_FILE => Ok(Some((
            m.len(),
            m.modified().map_err(|_| "Cannot inspect history file")?,
        ))),
        Ok(_) => Err("The history file exceeds the 512 MB snapshot limit".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Windows denied access to the source history file".into()),
    }
}
fn same(a: &Path, b: &Path) -> Result<bool, String> {
    let mut a = File::open(a).map_err(|_| "Source history cannot be read")?;
    let mut b = File::open(b).map_err(|_| "History snapshot cannot be read")?;
    let mut x = [0; 65536];
    let mut y = [0; 65536];
    loop {
        let nx = a
            .read(&mut x)
            .map_err(|_| "Source history changed while reading")?;
        let ny = b
            .read(&mut y)
            .map_err(|_| "History snapshot cannot be read")?;
        if nx != ny || x[..nx] != y[..ny] {
            return Ok(false);
        }
        if nx == 0 {
            return Ok(true);
        }
    }
}
// Used when a browser holds an exclusive SQLite lock. Copies are checked against
// their sources, include the WAL/journal, and are recovered/validated only in TEMP.
// This is a best-effort stable copy, not a transaction guarantee for an active writer.
fn locked_copy(path: &Path) -> Result<Connection, String> {
    for _ in 0..3 {
        let scratch = Scratch::new()?;
        let target = scratch.0.join("snapshot.sqlite");
        let paths = [
            ("", path.to_path_buf()),
            ("-wal", side(path, "-wal")),
            ("-journal", side(path, "-journal")),
        ];
        let before = paths
            .iter()
            .map(|(_, p)| stamp(p))
            .collect::<Result<Vec<_>, _>>()?;
        if before[0].is_none() {
            return Err("The source history file is missing".into());
        }
        for ((suffix, source), state) in paths.iter().zip(&before) {
            if state.is_some() {
                fs::copy(source,side(&target,suffix)).map_err(|_|"Windows is preventing a history snapshot. Close the source browser completely and retry.")?;
            }
        }
        let after = paths
            .iter()
            .map(|(_, p)| stamp(p))
            .collect::<Result<Vec<_>, _>>()?;
        let mut stable = before == after;
        if stable {
            for ((suffix, source), state) in paths.iter().zip(&before) {
                if state.is_some() && !same(source, &side(&target, suffix))? {
                    stable = false;
                    break;
                }
            }
        }
        if stable
            && paths
                .iter()
                .map(|(_, p)| stamp(p))
                .collect::<Result<Vec<_>, _>>()?
                == before
        {
            let snapshot =
                Connection::open(&target).map_err(|_| "History snapshot cannot be opened")?;
            snapshot
                .pragma_update(None, "trusted_schema", false)
                .map_err(|_| "History snapshot is unsupported")?;
            let valid: String = snapshot
                .query_row("PRAGMA quick_check(1)", [], |r| r.get(0))
                .map_err(
                    |_| "History snapshot is inconsistent. Close the source browser and retry.",
                )?;
            if valid != "ok" {
                return Err("History snapshot failed its integrity check. Close the source browser and retry.".into());
            }
            let memory = memory_copy(&snapshot)?;
            drop(snapshot);
            return Ok(memory);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("The source browser is updating history. Close it completely, then retry. Bookmarks can still be imported.".into())
}
pub fn read(path: &Path) -> Result<Connection, String> {
    if stamp(path)?.is_none() {
        return Err("No history database was found in this profile".into());
    }
    if let Ok(source) = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        let _ = source.busy_timeout(Duration::from_millis(100));
        let _ = source.pragma_update(None, "trusted_schema", false);
        if let Ok(snapshot) = memory_copy(&source) {
            return Ok(snapshot);
        }
    }
    locked_copy(path)
}
