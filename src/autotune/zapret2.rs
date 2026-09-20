use std::io::Write;
use std::process::Command;
use std::time::Instant;

use crate::config::ZapretEngine;
use crate::firewalls::FirewallBackend;

use super::bundles::{is_domain_list, TargetBundle};
use super::types::{is_cancelled, reset_cancel};

pub const Z2_TARGETS: &[&str] = &["discord.com", "gateway.discord.gg", "youtube.com", "googlevideo.com"];

pub const Z2_MAX_TARGETS: usize = 12;

const PROBE_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone, Default)]
pub struct Z2Config {
    pub presets: Vec<String>,
    pub targets: Vec<String>,
    pub bundle: TargetBundle,
    pub lists: Vec<String>,
}

impl Z2Config {
    pub fn resolved_targets(&self) -> Vec<String> {
        if self.targets.is_empty() {
            Z2_TARGETS.iter().map(|d| d.to_string()).collect()
        } else {
            self.targets.clone()
        }
    }

    pub fn resolved_presets(&self) -> Vec<String> {
        if self.presets.is_empty() {
            crate::strategy::zapret2_presets()
        } else {
            self.presets.clone()
        }
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum ProbeOutcome {
    Ok,
    Fail,
    Unsupported,
    Ssl,
}

impl ProbeOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Fail => "FAIL",
            Self::Unsupported => "UNSUP",
            Self::Ssl => "SSL",
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

#[derive(Debug, Clone)]
pub struct Z2Probe {
    pub domain: String,
    pub https: ProbeOutcome,
    pub tls13: ProbeOutcome,
    pub latency_ms: u64,
}

impl Z2Probe {
    pub fn successes(&self) -> usize {
        self.https.is_ok() as usize + self.tls13.is_ok() as usize
    }
}

#[derive(Debug, Clone)]
pub struct Z2PresetResult {
    pub preset: String,
    pub launched: bool,
    pub error: Option<String>,
    pub probes: Vec<Z2Probe>,
    pub tcp_ports: String,
    pub udp_ports: String,
}

impl Z2PresetResult {
    pub fn successes(&self) -> usize {
        self.probes.iter().map(|p| p.successes()).sum()
    }

    pub fn total(&self) -> usize {
        self.probes.len() * 2
    }

    pub fn avg_latency_ms(&self) -> u64 {
        let oks: Vec<u64> = self
            .probes
            .iter()
            .filter(|p| p.successes() > 0)
            .map(|p| p.latency_ms)
            .collect();
        if oks.is_empty() {
            u64::MAX
        } else {
            oks.iter().sum::<u64>() / oks.len() as u64
        }
    }

    pub fn score(&self) -> usize {
        if !self.launched {
            return 0;
        }
        let reachable = self.probes.iter().filter(|p| p.successes() > 0).count();
        self.successes() * 10 + reachable
    }
}

#[derive(Debug, Clone)]
pub struct Z2Results {
    pub presets: Vec<Z2PresetResult>,
    pub ranking: Vec<usize>,
    pub best: Option<String>,
    pub elapsed_secs: u64,
    pub cancelled: bool,
    pub bundle_label: String,
    pub lists: Vec<String>,
    pub targets: Vec<String>,
}

fn null_device() -> &'static str {
    if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    }
}

fn classify(code: &str, stderr: &str, status: Option<i32>) -> ProbeOutcome {
    let lower = stderr.to_lowercase();
    if status == Some(35) || lower.contains("not supported") || lower.contains("unsupported protocol") {
        return ProbeOutcome::Unsupported;
    }
    if lower.contains("certificate") || lower.contains("self-signed") || lower.contains("self signed") {
        return ProbeOutcome::Ssl;
    }
    if !code.is_empty() && code != "000" {
        ProbeOutcome::Ok
    } else {
        ProbeOutcome::Fail
    }
}

fn curl_probe(domain: &str, extra: &[&str]) -> (ProbeOutcome, u64) {
    if is_cancelled() {
        return (ProbeOutcome::Fail, 0);
    }
    let timeout = PROBE_TIMEOUT_SECS.to_string();
    let url = format!("https://{}", domain);
    let started = Instant::now();

    let output = Command::new("curl")
        .args(["-I", "-s", "--show-error"])
        .args(extra)
        .args([
            "--connect-timeout",
            &timeout,
            "--max-time",
            &timeout,
            "-o",
            null_device(),
            "-w",
            "%{http_code}",
        ])
        .arg(&url)
        .output();

    let elapsed = started.elapsed().as_millis() as u64;

    match output {
        Ok(out) => {
            let code = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            (classify(&code, &err, out.status.code()), elapsed)
        }
        Err(_) => (ProbeOutcome::Fail, elapsed),
    }
}

fn probe_domain(domain: &str) -> Z2Probe {
    let (https, https_ms) = curl_probe(domain, &["--tlsv1.2"]);
    let (tls13, tls13_ms) = curl_probe(domain, &["--tlsv1.3", "--tls-max", "1.3"]);

    let mut latencies: Vec<u64> = Vec::new();
    if https.is_ok() {
        latencies.push(https_ms);
    }
    if tls13.is_ok() {
        latencies.push(tls13_ms);
    }
    let latency_ms = if latencies.is_empty() {
        PROBE_TIMEOUT_SECS * 1000
    } else {
        latencies.iter().sum::<u64>() / latencies.len() as u64
    };

    Z2Probe {
        domain: domain.to_string(),
        https,
        tls13,
        latency_ms,
    }
}

fn rank(presets: &[Z2PresetResult]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..presets.len()).filter(|&i| presets[i].score() > 0).collect();
    indices.sort_by(|&a, &b| {
        presets[b]
            .score()
            .cmp(&presets[a].score())
            .then(presets[a].avg_latency_ms().cmp(&presets[b].avg_latency_ms()))
            .then(presets[a].preset.cmp(&presets[b].preset))
    });
    indices
}

pub fn save_results_file(results: &Z2Results) {
    let path = crate::config::get_cache_dir().join(super::storage::RESULTS_FILE);
    let Ok(mut file) = std::fs::File::create(&path) else {
        return;
    };

    let mins = results.elapsed_secs / 60;
    let secs = results.elapsed_secs % 60;
    let _ = writeln!(file, "Zapret2 preset autotune");
    let _ = writeln!(
        file,
        "{} {:02}:{:02}",
        rust_i18n::t!("autotune_time_elapsed"),
        mins,
        secs
    );
    if results.cancelled {
        let _ = writeln!(file, "{}", rust_i18n::t!("autotune_cancelled"));
    }
    let _ = writeln!(file, "{} {}", rust_i18n::t!("autotune_z2_bundle"), results.bundle_label);
    if !results.lists.is_empty() {
        let _ = writeln!(
            file,
            "{} {}",
            rust_i18n::t!("autotune_z2_lists_used"),
            results.lists.join(", ")
        );
    }
    if !results.targets.is_empty() {
        let _ = writeln!(
            file,
            "{} {}",
            rust_i18n::t!("autotune_z2_targets_used"),
            results.targets.join(", ")
        );
    }
    let _ = writeln!(file);

    for pr in &results.presets {
        let _ = writeln!(file, "--- {} ---", pr.preset);
        if !pr.launched {
            let _ = writeln!(
                file,
                "  {}: {}",
                rust_i18n::t!("autotune_z2_launch_failed"),
                pr.error.clone().unwrap_or_default()
            );
            let _ = writeln!(file);
            continue;
        }
        let _ = writeln!(file, "  ports: tcp={} udp={}", pr.tcp_ports, pr.udp_ports);
        for probe in &pr.probes {
            let _ = writeln!(
                file,
                "  {}: HTTPS:{} TLS1.3:{} | {}ms",
                probe.domain,
                probe.https.label(),
                probe.tls13.label(),
                probe.latency_ms
            );
        }
        let latency = pr.avg_latency_ms();
        let latency_str = if latency == u64::MAX {
            "n/a".to_string()
        } else {
            format!("{}ms", latency)
        };
        let _ = writeln!(
            file,
            "  score: {} ({}/{} probes)  latency: {}",
            pr.score(),
            pr.successes(),
            pr.total(),
            latency_str
        );
        let _ = writeln!(file);
    }

    let _ = writeln!(file, "=== {} ===", rust_i18n::t!("autotune_z2_ranking"));
    for (position, &idx) in results.ranking.iter().enumerate() {
        let pr = &results.presets[idx];
        let latency = pr.avg_latency_ms();
        let latency_str = if latency == u64::MAX {
            "n/a".to_string()
        } else {
            format!("{}ms", latency)
        };
        let _ = writeln!(
            file,
            "  {}. {} - {}/{} ({})",
            position + 1,
            pr.preset,
            pr.successes(),
            pr.total(),
            latency_str
        );
    }
    if let Some(ref best) = results.best {
        let _ = writeln!(file);
        let _ = writeln!(file, "{} {}", rust_i18n::t!("autotune_z2_best"), best);
    }
}

fn teardown(backend: &dyn FirewallBackend) {
    crate::runner::stop_zapret_quiet(backend);
}

pub fn run_zapret2_autotune(
    progress: &dyn Fn(usize, usize) -> bool,
    backend: &dyn FirewallBackend,
    interface: &str,
    config: &Z2Config,
) -> Z2Results {
    reset_cancel();
    let started = Instant::now();
    crate::runner::set_engine(ZapretEngine::Zapret2);

    let presets = config.resolved_presets();
    let targets = config.resolved_targets();

    println!(
        "  {} {}",
        rust_i18n::t!("autotune_z2_bundle"),
        config.bundle.label()
    );
    if !config.lists.is_empty() {
        println!(
            "  {} {}",
            rust_i18n::t!("autotune_z2_lists_used"),
            config.lists.join(", ")
        );
    }
    println!("  {} {}", rust_i18n::t!("autotune_z2_targets_used"), targets.join(", "));
    println!();
    let _ = std::io::stdout().flush();

    let steps_per_preset = 1 + targets.len();
    let total = (presets.len() * steps_per_preset).max(1);
    let mut done = 0usize;
    let mut results: Vec<Z2PresetResult> = Vec::new();
    let mut cancelled = false;

    for preset in &presets {
        if is_cancelled() {
            cancelled = true;
            break;
        }

        println!("  [{}/{}] {}", results.len() + 1, presets.len(), preset);
        let _ = std::io::stdout().flush();

        teardown(backend);

        let launch = crate::runner::prepare(&ZapretEngine::Zapret2, preset, false, false, crate::config::load_ttl());
        let (tcp_ports, udp_ports) = match &launch {
            Ok(l) => (l.tcp_ports.clone(), l.udp_ports.clone()),
            Err(_) => (String::new(), String::new()),
        };

        let started_ok = match launch {
            Ok(_) => crate::runner::run_zapret_silent(preset, interface, false, false, backend),
            Err(e) => Err(e),
        };

        done += 1;
        if !progress(done, total) {
            teardown(backend);
            cancelled = true;
            break;
        }

        let mut entry = Z2PresetResult {
            preset: preset.clone(),
            launched: false,
            error: None,
            probes: Vec::new(),
            tcp_ports,
            udp_ports,
        };

        if let Err(e) = started_ok {
            println!("      {}: {}", rust_i18n::t!("autotune_z2_launch_failed"), e);
            let _ = std::io::stdout().flush();
            entry.error = Some(e);
            teardown(backend);
            results.push(entry);
            done += targets.len();
            if !progress(done, total) {
                cancelled = true;
                break;
            }
            continue;
        }

        entry.launched = true;

        let probes: Vec<Z2Probe> = std::thread::scope(|scope| {
            let handles: Vec<_> = targets
                .iter()
                .map(|domain| scope.spawn(move || probe_domain(domain)))
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or(Z2Probe {
                        domain: String::new(),
                        https: ProbeOutcome::Fail,
                        tls13: ProbeOutcome::Fail,
                        latency_ms: 0,
                    })
                })
                .collect()
        });

        for probe in &probes {
            println!(
                "      {:<28} HTTPS:{:<6} TLS1.3:{:<6} {}ms",
                probe.domain,
                probe.https.label(),
                probe.tls13.label(),
                probe.latency_ms
            );
            done += 1;
        }
        let _ = std::io::stdout().flush();

        entry.probes = probes;
        teardown(backend);
        results.push(entry);

        if !progress(done, total) {
            cancelled = true;
            break;
        }
    }

    teardown(backend);

    let ranking = rank(&results);
    let best = ranking.first().map(|&i| results[i].preset.clone());

    if let Some(ref name) = best {
        let _ = crate::config::save_strategy(name);
    }

    let out = Z2Results {
        presets: results,
        ranking,
        best,
        elapsed_secs: started.elapsed().as_secs(),
        cancelled: cancelled || is_cancelled(),
        bundle_label: config.bundle.label(),
        lists: config.lists.clone(),
        targets,
    };

    save_results_file(&out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(domain: &str, https: ProbeOutcome, tls13: ProbeOutcome, latency: u64) -> Z2Probe {
        Z2Probe {
            domain: domain.to_string(),
            https,
            tls13,
            latency_ms: latency,
        }
    }

    fn result(name: &str, launched: bool, probes: Vec<Z2Probe>) -> Z2PresetResult {
        Z2PresetResult {
            preset: name.to_string(),
            launched,
            error: None,
            probes,
            tcp_ports: String::new(),
            udp_ports: String::new(),
        }
    }

    #[test]
    fn failed_launch_scores_zero() {
        let r = result("dead.txt", false, vec![probe("a.com", ProbeOutcome::Ok, ProbeOutcome::Ok, 10)]);
        assert_eq!(r.score(), 0);
    }

    #[test]
    fn ranking_prefers_score_then_latency() {
        let presets = vec![
            result(
                "slow.txt",
                true,
                vec![probe("a.com", ProbeOutcome::Ok, ProbeOutcome::Ok, 900)],
            ),
            result(
                "fast.txt",
                true,
                vec![probe("a.com", ProbeOutcome::Ok, ProbeOutcome::Ok, 40)],
            ),
            result(
                "weak.txt",
                true,
                vec![probe("a.com", ProbeOutcome::Ok, ProbeOutcome::Fail, 40)],
            ),
            result("dead.txt", false, Vec::new()),
        ];
        let ranking = rank(&presets);
        assert_eq!(ranking.len(), 3);
        assert_eq!(presets[ranking[0]].preset, "fast.txt");
        assert_eq!(presets[ranking[1]].preset, "slow.txt");
        assert_eq!(presets[ranking[2]].preset, "weak.txt");
    }

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
    fn empty_target_selection_falls_back_to_defaults() {
        let cfg = Z2Config::default();
        assert_eq!(cfg.resolved_targets().len(), Z2_TARGETS.len());
        assert_eq!(cfg.bundle, TargetBundle::CoreMedia);
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
