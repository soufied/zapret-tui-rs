
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::dns;
use crate::common::utils::find_in_path;
use crate::common::{config, logger};

const CONFIG_CURL: &str = "curl";
const CONFIG_IMPERSONATE: &str = "curl-impersonate";

const BINARIES: &[&str] = &["curl-impersonate", "curl-impersonate-chrome"];

const FALLBACK_PROFILES: &[&str] = &["chrome", "firefox"];

const REFERENCE_HOST: &str = "google.com";

const DISABLE_ENV: &str = "ZAPRET_NO_IMPERSONATE";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpClientKind {
    #[default]
    Curl,
    CurlImpersonate,
}

impl HttpClientKind {
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            CONFIG_IMPERSONATE | "impersonate" => Self::CurlImpersonate,
            _ => Self::Curl,
        }
    }

    pub fn as_config(self) -> &'static str {
        match self {
            Self::Curl => CONFIG_CURL,
            Self::CurlImpersonate => CONFIG_IMPERSONATE,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Curl => Self::CurlImpersonate,
            Self::CurlImpersonate => Self::Curl,
        }
    }
}

pub fn selected() -> HttpClientKind {
    HttpClientKind::from_config(&config::load_http_client())
}

pub fn impersonate_installed() -> bool {
    BINARIES.iter().any(|name| find_in_path(name).is_some())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impersonator {
    pub path: PathBuf,
    pub profile: String,
}

pub fn find_impersonator() -> Result<Impersonator, String> {
    if env::var_os(DISABLE_ENV).is_some() {
        return Err(format!(
            "curl-impersonate is disabled by the {} environment variable",
            DISABLE_ENV
        ));
    }

    let path = BINARIES
        .iter()
        .find_map(|name| find_in_path(name))
        .ok_or_else(|| "curl-impersonate was not found in PATH; install it or choose curl in Settings".to_string())?;

    let profiles = candidate_profiles(&path);
    let mut last_error = String::new();
    for profile in &profiles {
        match reference_probe(&path, profile) {
            Ok(()) => {
                logger::log_info(&format!("http client: {} --impersonate {}", path.display(), profile));
                return Ok(Impersonator {
                    path,
                    profile: profile.clone(),
                });
            }
            Err(e) => last_error = e,
        }
    }

    Err(format!(
        "curl-impersonate at {} accepted none of {} profiles (last error: {})",
        path.display(),
        profiles.len(),
        last_error
    ))
}

fn reference_probe(path: &Path, profile: &str) -> Result<(), String> {
    let pin = dns::pinned_v4(REFERENCE_HOST).ok_or_else(|| format!("no IPv4 address for {} (DNS)", REFERENCE_HOST))?;
    let url = format!("https://{}/", REFERENCE_HOST);

    let out = Command::new(path)
        .args([
            "--compressed",
            "--impersonate",
            profile,
            "-sS",
            "-k",
            "-o",
            null_device(),
            "-w",
            "%{http_code}",
            "--connect-timeout",
            "4",
            "--max-time",
            "6",
        ])
        .arg("--resolve")
        .arg(format!("{}:443:{}", REFERENCE_HOST, pin))
        .arg(&url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;

    let code = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && !code.is_empty() && code != "000" {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string();
    Err(format!(
        "{}: {}",
        profile,
        if stderr.is_empty() { code } else { stderr }
    ))
}

fn candidate_profiles(binary: &Path) -> Vec<String> {
    let mut found: Vec<(bool, u32, String)> = Vec::new();
    if let Some(dir) = binary.parent() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let Some(profile) = name.strip_prefix("curl_") else {
                    continue;
                };
                let is_chrome = profile.starts_with("chrome");
                if !is_chrome && !profile.starts_with("firefox") {
                    continue;
                }
                let digits: String = profile.chars().filter(|c| c.is_ascii_digit()).collect();
                let version = digits.parse::<u32>().unwrap_or(0);
                found.push((is_chrome, version, profile.to_string()));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    let mut profiles: Vec<String> = found.into_iter().map(|(_, _, name)| name).collect();
    for fallback in FALLBACK_PROFILES {
        if !profiles.iter().any(|p| p == fallback) {
            profiles.push(fallback.to_string());
        }
    }
    profiles
}

pub fn failed_phase(time_connect: f64) -> &'static str {
    if time_connect <= 0.0 {
        "TCP connect"
    } else {
        "TLS/HTTP after connect"
    }
}

pub fn null_device() -> &'static str {
    if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_phase_names_the_stage() {
        assert_eq!(failed_phase(0.0), "TCP connect");
        assert_eq!(failed_phase(0.042), "TLS/HTTP after connect");
    }

    #[test]
    fn config_values_round_trip() {
        assert_eq!(HttpClientKind::from_config("curl"), HttpClientKind::Curl);
        assert_eq!(
            HttpClientKind::from_config("curl-impersonate"),
            HttpClientKind::CurlImpersonate
        );
        assert_eq!(HttpClientKind::from_config(""), HttpClientKind::Curl);
        assert_eq!(HttpClientKind::from_config("garbage"), HttpClientKind::Curl);
        assert_eq!(HttpClientKind::CurlImpersonate.as_config(), "curl-impersonate");
        assert_eq!(HttpClientKind::Curl.toggle(), HttpClientKind::CurlImpersonate);
        assert_eq!(HttpClientKind::CurlImpersonate.toggle(), HttpClientKind::Curl);
    }
}
