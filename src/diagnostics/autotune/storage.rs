use std::path::PathBuf;

use super::presets::{PRESETS, PRESET_FILES};

pub const CUSTOM_DOMAINS_FILE: &str = "autotune_custom.txt";
pub const RESULTS_FILE: &str = "autotune_results.txt";

pub fn load_results_file() -> Option<String> {
    let path = crate::common::config::get_cache_dir().join(RESULTS_FILE);
    if path.exists() {
        std::fs::read_to_string(&path).ok()
    } else {
        None
    }
}

pub fn preset_domains_file_path(preset_idx: usize) -> PathBuf {
    let name = PRESET_FILES.get(preset_idx).copied().unwrap_or(CUSTOM_DOMAINS_FILE);
    crate::common::config::get_cache_dir().join(name)
}

pub fn load_domain_file(path: &std::path::Path) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_domains_for_preset(preset_idx: usize) -> Vec<String> {
    if preset_idx >= PRESETS.len() {
        return Vec::new();
    }
    let file_domains = load_domain_file(&preset_domains_file_path(preset_idx));
    if !file_domains.is_empty() {
        return file_domains;
    }
    PRESETS[preset_idx].domains.iter().map(|s| s.to_string()).collect()
}

pub fn ensure_domain_files() -> Result<(), String> {
    for (idx, preset) in PRESETS.iter().enumerate() {
        let path = preset_domains_file_path(idx);
        let header = rust_i18n::t!("domain_file_header_full").replace("{}", preset.name);
        ensure_domain_file(&path, &header, preset.domains)?;
    }
    crate::common::ttl::ensure_ttl_file()
}

fn ensure_domain_file(path: &std::path::Path, header: &str, defaults: &[&str]) -> Result<(), String> {
    if path.exists() && !load_domain_file(path).is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory '{}': {}", parent.display(), e))?;
    }
    let mut content = format!("# {}\n", header);
    for d in defaults {
        content.push_str(d);
        content.push('\n');
    }
    std::fs::write(path, content).map_err(|e| format!("Cannot write '{}': {}", path.display(), e))
}

