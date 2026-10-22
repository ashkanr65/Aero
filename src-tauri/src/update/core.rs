use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_PACKAGE: u64 = 128 * 1024 * 1024;
pub const MAX_MANIFEST: u64 = 16 * 1024;
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub repository: String,
    pub public_key: String,
    pub channel: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub target: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
// Preserve the exact signed bytes, rather than reserializing JSON before verification.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub manifest: String,
    pub signature: String,
}
pub fn version(raw: &str) -> Result<semver::Version, String> {
    if raw.len() > 32 || !raw.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return Err("Invalid stable update version".into());
    }
    let v = semver::Version::parse(raw).map_err(|_| "Invalid stable update version")?;
    if v.to_string() != raw {
        return Err("Noncanonical update version".into());
    }
    Ok(v)
}
pub fn verify(config: &Config, record: &Record) -> Result<Manifest, String> {
    if record.manifest.len() as u64 > MAX_MANIFEST || record.signature.len() > 128 {
        return Err("Update metadata is too large".into());
    }
    let key: [u8; 32] = STANDARD
        .decode(&config.public_key)
        .map_err(|_| "Invalid update verification key")?
        .try_into()
        .map_err(|_| "Invalid update verification key")?;
    let signature = STANDARD
        .decode(record.signature.trim())
        .map_err(|_| "Invalid update signature")?;
    let signature =
        Signature::try_from(signature.as_slice()).map_err(|_| "Invalid update signature")?;
    VerifyingKey::from_bytes(&key)
        .map_err(|_| "Invalid update verification key")?
        .verify_strict(record.manifest.as_bytes(), &signature)
        .map_err(|_| "Update signature verification failed")?;
    let m: Manifest =
        serde_json::from_str(&record.manifest).map_err(|_| "Invalid update metadata")?;
    version(&m.version)?;
    if config.channel != "stable"
        || config.repository.split('/').count() != 2
        || !config
            .repository
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    {
        return Err("Invalid release configuration".into());
    }
    let expected = format!(
        "https://github.com/{}/releases/download/v{}/Aero-Rust-Portable-{}-x64.exe",
        config.repository, m.version, m.version
    );
    if m.schema != 1
        || m.target != "windows-x86_64"
        || m.url != expected
        || m.size < 256
        || m.size > MAX_PACKAGE
        || m.sha256.len() != 64
        || !m
            .sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("Update metadata has an unsupported target, size or destination".into());
    }
    Ok(m)
}
pub fn ordinary(path: &Path, directory: bool) -> bool {
    let Ok(m) = fs::symlink_metadata(path) else {
        return false;
    };
    if m.file_type().is_symlink() || m.is_dir() != directory {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}
pub fn directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir(path).map_err(|_| "The application folder is not writable")?;
    }
    if !ordinary(path, true) {
        return Err("Update directory is not a regular directory".into());
    }
    Ok(())
}
pub fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if !ordinary(path, false) {
        return Err("Update file is unavailable".into());
    }
    let f = fs::File::open(path).map_err(|_| "Update file is unavailable")?;
    let mut bytes = Vec::new();
    f.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Update file could not be read")?;
    if bytes.len() as u64 > limit {
        return Err("Update file is too large".into());
    }
    Ok(bytes)
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn verify_package(bytes: &[u8], manifest: &Manifest) -> Result<(), String> {
    if bytes.len() as u64 != manifest.size || hash(bytes) != manifest.sha256 {
        return Err("The update download is incomplete or changed".into());
    }
    let pe = bytes
        .get(0x3c..0x40)
        .and_then(|v| v.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0) as usize;
    if bytes.get(..2) != Some(b"MZ")
        || pe < 64
        || bytes.get(pe..pe + 4) != Some(b"PE\0\0")
        || bytes.get(pe + 4..pe + 6) != Some(&[0x64, 0x86])
        || bytes.get(pe + 24..pe + 26) != Some(&[0x0b, 0x02])
    {
        return Err("The update is not a Windows x64 application".into());
    }
    Ok(())
}
struct Partial(PathBuf);
impl Drop for Partial {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = Partial(path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4())));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp.0)
        .map_err(|_| "Cannot stage update metadata")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot save update metadata")?;
    drop(file);
    replace(&temp.0, path)
}
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            MoveFileExW(
                windows::core::PCWSTR(a.as_ptr()),
                windows::core::PCWSTR(b.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|_| "The update could not be committed".to_string())
    }
    #[cfg(not(windows))]
    {
        fs::rename(from, to).map_err(|_| "The update could not be committed".into())
    }
}
#[derive(Clone)]
pub struct Store {
    pub root: PathBuf,
    pub config: Config,
}
impl Store {
    pub fn new(application_root: &Path, config: Config) -> Result<Self, String> {
        let base = application_root
            .canonicalize()
            .map_err(|_| "Application folder is unavailable")?;
        let root = base.join(".aero-updates");
        directory(&root)?;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::os::windows::fs::MetadataExt;
            use windows::Win32::Storage::FileSystem::{
                FILE_FLAGS_AND_ATTRIBUTES, SetFileAttributesW,
            };
            if let Ok(metadata) = fs::metadata(&root) {
                let wide: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
                let _ = unsafe {
                    SetFileAttributesW(
                        windows::core::PCWSTR(wide.as_ptr()),
                        FILE_FLAGS_AND_ATTRIBUTES(metadata.file_attributes() | 2),
                    )
                };
            }
        }
        directory(&root.join("versions"))?;
        Ok(Self { root, config })
    }
    pub fn lock(&self) -> Result<fs::File, String> {
        let path = self.root.join("download.lock");
        if path.exists() && !ordinary(&path, false) {
            return Err("Invalid update lock".into());
        }
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| "Cannot lock the update folder")?;
        f.try_lock()
            .map_err(|_| "Another Aero process is checking for updates")?;
        Ok(f)
    }
    pub fn read_record(&self, name: &str) -> Option<Record> {
        let bytes = bounded(&self.root.join(name), MAX_MANIFEST * 2).ok()?;
        let r: Record = serde_json::from_slice(&bytes).ok()?;
        verify(&self.config, &r).ok()?;
        Some(r)
    }
    pub fn executable(&self, version: &str) -> Result<PathBuf, String> {
        self::version(version)?;
        let dir = self.root.join("versions").join(version);
        if !ordinary(&self.root, true)
            || !ordinary(&self.root.join("versions"), true)
            || !ordinary(&dir, true)
        {
            return Err("Cached update folder is unavailable".into());
        }
        Ok(dir.join("Aero.exe"))
    }
    pub fn cached(&self, record: &Record) -> Result<(Manifest, PathBuf), String> {
        let m = verify(&self.config, record)?;
        let path = self.executable(&m.version)?;
        let bytes = bounded(&path, MAX_PACKAGE)?;
        verify_package(&bytes, &m)?;
        Ok((m, path))
    }
    pub fn newest(&self, current: &str) -> Result<semver::Version, String> {
        let mut highest = version(current)?;
        for name in ["active.json", "previous.json"] {
            if let Some(r) = self.read_record(name) {
                let m = verify(&self.config, &r)?;
                highest = highest.max(version(&m.version)?);
            }
        }
        Ok(highest)
    }
    pub fn failed(&self) -> Option<String> {
        let b = bounded(&self.root.join("failed-version"), 64).ok()?;
        let text = String::from_utf8(b).ok()?;
        version(&text).ok()?;
        Some(text)
    }
    pub fn fail(&self, version: &str) {
        let _ = atomic(&self.root.join("failed-version"), version.as_bytes());
    }
    // Caller holds download.lock. Only the pointer is replaced; running executables stay intact.
    pub fn stage(
        &self,
        record: &Record,
        current: &str,
        mut input: impl Read,
    ) -> Result<bool, String> {
        let m = verify(&self.config, record)?;
        if !self.should_stage(record, current)? {
            return Ok(false);
        }
        let dir = self.root.join("versions").join(&m.version);
        directory(&dir)?;
        let temp = Partial(dir.join(format!("download-{}.part", uuid::Uuid::new_v4())));
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp.0)
            .map_err(|_| "Cannot stage the update download")?;
        let copied = std::io::copy(&mut input.by_ref().take(m.size + 1), &mut out)
            .map_err(|_| "Update download was interrupted")?;
        out.sync_all()
            .map_err(|_| "The update download could not be saved")?;
        drop(out);
        if copied != m.size {
            return Err("Update download was interrupted or has the wrong length".into());
        }
        verify_package(&bounded(&temp.0, MAX_PACKAGE)?, &m)?;
        let executable = dir.join("Aero.exe");
        // Never overwrite an existing cached executable, including another process's image.
        if executable.exists() {
            if verify_package(&bounded(&executable, MAX_PACKAGE)?, &m).is_err() {
                let bad = dir.join(format!("damaged-{}.exe", uuid::Uuid::new_v4()));
                fs::rename(&executable, &bad)
                    .map_err(|_| "Cannot isolate a damaged cached update")?;
                if fs::rename(&temp.0, &executable).is_err() {
                    let _ = fs::rename(&bad, &executable);
                    return Err("Cannot repair the cached update".into());
                }
                let _ = fs::remove_file(bad);
            }
        } else {
            fs::rename(&temp.0, &executable).map_err(|_| "Cannot commit the update application")?;
        }
        if let Some(old) = self.read_record("active.json") {
            if verify(&self.config, &old)?.version == current && self.cached(&old).is_ok() {
                atomic(
                    &self.root.join("previous.json"),
                    &serde_json::to_vec(&old).map_err(|_| "Invalid update record")?,
                )?;
            }
        }
        atomic(
            &self.root.join("active.json"),
            &serde_json::to_vec(record).map_err(|_| "Invalid update record")?,
        )?;
        Ok(true)
    }
    pub fn should_stage(&self, record: &Record, current: &str) -> Result<bool, String> {
        let m = verify(&self.config, record)?;
        let next = version(&m.version)?;
        if next <= version(current)?
            || next < self.newest(current)?
            || self.failed().as_deref() == Some(&m.version)
        {
            return Ok(false);
        }
        if let Some(active) = self.read_record("active.json") {
            if verify(&self.config, &active)?.version == m.version && self.cached(&active).is_ok() {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn choices(&self, current: &str) -> Vec<(Manifest, PathBuf)> {
        let mut entries = Vec::new();
        for name in ["active.json", "previous.json"] {
            if let Some(r) = self.read_record(name) {
                if let Ok((m, path)) = self.cached(&r) {
                    if version(&m.version).ok() > version(current).ok()
                        && self.failed().as_deref() != Some(&m.version)
                    {
                        entries.push((m, path));
                    }
                }
            }
        }
        entries.sort_by(|a, b| {
            version(&b.0.version)
                .unwrap()
                .cmp(&version(&a.0.version).unwrap())
        });
        entries.dedup_by(|a, b| a.0.version == b.0.version);
        entries
    }
    pub fn clean(&self, current: &str) {
        let mut keep = vec![current.to_string()];
        for name in ["active.json", "previous.json"] {
            if let Some(r) = self.read_record(name) {
                if let Ok(m) = verify(&self.config, &r) {
                    keep.push(m.version);
                }
            }
        }
        let base = self.root.join("versions");
        let Ok(entries) = fs::read_dir(&base) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if version(&name).is_err() || !ordinary(&path, true) {
                continue;
            }
            let Ok(files) = fs::read_dir(&path) else {
                continue;
            };
            for file in files.flatten() {
                let p = file.path();
                let name = file.file_name().to_string_lossy().to_string();
                let stale_part = name.starts_with("download-") && name.ends_with(".part");
                if ordinary(&p, false)
                    && (stale_part
                        || (name == "Aero.exe"
                            && !keep.contains(&entry.file_name().to_string_lossy().to_string())))
                {
                    let _ = fs::remove_file(p);
                }
            }
            if !keep.contains(&name) {
                let _ = fs::remove_dir(path);
            }
        }
    }
}

#[cfg(test)]
mod tests;

impl Store {
    pub fn launch_cached(
        &self,
        current: &str,
        original: &Path,
        args: &[std::ffi::OsString],
    ) -> bool {
        for (manifest, path) in self.choices(current) {
            if self.launch(original, &path, &manifest, args) {
                return true;
            }
            self.fail(&manifest.version);
        }
        false
    }
    fn launch(
        &self,
        original: &Path,
        application: &Path,
        manifest: &Manifest,
        args: &[std::ffi::OsString],
    ) -> bool {
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1).custom_flags(0x00200000);
        }
        let Ok(mut verified) = options.open(application) else {
            return false;
        };
        let mut bytes = Vec::new();
        if Read::by_ref(&mut verified)
            .take(MAX_PACKAGE + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || verify_package(&bytes, manifest).is_err()
        {
            return false;
        }
        drop(bytes);
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let marker = self.root.join(format!("health-{nonce}"));
        let mut cmd = std::process::Command::new(application);
        cmd.args(args)
            .env("AERO_LAUNCHER_PATH", original)
            .env("AERO_UPDATE_HEALTH", &nonce)
            .current_dir(original.parent().unwrap());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let Ok(mut child) = cmd.spawn() else {
            return false;
        };
        drop(verified);
        let start = std::time::Instant::now();
        loop {
            if bounded(&marker, 64).is_ok_and(|b| b == manifest.version.as_bytes()) {
                let _ = fs::remove_file(marker);
                return true;
            }
            if child.try_wait().ok().flatten().is_some() {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(45) {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = fs::remove_file(marker);
        false
    }
}
