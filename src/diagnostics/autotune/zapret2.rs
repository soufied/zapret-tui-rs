
use crate::common::config::ZapretEngine;

use super::bundles::{is_domain_list};

pub const Z2_TARGETS: &[&str] = &["discord.com", "gateway.discord.gg", "youtube.com", "googlevideo.com"];

pub const Z2_MAX_TARGETS: usize = 12;

pub fn lists_dir() -> std::path::PathBuf {
    ZapretEngine::Zapret2.workspace_dir().join("lists")
}

pub fn all_list_files() -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    let Ok(entries) = std::fs::read_dir(lists_dir()) else {
        return files;
    };

    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('_') || !name.to_lowercase().ends_with(".txt") {
            continue;
        }
        files.push(name);
    }

    files.sort();
    files
}

pub fn available_lists() -> Vec<String> {
    all_list_files()
        .into_iter()
        .filter(|name| is_domain_list(name))
        .collect()
}

fn is_probeable_domain(line: &str) -> bool {
    if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
        return false;
    }
    if line.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    if line.contains('/') || line.contains(' ') || line.starts_with('*') {
        return false;
    }
    if line.ends_with(".invalid") {
        return false;
    }
    line.contains('.')
}

fn representativeness(domain: &str) -> (usize, usize) {
    (domain.matches('.').count(), domain.len())
}

pub fn domains_in_list(file: &str) -> Vec<String> {
    let dir = lists_dir();
    let Ok(content) = std::fs::read_to_string(dir.join(file)) else {
        return Vec::new();
    };

    let mut out: Vec<String> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim().trim_start_matches('.').to_lowercase();
        if !is_probeable_domain(&line) {
            continue;
        }
        if !out.contains(&line) {
            out.push(line);
        }
    }

    out.sort_by(|a, b| representativeness(a).cmp(&representativeness(b)).then(a.cmp(b)));
    out
}

pub fn domains_from_lists(files: &[String], limit: usize) -> Vec<String> {
    let buckets: Vec<Vec<String>> = files.iter().map(|file| domains_in_list(file)).collect();

    let mut out: Vec<String> = Vec::new();
    let deepest = buckets.iter().map(|b| b.len()).max().unwrap_or(0);

    for depth in 0..deepest {
        for bucket in &buckets {
            let Some(domain) = bucket.get(depth) else {
                continue;
            };
            if out.iter().any(|d| d == domain) {
                continue;
            }
            out.push(domain.clone());
            if out.len() >= limit {
                return out;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_parser_skips_ips_cidrs_and_comments() {
        assert!(is_probeable_domain("discord.com"));
        assert!(!is_probeable_domain("# comment"));
        assert!(!is_probeable_domain("1.1.1.1"));
        assert!(!is_probeable_domain("10.0.0.0/8"));
        assert!(!is_probeable_domain(""));
        assert!(!is_probeable_domain("localhost"));
        assert!(!is_probeable_domain(crate::strategy::refs::PLACEHOLDER_HOST));
    }

    #[test]
    fn representative_domains_prefer_apex_names() {
        let mut domains = vec![
            "cdn.discordapp.com".to_string(),
            "discord.com".to_string(),
            "discord-attachments-uploads-prd.storage.googleapis.com".to_string(),
        ];
        domains.sort_by(|a, b| representativeness(a).cmp(&representativeness(b)).then(a.cmp(b)));
        assert_eq!(domains[0], "discord.com");
    }
}
