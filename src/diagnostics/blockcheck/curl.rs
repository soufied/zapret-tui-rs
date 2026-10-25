
use std::process::{Command, Stdio};

use crate::common::utils::find_in_path;
use crate::diagnostics::dns;
use crate::diagnostics::http_client::{self, failed_phase, null_device, HttpClientKind};

const CONNECT_SECS: &str = "3";
const MAX_SECS: &str = "5";

#[derive(Debug, Clone)]
pub struct Curl {
    pub bin: String,
    pub version: String,
    pub profile: Option<String>,
    pub tls_max: bool,
    pub tls13: bool,
    pub http3: bool,
}

impl Curl {
    pub fn label(&self) -> String {
        match &self.profile {
            Some(profile) => format!("{} --impersonate {}", self.bin, profile),
            None => self.bin.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Http,
    Tls12,
    Tls13,
    Quic,
}

impl Kind {
    fn url(self, domain: &str) -> String {
        match self {
            Kind::Http => format!("http://{}/", domain),
            _ => format!("https://{}/", domain),
        }
    }

    fn port(self) -> u16 {
        match self {
            Kind::Http => 80,
            _ => 443,
        }
    }

    fn args(self, c: &Curl) -> Vec<&'static str> {
        match self {
            Kind::Http => Vec::new(),
            Kind::Tls12 if c.tls_max => vec!["--tlsv1.2", "--tls-max", "1.2"],
            Kind::Tls12 => vec!["--tlsv1.2"],
            Kind::Tls13 => vec!["--tlsv1.3", "--tls-max", "1.3"],
            Kind::Quic => vec!["--http3-only"],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Tok {
    Ok { ms: u64 },
    Fail { rc: i32, msg: String },
}

#[derive(Debug, Clone)]
pub struct DomainResult {
    pub probes: Vec<(Kind, Tok)>,
}

fn accepts(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn version_of(bin: &str) -> Option<String> {
    let out = Command::new(bin).arg("-V").stdin(Stdio::null()).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn describe(bin: String, profile: Option<String>) -> Result<Curl, String> {
    let version = version_of(&bin).ok_or_else(|| format!("{} did not report a version", bin))?;
    let http3 = version.contains("HTTP3") && accepts(&bin, &["--http3-only", "-V"]);
    Ok(Curl {
        tls_max: accepts(&bin, &["--tls-max", "1.2", "--tlsv1.2", "-V"]),
        tls13: accepts(&bin, &["--tlsv1.3", "--tls-max", "1.3", "-V"]),
        http3,
        version: version.lines().next().unwrap_or("").trim().to_string(),
        profile,
        bin,
    })
}

pub fn detect() -> Result<Curl, String> {
    match http_client::selected() {
        HttpClientKind::Curl => {
            let path = find_in_path("curl").ok_or_else(|| "curl not found in PATH; install curl".to_string())?;
            describe(path.to_string_lossy().into_owned(), None)
        }
        HttpClientKind::CurlImpersonate => {
            let imp = http_client::find_impersonator()?;
            describe(imp.path.to_string_lossy().into_owned(), Some(imp.profile))
        }
    }
}

pub fn probe(c: &Curl, kind: Kind, domain: &str) -> Tok {
    let Some(ip) = dns::pinned_v4(domain) else {
        return Tok::Fail {
            rc: -1,
            msg: "no IPv4 address from DNS".to_string(),
        };
    };
    let pin = format!("{}:{}:{}", domain, kind.port(), ip);

    let mut cmd = Command::new(&c.bin);
    if let Some(profile) = &c.profile {
        cmd.args(["--compressed", "--impersonate", profile.as_str()]);
    }
    let out = cmd
        .args([
            "-sS",
            "-k",
            "-o",
            null_device(),
            "-w",
            "%{http_code} %{time_connect} %{time_total}",
            "--connect-timeout",
            CONNECT_SECS,
            "--max-time",
            MAX_SECS,
            "--resolve",
        ])
        .arg(pin)
        .args(kind.args(c))
        .arg(kind.url(domain))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let out = match out {
        Ok(out) => out,
        Err(e) => {
            return Tok::Fail {
                rc: -1,
                msg: e.to_string(),
            }
        }
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut parts = stdout.split_whitespace();
    let code = parts.next().unwrap_or("000");
    let connect: f64 = parts.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
    let total: f64 = parts.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);

    if !code.is_empty() && code != "000" {
        return Tok::Ok {
            ms: (total * 1000.0).max(0.0) as u64,
        };
    }

    let detail = String::from_utf8_lossy(&out.stderr)
        .lines()
        .map(str::trim)
        .rev()
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .replace('|', "/");
    Tok::Fail {
        rc: out.status.code().unwrap_or(-1),
        msg: format!("{} failed: {}", failed_phase(connect), detail),
    }
}

pub fn check_domain(c: &Curl, domain: &str) -> DomainResult {
    let mut probes = vec![
        (Kind::Http, probe(c, Kind::Http, domain)),
        (Kind::Tls12, probe(c, Kind::Tls12, domain)),
    ];
    if c.tls13 {
        probes.push((Kind::Tls13, probe(c, Kind::Tls13, domain)));
    }
    if c.http3 {
        probes.push((Kind::Quic, probe(c, Kind::Quic, domain)));
    }
    DomainResult { probes }
}

pub fn control(c: &Curl) -> String {
    let out = Command::new(&c.bin)
        .args([
            "-sS",
            "-o",
            null_device(),
            "-w",
            "%{http_code}",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "https://www.cloudflare.com/cdn-cgi/trace",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match out {
        Ok(o) if o.status.success() => format!(
            "control: https://www.cloudflare.com/ answered {} with no bypass",
            String::from_utf8_lossy(&o.stdout).trim()
        ),
        Ok(o) => {
            let msg = String::from_utf8_lossy(&o.stderr)
                .lines()
                .map(str::trim)
                .rev()
                .find(|l| !l.is_empty())
                .unwrap_or("")
                .to_string();
            format!(
                "control: https://www.cloudflare.com/ failed, curl exit {}: {}",
                o.status.code().unwrap_or(-1),
                msg
            )
        }
        Err(e) => format!("control: could not run curl: {}", e),
    }
}
