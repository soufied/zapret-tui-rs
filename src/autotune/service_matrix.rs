use crate::autotune::domain_checks::{check_domain_tls, test_http};
use crate::autotune::net_checks::{resolve_domain, try_tcp_connect_domain};
use crate::autotune::types::{is_cancelled, CheckStatus};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const PROBE_REQUESTS: usize = 2;
const UDP_TIMEOUT: Duration = Duration::from_millis(1500);
const VOICE_PORT: u16 = 443;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Ok,
    Degraded,
    Blocked,
    Skipped,
}

impl ServiceStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            ServiceStatus::Ok => "OK",
            ServiceStatus::Degraded => "PARTIAL",
            ServiceStatus::Blocked => "BLOCKED",
            ServiceStatus::Skipped => "SKIP",
        }
    }

    pub fn is_usable(&self) -> bool {
        matches!(self, ServiceStatus::Ok | ServiceStatus::Degraded)
    }

    fn from_pair(primary: bool, secondary: bool) -> Self {
        match (primary, secondary) {
            (true, true) => ServiceStatus::Ok,
            (true, false) => ServiceStatus::Degraded,
            (false, true) => ServiceStatus::Degraded,
            (false, false) => ServiceStatus::Blocked,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceReport {
    pub service: String,
    pub status: ServiceStatus,
    pub detail: String,
    pub elapsed_ms: u128,
}

impl ServiceReport {
    fn new(service: &str, status: ServiceStatus, detail: impl Into<String>, started: Instant) -> Self {
        Self {
            service: service.to_string(),
            status,
            detail: detail.into(),
            elapsed_ms: started.elapsed().as_millis(),
        }
    }

    fn skipped(service: &str) -> Self {
        Self {
            service: service.to_string(),
            status: ServiceStatus::Skipped,
            detail: "cancelled by user".to_string(),
            elapsed_ms: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatrixRow {
    pub profile: String,
    pub youtube: ServiceStatus,
    pub discord_text: ServiceStatus,
    pub discord_voice: ServiceStatus,
    pub general: ServiceStatus,
    pub elapsed_ms: u128,
}

impl MatrixRow {
    pub fn unblocks_everything(&self) -> bool {
        self.youtube.is_usable() && self.discord_text.is_usable() && self.discord_voice.is_usable()
    }

    pub fn from_reports(profile: &str, reports: &[ServiceReport]) -> Self {
        let pick = |name: &str| {
            reports
                .iter()
                .find(|report| report.service == name)
                .map(|report| report.status)
                .unwrap_or(ServiceStatus::Skipped)
        };

        Self {
            profile: profile.to_string(),
            youtube: pick("YouTube"),
            discord_text: pick("Discord Text"),
            discord_voice: pick("Discord Voice"),
            general: pick("General TCP"),
            elapsed_ms: reports.iter().map(|report| report.elapsed_ms).sum(),
        }
    }
}

fn tls_ok(domain: &str) -> bool {
    check_domain_tls(domain, PROBE_REQUESTS) == CheckStatus::Pass
}

fn tcp_ok(domain: &str, port: u16) -> bool {
    try_tcp_connect_domain(domain, port).is_ok()
}

fn udp_reachable(domain: &str, port: u16) -> (bool, String) {
    let addresses = resolve_domain(domain);
    let Some(target) = addresses.into_iter().next() else {
        return (false, format!("{} does not resolve", domain));
    };

    let bind = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let Ok(socket) = UdpSocket::bind(bind) else {
        return (false, "cannot bind a local UDP socket".to_string());
    };

    let _ = socket.set_read_timeout(Some(UDP_TIMEOUT));
    let _ = socket.set_write_timeout(Some(UDP_TIMEOUT));

    let destination = SocketAddr::new(target, port);
    if socket.connect(destination).is_err() {
        return (false, format!("UDP connect to {} refused", destination));
    }

    let payload = [0u8; 8];
    if socket.send(&payload).is_err() {
        return (false, format!("UDP send to {} refused", destination));
    }

    let mut buffer = [0u8; 64];
    match socket.recv(&mut buffer) {
        Ok(_) => (true, format!("UDP round trip with {} succeeded", destination)),
        Err(error) => match error.kind() {
            std::io::ErrorKind::ConnectionRefused => {
                (false, format!("ICMP unreachable from {}", destination))
            }
            _ => (
                true,
                format!("UDP path to {} open, no reply within timeout", destination),
            ),
        },
    }
}

fn probe_youtube() -> ServiceReport {
    let started = Instant::now();
    let handshake = tls_ok("www.youtube.com");
    let cdn = tls_ok("rr1---sn-4g5e6nze.googlevideo.com") || tls_ok("i.ytimg.com");

    let status = ServiceStatus::from_pair(handshake, cdn);
    let detail = format!(
        "www.youtube.com TLS {}, video CDN TLS {}",
        if handshake { "ok" } else { "failed" },
        if cdn { "ok" } else { "failed" }
    );

    ServiceReport::new("YouTube", status, detail, started)
}

fn probe_discord_text() -> ServiceReport {
    let started = Instant::now();
    let api = tls_ok("discord.com");
    let gateway = tls_ok("gateway.discord.gg") || tcp_ok("gateway.discord.gg", 443);

    let status = ServiceStatus::from_pair(api, gateway);
    let detail = format!(
        "discord.com TLS {}, gateway.discord.gg {}",
        if api { "ok" } else { "failed" },
        if gateway { "ok" } else { "failed" }
    );

    ServiceReport::new("Discord Text", status, detail, started)
}

fn probe_discord_voice() -> ServiceReport {
    let started = Instant::now();
    let (reachable, detail) = udp_reachable("russia1234.discord.media", VOICE_PORT);
    let fallback = if reachable {
        (true, String::new())
    } else {
        udp_reachable("discord.media", VOICE_PORT)
    };

    let open = reachable || fallback.0;
    let status = if open {
        ServiceStatus::Ok
    } else {
        ServiceStatus::Blocked
    };

    let detail = if reachable || fallback.1.is_empty() {
        detail
    } else {
        format!("{}; fallback: {}", detail, fallback.1)
    };

    ServiceReport::new("Discord Voice", status, detail, started)
}

fn probe_general() -> ServiceReport {
    let started = Instant::now();
    let https = tcp_ok("example.com", 443);
    let http = test_http("example.com", PROBE_REQUESTS);

    let status = ServiceStatus::from_pair(https, http);
    let detail = format!(
        "TCP 443 {}, HTTP 80 {}",
        if https { "ok" } else { "failed" },
        if http { "ok" } else { "failed" }
    );

    ServiceReport::new("General TCP", status, detail, started)
}

pub fn probe_all() -> Vec<ServiceReport> {
    let probes: [(&str, fn() -> ServiceReport); 4] = [
        ("YouTube", probe_youtube),
        ("Discord Text", probe_discord_text),
        ("Discord Voice", probe_discord_voice),
        ("General TCP", probe_general),
    ];

    let mut reports = Vec::with_capacity(probes.len());

    for (name, probe) in probes {
        if is_cancelled() {
            reports.push(ServiceReport::skipped(name));
            continue;
        }
        reports.push(probe());
    }

    reports
}

pub fn run_matrix(profiles: &[String]) -> Vec<MatrixRow> {
    if profiles.is_empty() {
        let reports = probe_all();
        return vec![MatrixRow::from_reports("current", &reports)];
    }

    let mut rows = Vec::with_capacity(profiles.len());

    for profile in profiles {
        if is_cancelled() {
            rows.push(MatrixRow {
                profile: profile.clone(),
                youtube: ServiceStatus::Skipped,
                discord_text: ServiceStatus::Skipped,
                discord_voice: ServiceStatus::Skipped,
                general: ServiceStatus::Skipped,
                elapsed_ms: 0,
            });
            continue;
        }

        let reports = probe_all();
        rows.push(MatrixRow::from_reports(profile, &reports));
    }

    rows
}

fn pad(value: &str, width: usize) -> String {
    let mut padded = value.to_string();
    while padded.chars().count() < width {
        padded.push(' ');
    }
    padded
}

pub fn render_matrix(rows: &[MatrixRow]) -> Vec<String> {
    let profile_width = rows
        .iter()
        .map(|row| row.profile.chars().count())
        .max()
        .unwrap_or(7)
        .max(7);

    let mut lines = Vec::with_capacity(rows.len() + 2);

    lines.push(format!(
        "{}  {}  {}  {}  {}  {}",
        pad("PROFILE", profile_width),
        pad("YOUTUBE", 8),
        pad("DC TEXT", 8),
        pad("DC VOICE", 8),
        pad("GENERAL", 8),
        pad("LATENCY", 10)
    ));

    lines.push("-".repeat(profile_width + 54));

    for row in rows {
        lines.push(format!(
            "{}  {}  {}  {}  {}  {}",
            pad(&row.profile, profile_width),
            pad(row.youtube.symbol(), 8),
            pad(row.discord_text.symbol(), 8),
            pad(row.discord_voice.symbol(), 8),
            pad(row.general.symbol(), 8),
            pad(&format!("{} ms", row.elapsed_ms), 10)
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(service: &str, status: ServiceStatus) -> ServiceReport {
        ServiceReport {
            service: service.to_string(),
            status,
            detail: String::new(),
            elapsed_ms: 5,
        }
    }

    #[test]
    fn a_fully_open_row_unblocks_everything() {
        let reports = vec![
            report("YouTube", ServiceStatus::Ok),
            report("Discord Text", ServiceStatus::Ok),
            report("Discord Voice", ServiceStatus::Degraded),
            report("General TCP", ServiceStatus::Ok),
        ];

        let row = MatrixRow::from_reports("preset-a", &reports);
        assert!(row.unblocks_everything());
        assert_eq!(row.elapsed_ms, 20);
    }

    #[test]
    fn a_blocked_service_fails_the_row() {
        let reports = vec![
            report("YouTube", ServiceStatus::Ok),
            report("Discord Text", ServiceStatus::Ok),
            report("Discord Voice", ServiceStatus::Blocked),
            report("General TCP", ServiceStatus::Ok),
        ];

        let row = MatrixRow::from_reports("preset-b", &reports);
        assert!(!row.unblocks_everything());
    }

    #[test]
    fn missing_reports_become_skipped() {
        let row = MatrixRow::from_reports("preset-c", &[]);
        assert_eq!(row.youtube, ServiceStatus::Skipped);
        assert_eq!(row.discord_voice, ServiceStatus::Skipped);
    }

    #[test]
    fn a_partial_pair_is_degraded() {
        assert_eq!(ServiceStatus::from_pair(true, false), ServiceStatus::Degraded);
        assert_eq!(ServiceStatus::from_pair(true, true), ServiceStatus::Ok);
        assert_eq!(ServiceStatus::from_pair(false, false), ServiceStatus::Blocked);
    }

    #[test]
    fn the_matrix_renders_a_header_and_one_line_per_row() {
        let rows = vec![MatrixRow {
            profile: "general".to_string(),
            youtube: ServiceStatus::Ok,
            discord_text: ServiceStatus::Ok,
            discord_voice: ServiceStatus::Blocked,
            general: ServiceStatus::Ok,
            elapsed_ms: 12,
        }];

        let lines = render_matrix(&rows);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("DC VOICE"));
        assert!(lines[2].contains("general"));
        assert!(lines[2].contains("BLOCKED"));
    }
}
