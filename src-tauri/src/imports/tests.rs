use super::*;
use std::fs;
use std::path::PathBuf;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("aero-import-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn roots(&self) -> Roots {
        Roots {
            local: self.0.join("Local"),
            roaming: self.0.join("Roaming"),
        }
    }
    fn chromium(&self, rel: &str, folder: &str, name: &str) -> PathBuf {
        let root = self.roots().local.join(rel);
        let path = root.join(folder);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            root.join("Local State"),
            json!({"profile":{"last_used":folder,"info_cache":{folder:{"name":name}}}}).to_string(),
        )
        .unwrap();
        fs::write(path.join("Bookmarks"),json!({"roots":{"bookmark_bar":{"children":[{"name":"Synthetic","url":"https://synthetic.test/"}]}}}).to_string()).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn history(path: &Path) -> Connection {
    let db = Connection::open(path).unwrap();
    db.execute_batch("CREATE TABLE urls(url TEXT,title TEXT,last_visit_time INTEGER); INSERT INTO urls VALUES('https://synthetic.test/history','Synthetic history',13300000000000000);").unwrap();
    db
}
#[test]
fn discovers_edge_display_name_and_actual_folder() {
    let f = Fixture::new();
    let path = f.chromium("Microsoft/Edge/User Data", "Profile 1", "Profile 2");
    let found = profiles::discover(&f.roots(), "edge").unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "Profile 2");
    assert!(found[0].last_used);
    assert_eq!(found[0].path, path.canonicalize().unwrap());
    let result = detect(&f.roots(), "edge", None).unwrap();
    assert_eq!(result["stage"], "ready");
    assert_eq!(result["bookmarks"].as_array().unwrap().len(), 1);
    assert_eq!(result["bookmarksImported"], 0);
}
#[test]
fn multiple_profiles_require_choice_and_reject_unknown_paths() {
    let f = Fixture::new();
    f.chromium("Google/Chrome/User Data", "Default", "Personal");
    f.chromium("Google/Chrome/User Data", "Profile 1", "Work");
    let result = detect(&f.roots(), "chrome", None).unwrap();
    assert_eq!(result["stage"], "choose-profile");
    assert_eq!(result["profiles"].as_array().unwrap().len(), 2);
    assert!(detect(&f.roots(), "chrome", Some("C:/Windows")).is_err());
    assert!(detect(&f.roots(), "unsupported", None).is_err());
}
#[test]
fn firefox_relative_and_absolute_profiles_and_install_default() {
    let f = Fixture::new();
    let root = f.roots().roaming.join("Mozilla/Firefox");
    fs::create_dir_all(root.join("Profiles/main")).unwrap();
    let other = f.0.join("custom-firefox");
    fs::create_dir(&other).unwrap();
    fs::write(root.join("profiles.ini"),format!("[Profile0]\nName=Personal\nIsRelative=1\nPath=Profiles/main\n[Profile1]\nName=Work\nIsRelative=0\nPath={}\n",other.display())).unwrap();
    fs::write(
        root.join("installs.ini"),
        "[InstallABC]\nDefault=Profiles/main\n",
    )
    .unwrap();
    let found = profiles::discover(&f.roots(), "firefox").unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].name, "Personal");
    assert!(found[0].last_used);
}
#[test]
fn opera_supports_root_and_nested_profiles() {
    let f = Fixture::new();
    let root = f.roots().roaming.join("Opera Software/Opera Stable");
    fs::create_dir_all(root.join("Default")).unwrap();
    fs::write(root.join("Bookmarks"), "{}").unwrap();
    fs::write(root.join("Default/Bookmarks"), "{}").unwrap();
    let found = profiles::discover(&f.roots(), "opera").unwrap();
    assert_eq!(found.len(), 2);
}
#[test]
fn locked_history_import_preserves_source_and_bookmarks() {
    let f = Fixture::new();
    let path = f.chromium("Microsoft/Edge/User Data", "Profile 1", "Personal");
    let file = path.join("History");
    let db = history(&file);
    db.execute_batch("PRAGMA locking_mode=EXCLUSIVE; BEGIN EXCLUSIVE; COMMIT;")
        .unwrap();
    let before = fs::read(&file).unwrap();
    let result = read(&path, "edge").unwrap();
    assert_eq!(result["history"].as_array().unwrap().len(), 1);
    assert_eq!(result["bookmarks"].as_array().unwrap().len(), 1);
    assert_eq!(before, fs::read(&file).unwrap());
}
#[test]
fn wal_snapshot_includes_committed_uncheckpointed_rows() {
    let f = Fixture::new();
    let path = f.0.join("History");
    let db = history(&path);
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; INSERT INTO urls VALUES('https://synthetic.test/wal','WAL row',13300000000001000);").unwrap();
    let snap = crate::import_snapshot::read(&path).unwrap();
    let n: i64 = snap
        .query_row("SELECT COUNT(*) FROM urls", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}
#[test]
fn corrupt_history_does_not_discard_bookmarks() {
    let f = Fixture::new();
    let path = f.chromium("Microsoft/Edge/User Data", "Default", "Personal");
    fs::write(path.join("History"), "not sqlite").unwrap();
    let result = read(&path, "edge").unwrap();
    assert_eq!(result["stage"], "ready");
    assert_eq!(result["bookmarks"].as_array().unwrap().len(), 1);
    assert!(!result["problems"].as_array().unwrap().is_empty());
}
#[test]
fn damaged_bookmarks_do_not_discard_history_and_bad_schemes_are_filtered() {
    let f = Fixture::new();
    let path = f.chromium("Microsoft/Edge/User Data", "Default", "Personal");
    fs::write(path.join("Bookmarks"), "{invalid").unwrap();
    let db = history(&path.join("History"));
    db.execute_batch("INSERT INTO urls VALUES('javascript:alert(1)','Unsafe',13300000000000000); INSERT INTO urls VALUES('https://synthetic.test/null','Null',NULL);").unwrap();
    let result = read(&path, "edge").unwrap();
    assert_eq!(result["history"].as_array().unwrap().len(), 1);
    assert_eq!(result["problems"][0]["kind"], "bookmarks");
}
#[test]
fn firefox_bookmark_and_history_tables_are_read() {
    let f = Fixture::new();
    let db = Connection::open(f.0.join("places.sqlite")).unwrap();
    db.execute_batch("CREATE TABLE moz_places(id INTEGER,url TEXT,title TEXT,last_visit_date INTEGER);CREATE TABLE moz_bookmarks(fk INTEGER,title TEXT,type INTEGER);INSERT INTO moz_places VALUES(1,'https://synthetic.test/','Page',1700000000000000);INSERT INTO moz_bookmarks VALUES(1,'Favorite',1);").unwrap();
    let result = read(&f.0, "firefox").unwrap();
    assert_eq!(result["history"][0]["visitedAt"], 1700000000000i64);
    assert_eq!(result["bookmarks"][0]["title"], "Favorite");
}
#[test]
fn bookmarks_are_deduplicated_and_unsafe_links_are_rejected() {
    let mut out = vec![];
    bookmarks(
        &json!({"children":[{"url":"javascript:alert(1)"},{"url":"file:///secret"},{"url":"https://synthetic.test/","name":"Safe"},{"url":"https://synthetic.test/","name":"Duplicate"}]}),
        &mut out,
        0,
    );
    dedup(&mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0]["title"], "Safe");
}
#[test]
fn missing_and_empty_profiles_are_distinguished() {
    let f = Fixture::new();
    assert!(read(&f.0, "edge").is_err());
    let result = detect(&f.roots(), "edge", None).unwrap();
    assert_eq!(result["found"], false);
    fs::write(f.0.join("Bookmarks"), "{\"roots\":{}}").unwrap();
    assert_eq!(read(&f.0, "edge").unwrap()["stage"], "ready");
}
#[test]
fn browser_launch_uses_normal_windows_paths() {
    assert_eq!(
        command_path(Path::new(r"\\?\C:\Users\Test\Profile 1")),
        r"C:\Users\Test\Profile 1"
    );
    assert_eq!(
        command_path(Path::new(r"\\?\UNC\server\share\profile")),
        r"\\server\share\profile"
    );
}
#[test]
#[ignore = "Only run explicitly to diagnose the user's Edge import; prints counts only"]
fn edge_live_counts() {
    let roots = Roots::current().unwrap();
    let found = profiles::discover(&roots, "edge").unwrap();
    let profile = found.into_iter().find(|p| p.last_used).unwrap();
    let result = read_profile(profile, "edge").unwrap();
    println!(
        "{}",
        json!({"bookmarks":result["bookmarks"].as_array().map(Vec::len),"history":result["history"].as_array().map(Vec::len),"problems":result["problems"],"stage":result["stage"]})
    );
    assert_eq!(result["stage"], "ready");
    assert!(!result["history"].as_array().unwrap().is_empty());
}
