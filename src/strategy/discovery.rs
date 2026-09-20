use crate::config::ZapretEngine;
use std::fs;

fn natural_key(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 8);
    let mut digits = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                out.push_str(&format!("{:0>8}", digits));
                digits.clear();
            }
            out.extend(ch.to_lowercase());
        }
    }
    if !digits.is_empty() {
        out.push_str(&format!("{:0>8}", digits));
    }
    out
}

fn zapret1_strategies() -> Vec<String> {
    let ws_dir = super::embedded::repo_dir();
    let mut strategies: Vec<String> = Vec::new();

    for dir in [ws_dir.clone(), ws_dir.join("custom-strategies")] {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.ends_with(".bat") && !strategies.contains(&name) {
                        strategies.push(name);
                    }
                }
            }
        }
    }

    strategies.sort();
    strategies
}

pub fn is_valid_preset_name(name: &str) -> bool {
    !name.starts_with('_') && name.to_lowercase().ends_with(".txt")
}

pub fn zapret2_presets() -> Vec<String> {
    let dir = ZapretEngine::Zapret2.presets_dir();
    let mut presets: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if !entry.path().is_file() {
                continue;
            }
            if let Ok(name) = entry.file_name().into_string() {
                if is_valid_preset_name(&name) {
                    presets.push(name);
                }
            }
        }
    }

    presets.sort_by_key(|n| natural_key(n));
    presets
}

pub fn get_strategies_for(engine: &ZapretEngine) -> Vec<String> {
    match engine {
        ZapretEngine::Zapret1 => zapret1_strategies(),
        ZapretEngine::Zapret2 => zapret2_presets(),
    }
}

pub fn get_strategies() -> Vec<String> {
    get_strategies_for(&crate::runner::active_engine())
}
