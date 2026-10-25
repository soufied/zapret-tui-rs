
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::config::ZapretEngine;

pub const DEFAULT_DOMAINS: &[&str] = &["discord.com", "cdn.discordapp.com", "youtube.com", "googlevideo.com"];

const SERVICES: &[(&str, &str, &str)] = &[
    ("yt", "YouTube", "youtube.com youtu.be googlevideo.com ytimg.com"),
    (
        "ds",
        "Discord",
        "discord.com discord.gg discordapp.com cdn.discordapp.com gateway.discord.gg discord.media",
    ),
    ("ig", "Instagram", "instagram.com cdninstagram.com"),
    ("fb", "Facebook", "facebook.com fbcdn.net messenger.com"),
    ("x", "X (Twitter)", "x.com twitter.com twimg.com t.co"),
    ("tg", "Telegram", "telegram.org t.me web.telegram.org"),
    ("tt", "TikTok", "tiktok.com tiktokcdn.com"),
    ("sp", "Spotify", "spotify.com scdn.co"),
];

pub fn default_domains() -> Vec<String> {
    DEFAULT_DOMAINS.iter().map(|d| d.to_string()).collect()
}

pub fn clean_domain(raw: &str) -> Option<String> {
    let lowered = raw.trim().to_ascii_lowercase();
    let stripped = lowered.strip_prefix("*.").map(|r| r.to_string());
    let mut d = stripped.unwrap_or(lowered);
    d = d.trim_start_matches('.').to_string();
    if d.is_empty() || !d.contains('.') {
        return None;
    }
    if !d.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return None;
    }
    if !d.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
        return None;
    }
    if d.split('.').any(|label| label.is_empty()) {
        return None;
    }
    if d.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(d)
}

pub fn load_domains(path: &Path) -> Vec<String> {
    if !path.exists() {
        let body = format!("{}\n", DEFAULT_DOMAINS.join("\n"));
        let _ = fs::write(path, body);
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("");
        if let Some(d) = clean_domain(line) {
            if !out.contains(&d) {
                out.push(d);
            }
        }
    }
    if out.is_empty() {
        default_domains()
    } else {
        out
    }
}

pub fn service_domains(keys: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for key in keys {
        let entry = SERVICES
            .iter()
            .find(|(k, _, _)| *k == key.as_str())
            .ok_or_else(|| format!("unknown service: {} (known: {})", key, service_keys()))?;
        for d in entry.2.split_whitespace() {
            if !out.iter().any(|x| x == d) {
                out.push(d.to_string());
            }
        }
    }
    Ok(out)
}

pub fn service_keys() -> String {
    SERVICES
        .iter()
        .map(|(k, _, _)| *k)
        .collect::<Vec<_>>()
        .join(", ")
}

pub const USER_LIST: &str = "list-general-user.txt";
pub const CHECK_LIST: &str = "list-general-user-check.txt";

pub fn prepare_check_list(domains: &[String]) -> Result<PathBuf, String> {
    let lists = crate::strategy::repo_dir().join("lists");
    fs::create_dir_all(&lists).map_err(|e| format!("cannot create {}: {}", lists.display(), e))?;
    let mut text = fs::read_to_string(lists.join(USER_LIST)).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let existing: Vec<String> = text.lines().map(|l| l.trim().to_string()).collect();
    for d in domains {
        if !existing.iter().any(|l| l == d) {
            text.push_str(d);
            text.push('\n');
        }
    }
    let path = lists.join(CHECK_LIST);
    fs::write(&path, text).map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
    Ok(path)
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

fn pick(items: &[String], n: usize, rng: &mut Rng) -> Vec<String> {
    let mut v = items.to_vec();
    let n = n.min(v.len());
    for i in 0..n {
        let span = v.len() - i;
        let j = i + (rng.next_u64() as usize) % span;
        v.swap(i, j);
    }
    v.truncate(n);
    v
}

fn read_hostlist(name: &str) -> Option<Vec<String>> {
    let text = fs::read_to_string(crate::diagnostics::autotune::zapret2::lists_dir().join(name)).ok()?;
    let domains = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .filter_map(clean_domain)
        .collect();
    Some(domains)
}

pub fn preset_file(name: &str) -> Option<PathBuf> {
    [
        ZapretEngine::Zapret2.presets_dir().join(name),
        ZapretEngine::Zapret2.profiles_dir().join(name),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

pub struct Zapret2Targets {
    pub domains: Vec<String>,
    pub lists: usize,
    pub missing: Vec<String>,
}

pub fn zapret2_targets(preset: &str, per_list: usize, seed: u64) -> Zapret2Targets {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    let text = preset_file(preset)
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_default();
    let text = text.replace('\r', " ").replace('"', " ");

    for tok in text.split_whitespace() {
        if let Some(list) = tok.strip_prefix("--hostlist-domains=") {
            let group: Vec<String> = list.split(',').filter_map(clean_domain).collect();
            if !group.is_empty() {
                groups.push(group);
            }
        } else if let Some(file) = tok.strip_prefix("--hostlist=") {
            let name = file.rsplit('/').next().unwrap_or(file);
            match read_hostlist(name) {
                Some(group) if !group.is_empty() => groups.push(group),
                Some(_) => {}
                None => missing.push(name.to_string()),
            }
        }
    }

    let mut rng = Rng::new(seed);
    let mut domains: Vec<String> = Vec::new();
    for group in &groups {
        for d in pick(group, per_list, &mut rng) {
            if !domains.contains(&d) {
                domains.push(d);
            }
        }
    }

    Zapret2Targets {
        domains,
        lists: groups.len(),
        missing,
    }
}
