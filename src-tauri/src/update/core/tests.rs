use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::io::Cursor;
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(format!("aero-updater-synthetic-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn config() -> Config {
    let key = SigningKey::from_bytes(&[42; 32]);
    Config {
        repository: "synthetic/Aero".into(),
        public_key: STANDARD.encode(key.verifying_key().to_bytes()),
        channel: "stable".into(),
    }
}
fn image() -> Vec<u8> {
    let mut b = vec![0; 512];
    b[..2].copy_from_slice(b"MZ");
    b[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
    b[128..132].copy_from_slice(b"PE\0\0");
    b[132..134].copy_from_slice(&[0x64, 0x86]);
    b[152..154].copy_from_slice(&[0x0b, 0x02]);
    b
}
fn record(v: &str, b: &[u8]) -> Record {
    let m = Manifest {
        schema: 1,
        version: v.into(),
        target: "windows-x86_64".into(),
        url: format!(
            "https://github.com/synthetic/Aero/releases/download/v{v}/Aero-Rust-Portable-{v}-x64.exe"
        ),
        size: b.len() as u64,
        sha256: hash(b),
    };
    signed(m)
}
fn signed(m: Manifest) -> Record {
    let manifest = serde_json::to_string(&m).unwrap();
    let sig = SigningKey::from_bytes(&[42; 32]).sign(manifest.as_bytes());
    Record {
        manifest,
        signature: STANDARD.encode(sig.to_bytes()),
    }
}
#[test]
fn rejects_unsigned_tampered_wrong_key_and_target() {
    let b = image();
    let r = record("1.0.5", &b);
    assert!(verify(&config(), &r).is_ok());
    let mut bad = r.clone();
    bad.manifest.push(' ');
    assert!(verify(&config(), &bad).is_err());
    bad = r.clone();
    bad.signature = STANDARD.encode([0; 64]);
    assert!(verify(&config(), &bad).is_err());
    let mut c = config();
    c.public_key = STANDARD.encode(SigningKey::from_bytes(&[7; 32]).verifying_key().to_bytes());
    assert!(verify(&c, &r).is_err());
    let mut m = verify(&config(), &r).unwrap();
    m.target = "linux-x86_64".into();
    assert!(verify(&config(), &signed(m)).is_err());
}
#[test]
fn rejects_insecure_or_unrelated_downloads() {
    let b = image();
    for u in [
        "http://github.com/synthetic/Aero/a.exe",
        "https://evil.test/a.exe",
        "https://github.com/other/Aero/releases/download/v1.0.5/Aero-Rust-Portable-1.0.5-x64.exe",
    ] {
        let mut m = verify(&config(), &record("1.0.5", &b)).unwrap();
        m.url = u.into();
        assert!(verify(&config(), &signed(m)).is_err());
    }
}
#[test]
fn stable_versions_are_strict_and_ordered() {
    for v in [
        "../1.0.5",
        "1.0.5-beta.1",
        "1.0.5+fake",
        "v1.0.5",
        "01.0.5",
        "1.0",
        "999999999999999999999999999999999999999",
    ] {
        assert!(version(v).is_err());
    }
    assert!(version("1.0.10").unwrap() > version("1.0.9").unwrap());
}
#[test]
fn verifies_payload_hash_length_and_windows_architecture() {
    let b = image();
    let m = verify(&config(), &record("1.0.5", &b)).unwrap();
    assert!(verify_package(&b, &m).is_ok());
    let mut changed = b.clone();
    changed[400] = 1;
    assert!(verify_package(&changed, &m).is_err());
    assert!(verify_package(&b[..500], &m).is_err());
    changed = b.clone();
    changed[132] = 0x4c;
    let r = record("1.0.5", &changed);
    assert!(verify_package(&changed, &verify(&config(), &r).unwrap()).is_err());
}
#[test]
fn interrupted_download_preserves_active_and_cleans_partial() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    let r = record("1.0.5", &b);
    s.stage(&r, "1.0.4", Cursor::new(&b)).unwrap();
    let before = fs::read(s.root.join("active.json")).unwrap();
    assert!(
        s.stage(&record("1.0.6", &b), "1.0.4", Cursor::new(&b[..200]))
            .is_err()
    );
    assert_eq!(fs::read(s.root.join("active.json")).unwrap(), before);
    assert!(
        fs::read_dir(s.root.join("versions/1.0.6"))
            .unwrap()
            .next()
            .is_none()
    );
}
#[test]
fn newer_versions_only_and_replayed_releases_do_not_downgrade() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    assert!(
        s.stage(&record("1.0.5", &b), "1.0.4", Cursor::new(&b))
            .unwrap()
    );
    for v in ["1.0.3", "1.0.4", "1.0.5"] {
        assert!(!s.stage(&record(v, &b), "1.0.4", Cursor::new(&b)).unwrap());
    }
    assert_eq!(s.choices("1.0.4")[0].0.version, "1.0.5");
}
#[test]
fn previous_working_release_is_retained_and_bad_startup_falls_back() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    s.stage(&record("1.0.5", &b), "1.0.4", Cursor::new(&b))
        .unwrap();
    s.stage(&record("1.0.6", &b), "1.0.5", Cursor::new(&b))
        .unwrap();
    assert_eq!(s.choices("1.0.4").len(), 2);
    s.fail("1.0.6");
    assert_eq!(s.choices("1.0.4")[0].0.version, "1.0.5");
    assert!(s.choices("1.0.5").is_empty());
}
#[test]
fn corrupted_cache_never_becomes_an_executable_choice() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    s.stage(&record("1.0.5", &b), "1.0.4", Cursor::new(&b))
        .unwrap();
    fs::write(s.executable("1.0.5").unwrap(), b"invalid").unwrap();
    assert!(s.choices("1.0.4").is_empty());
}
#[test]
fn multiple_instances_share_an_exclusive_download_lock() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let first = s.lock().unwrap();
    assert!(s.lock().is_err());
    drop(first);
    assert!(s.lock().is_ok());
}
#[test]
fn unknown_files_and_personal_profiles_are_never_cleaned() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    s.stage(&record("1.0.5", &b), "1.0.4", Cursor::new(&b))
        .unwrap();
    let unknown = s.root.join("versions/1.0.2");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("personal.txt"), "keep").unwrap();
    fs::create_dir(d.0.join("AeroData")).unwrap();
    fs::write(d.0.join("AeroData/settings.json"), "keep").unwrap();
    s.clean("1.0.4");
    assert_eq!(
        fs::read_to_string(unknown.join("personal.txt")).unwrap(),
        "keep"
    );
    assert_eq!(
        fs::read_to_string(d.0.join("AeroData/settings.json")).unwrap(),
        "keep"
    );
    assert!(s.cached(&s.read_record("active.json").unwrap()).is_ok());
}
#[test]
fn failed_pointer_commit_keeps_previous_metadata() {
    let d = Folder::new();
    let path = d.0.join("active.json");
    atomic(&path, b"original").unwrap();
    let blocked = d.0.join("blocked");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("keep"), "kept").unwrap();
    assert!(atomic(&blocked, b"replacement").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"original");
    assert_eq!(fs::read(blocked.join("keep")).unwrap(), b"kept");
    assert_eq!(fs::read_dir(&d.0).unwrap().count(), 2);
}

#[test]
fn corrupted_cached_update_is_downloaded_again_without_lowering_version() {
    let d = Folder::new();
    let s = Store::new(&d.0, config()).unwrap();
    let b = image();
    let r = record("1.0.5", &b);
    s.stage(&r, "1.0.4", Cursor::new(&b)).unwrap();
    fs::write(s.executable("1.0.5").unwrap(), b"damaged").unwrap();
    assert!(s.should_stage(&r, "1.0.4").unwrap());
    assert!(s.stage(&r, "1.0.4", Cursor::new(&b)).unwrap());
    assert_eq!(s.choices("1.0.4")[0].0.version, "1.0.5");
}
