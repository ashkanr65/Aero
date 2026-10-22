use super::*;
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(format!("aero-transfer-synthetic-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const CSV: &str = "name,url,username,password\nSynthetic,https://example.test/login,synthetic-user,synthetic-secret\n";
fn settle(s: &mut Session) {
    for c in s.candidates.values_mut() {
        c.unchanged_since = Instant::now() - STABLE_FOR;
    }
}
#[test]
fn browser_csv_bom_quotes_and_firefox_columns() {
    let p = parse("\u{feff}name,url,username,password\nTest,https://example.test/path,\"user,name\",\"synthetic,\"\"secret\"\"\"\n".as_bytes()).unwrap();
    assert_eq!(p.payload["entries"][0]["origin"], "https://example.test");
    assert_eq!(p.payload["entries"][0]["username"], "user,name");
    assert_eq!(p.payload["entries"][0]["password"], "synthetic,\"secret\"");
    assert!(parse(b"url,username,password,httpRealm,formActionOrigin,guid,timeCreated,timeLastUsed,timePasswordChanged\nhttps://example.test,u,synthetic,,,,,,\n").is_ok());
}
#[test]
fn invalid_rows_are_skipped_and_malformed_files_fail_closed() {
    let p = parse(format!("{CSV}Bad,http://insecure.test,u,p\nEmpty,https://example.test,u,\nBad,https://u:p@example.test,u,p\n").as_bytes()).unwrap();
    assert_eq!(p.skipped, 3);
    assert_eq!(p.payload["entries"].as_array().unwrap().len(), 1);
    assert!(parse(b"url,title\nhttps://example.test,hello").is_err());
    assert!(parse(b"url,username,password\nhttp://example.test,u,p").is_err());
    assert!(parse(format!("{CSV}only,two\n").as_bytes()).is_err());
    assert!(parse(&[255, 254]).is_err());
}
#[test]
fn row_and_file_size_limits() {
    let csv = format!(
        "url,username,password\n{}",
        "https://example.test,u,synthetic\n".repeat(5001)
    );
    assert!(parse(csv.as_bytes()).is_err());
    assert!(parse(&vec![b'x'; MAX_BYTES as usize + 1]).is_err());
}
#[test]
fn existing_exports_are_ignored_even_when_modified() {
    let d = Folder::new();
    let old = d.0.join("old.csv");
    fs::write(&old, CSV).unwrap();
    let mut s = Session::begin(d.0.clone(), "edge").unwrap();
    fs::write(old, format!("{CSV}\n")).unwrap();
    s.status().unwrap();
    settle(&mut s);
    assert!(s.status().unwrap()["files"].as_array().unwrap().is_empty());
}
#[test]
fn new_export_requires_stability_and_explicit_opaque_selection() {
    let d = Folder::new();
    let mut s = Session::begin(d.0.clone(), "edge").unwrap();
    let file = d.0.join("new.csv");
    fs::write(&file, CSV).unwrap();
    assert!(s.status().unwrap()["files"].as_array().unwrap().is_empty());
    settle(&mut s);
    let status = s.status().unwrap();
    let id = status["files"][0]["id"].as_str().unwrap().to_string();
    assert!(s.selected("wrong-session", &id).is_err());
    assert!(s.selected(&s.id, "../new.csv").is_err());
    assert_eq!(s.selected(&s.id, &id).unwrap(), file);
    assert!(!status.to_string().contains("synthetic-secret"));
    fs::write(&file, format!("{CSV}\n")).unwrap();
    assert!(s.selected(&s.id, &id).is_err());
    s.status().unwrap();
    settle(&mut s);
    assert_ne!(s.status().unwrap()["files"][0]["id"], id);
    fs::remove_file(file).unwrap();
    assert!(s.status().unwrap()["files"].as_array().unwrap().is_empty());
}
#[test]
fn multiple_candidates_and_expiration_are_explicit() {
    let d = Folder::new();
    let mut s = Session::begin(d.0.clone(), "firefox").unwrap();
    for name in ["one.csv", "two.CSV"] {
        fs::write(d.0.join(name), CSV).unwrap();
    }
    s.status().unwrap();
    settle(&mut s);
    assert_eq!(s.status().unwrap()["files"].as_array().unwrap().len(), 2);
    s.started = Instant::now() - LIFETIME;
    assert!(s.status().is_err());
    assert!(s.selected(&s.id, "any").is_err());
    assert!(Session::begin(d.0.clone(), "unsupported").is_err());
}
#[test]
fn file_reader_rejects_directories_empty_and_oversized_files() {
    let d = Folder::new();
    assert!(read_export(&d.0).is_err());
    let file = d.0.join("empty.csv");
    fs::write(&file, "").unwrap();
    assert!(read_export(&file).is_err());
    fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_len(MAX_BYTES + 1)
        .unwrap();
    assert!(read_export(&file).is_err());
    fs::write(&file, CSV).unwrap();
    let bytes = read_export(&file).unwrap();
    assert!(parse(&bytes).is_ok());
}
#[test]
fn wiping_clears_nested_sensitive_payloads() {
    let mut v = json!({"entries":[{"password":"synthetic-secret","username":"synthetic"}]});
    wipe(&mut v);
    assert_eq!(v["entries"][0]["password"], "");
    assert_eq!(v["entries"][0]["username"], "");
}
