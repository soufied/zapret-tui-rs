use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const FILE_NAME: &str = "installed.json";

fn path() -> PathBuf {
    crate::common::config::get_cache_dir().join(FILE_NAME)
}

fn load() -> BTreeMap<String, String> {
    fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn get(component: &str) -> Option<String> {
    load().remove(component)
}

pub fn record(component: &str, version: &str) {
    let mut map = load();
    map.insert(component.to_string(), version.to_string());
    let _ = fs::create_dir_all(crate::common::config::get_cache_dir());
    if let Ok(text) = serde_json::to_string_pretty(&map) {
        let _ = fs::write(path(), text);
    }
}
