use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

static RE_LINE_CONTINUATION: OnceLock<Regex> = OnceLock::new();
static RE_WF_TCP: OnceLock<Regex> = OnceLock::new();
static RE_WF_UDP: OnceLock<Regex> = OnceLock::new();
static RE_FILTER: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Default)]
pub struct ParsedStrategy {
    pub tcp_ports: String,

    pub udp_ports: String,

    pub nfqws_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameFilterPorts {
    pub ports: String,
    pub tcp_ports: String,
    pub udp_ports: String,
}

pub fn parse_bat_file(file_path: &str, game_filter: Option<&GameFilterPorts>) -> Result<ParsedStrategy, String> {
    if !Path::new(file_path).exists() {
        return Err(format!("Strategy file not found: {}", file_path));
    }

    let raw = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let mut content = raw.replace('\r', "");

    let re_continuation = RE_LINE_CONTINUATION.get_or_init(|| Regex::new(r"\^\s*\n").unwrap());
    content = re_continuation.replace_all(&content, "\n").to_string();

    content = content.replace("%BIN%", "bin/");
    content = content.replace("%LISTS%", "lists/");

    if let Some(gf) = game_filter {
        content = content.replace("%GameFilter%", &gf.ports);
        content = content.replace("%GameFilterTCP%", &gf.tcp_ports);
        content = content.replace("%GameFilterUDP%", &gf.udp_ports);
    } else {
        for placeholder in &[
            ",%GameFilter%",
            "%GameFilter%,",
            ",%GameFilterTCP%",
            "%GameFilterTCP%,",
            ",%GameFilterUDP%",
            "%GameFilterUDP%,",
        ] {
            content = content.replace(placeholder, "");
        }
    }

    let wf_tcp_re = RE_WF_TCP.get_or_init(|| Regex::new(r"--wf-tcp=([0-9,-]+)").unwrap());
    let wf_udp_re = RE_WF_UDP.get_or_init(|| Regex::new(r"--wf-udp=([0-9,-]+)").unwrap());

    let tcp_matches: Vec<_> = wf_tcp_re.find_iter(&content).collect();
    let udp_matches: Vec<_> = wf_udp_re.find_iter(&content).collect();

    if tcp_matches.is_empty() || udp_matches.is_empty() {
        return Err(format!("--wf-tcp or --wf-udp not found in '{}'", file_path));
    }
    if tcp_matches.len() > 1 {
        return Err(format!("Multiple --wf-tcp entries found in '{}'", file_path));
    }
    if udp_matches.len() > 1 {
        return Err(format!("Multiple --wf-udp entries found in '{}'", file_path));
    }

    let tcp_ports = wf_tcp_re.captures(&content).unwrap()[1].to_string();
    let udp_ports = wf_udp_re.captures(&content).unwrap()[1].to_string();

    let filter_re =
        RE_FILTER.get_or_init(|| Regex::new(r"--filter-(tcp|udp)=([0-9,-]+)\s+([\s\S]*?--new|.*)").unwrap());

    let nfqws_params = filter_re
        .captures_iter(&content)
        .map(|caps| {
            let args = caps[0]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .replace("=^!", "=!");
            args
        })
        .collect();

    Ok(ParsedStrategy {
        tcp_ports,
        udp_ports,
        nfqws_params,
    })
}

static RE_PRESET_TCP: OnceLock<Regex> = OnceLock::new();
static RE_PRESET_UDP: OnceLock<Regex> = OnceLock::new();

pub const DEFAULT_PRESET_TCP_PORTS: &str = "80,443";
pub const DEFAULT_PRESET_UDP_PORTS: &str = "443,50000-50100";

const LUA_DESYNC_PREFIX: &str = "--lua-desync=";
const TTL_PARAM_KEYS: &[&str] = &["ip_ttl", "ip6_ttl", "ip_autottl", "ip6_autottl"];

#[derive(Debug, Default, Clone)]
pub struct ParsedPreset {
    pub tcp_ports: String,
    pub udp_ports: String,
    pub args: Vec<String>,
}

fn split_preset_args(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' {
            in_quote = !in_quote;
            i += 1;
            continue;
        }
        if !in_quote && c.is_whitespace() {
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j + 1 < chars.len() && chars[j] == '-' && chars[j + 1] == '-' {
                let trimmed = cur.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
                cur.clear();
            } else if j < chars.len() {
                cur.push(' ');
            }
            i = j;
            continue;
        }
        cur.push(c);
        i += 1;
    }

    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

pub fn tokenize_preset(content: &str) -> Vec<String> {
    let normalized = content.replace('\r', "").replace('\u{feff}', "");
    let joined = RE_LINE_CONTINUATION
        .get_or_init(|| Regex::new(r"\^\s*\n").unwrap())
        .replace_all(&normalized, " ")
        .to_string();

    let mut tokens = Vec::new();
    for raw in joined.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("::") {
            continue;
        }
        if line.get(..3).map(|p| p.eq_ignore_ascii_case("rem")).unwrap_or(false) {
            continue;
        }
        tokens.extend(split_preset_args(line));
    }
    tokens
}

fn merge_ports(values: &[String]) -> String {
    let mut ranges: Vec<(u32, u32)> = Vec::new();

    for value in values {
        for part in value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let bounds = match part.split_once('-') {
                Some((lo, hi)) => match (lo.trim().parse::<u32>(), hi.trim().parse::<u32>()) {
                    (Ok(lo), Ok(hi)) if lo <= hi => (lo, hi),
                    _ => continue,
                },
                None => match part.parse::<u32>() {
                    Ok(port) => (port, port),
                    Err(_) => continue,
                },
            };
            if bounds.0 == 0 || bounds.1 > 65535 {
                continue;
            }
            ranges.push(bounds);
        }
    }

    if ranges.is_empty() {
        return String::new();
    }

    ranges.sort();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len());
    for (lo, hi) in ranges {
        match merged.last_mut() {
            Some(last) if lo <= last.1.saturating_add(1) => {
                if hi > last.1 {
                    last.1 = hi;
                }
            }
            _ => merged.push((lo, hi)),
        }
    }

    merged
        .into_iter()
        .map(|(lo, hi)| if lo == hi { lo.to_string() } else { format!("{}-{}", lo, hi) })
        .collect::<Vec<_>>()
        .join(",")
}

fn collect_ports(tokens: &[String], re: &Regex) -> String {
    let matches: Vec<String> = tokens
        .iter()
        .filter_map(|t| re.captures(t).map(|c| c[1].to_string()))
        .collect();
    merge_ports(&matches)
}

fn is_windivert_flag(token: &str) -> bool {
    token.starts_with("--wf-")
}

pub fn is_legacy_desync_flag(token: &str) -> bool {
    token.starts_with("--dpi-desync")
}

pub fn apply_ttl_to_lua_desync(token: &str, ttl: u8) -> String {
    let Some(rest) = token.strip_prefix(LUA_DESYNC_PREFIX) else {
        return token.to_string();
    };

    let mut parts: Vec<String> = Vec::new();
    for (index, part) in rest.split(':').enumerate() {
        if index == 0 {
            parts.push(part.to_string());
            continue;
        }
        let key = part.split('=').next().unwrap_or("").trim();
        if TTL_PARAM_KEYS.contains(&key) {
            continue;
        }
        parts.push(part.to_string());
    }

    parts.push(format!("ip_ttl={}", ttl));
    parts.push(format!("ip6_ttl={}", ttl));
    format!("{}{}", LUA_DESYNC_PREFIX, parts.join(":"))
}

fn preset_prelude(tcp_ports: &str, udp_ports: &str) -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        vec![format!("--wf-tcp={}", tcp_ports), format!("--wf-udp={}", udp_ports)]
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (tcp_ports, udp_ports);
        vec![
            format!("--qnum={}", crate::firewalls::NFQUEUE_NUM),
            format!("--fwmark={}", crate::firewalls::FWMARK_HEX),
        ]
    }
}

pub fn parse_preset_content(content: &str, ttl: Option<u8>) -> ParsedPreset {
    let tokens = tokenize_preset(content);

    let tcp_re = RE_PRESET_TCP.get_or_init(|| Regex::new(r"^--(?:wf|filter)-tcp(?:-in|-out)?=([0-9,\-]+)$").unwrap());
    let udp_re = RE_PRESET_UDP.get_or_init(|| Regex::new(r"^--(?:wf|filter)-udp(?:-in|-out)?=([0-9,\-]+)$").unwrap());

    let mut tcp_ports = collect_ports(&tokens, tcp_re);
    let mut udp_ports = collect_ports(&tokens, udp_re);
    if tcp_ports.is_empty() {
        tcp_ports = DEFAULT_PRESET_TCP_PORTS.to_string();
    }
    if udp_ports.is_empty() {
        udp_ports = DEFAULT_PRESET_UDP_PORTS.to_string();
    }

    let mut args = preset_prelude(&tcp_ports, &udp_ports);

    let mut group_filled = false;
    let mut pending_new = false;
    for token in tokens {
        if is_windivert_flag(&token) || is_legacy_desync_flag(&token) {
            continue;
        }
        if token == "--new" {
            pending_new = group_filled;
            continue;
        }
        if pending_new {
            args.push("--new".to_string());
            pending_new = false;
        }
        let token = match ttl {
            Some(value) if token.starts_with(LUA_DESYNC_PREFIX) => apply_ttl_to_lua_desync(&token, value),
            _ => token,
        };
        args.push(token);
        group_filled = true;
    }

    ParsedPreset {
        tcp_ports,
        udp_ports,
        args,
    }
}

pub fn parse_zapret2_preset(file_path: &str, ttl: Option<u8>) -> Result<ParsedPreset, String> {
    if !Path::new(file_path).exists() {
        return Err(format!("Preset file not found: {}", file_path));
    }
    let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let parsed = parse_preset_content(&content, ttl);
    if parsed.args.len() <= 2 {
        return Err(format!("Preset '{}' contains no usable arguments", file_path));
    }
    Ok(parsed)
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    const SAMPLE: &str = "# Preset: sample\n\
--lua-init=@lua/zapret-lib.lua\n\
\n\
--wf-tcp-out=80,443,2053\n\
--wf-udp-out=443,19294-19344\n\
--wf-raw-part=@windivert.filter/windivert_part.stun.txt\n\
\n\
--name=youtube.com (interface)\n\
--filter-tcp=80,443\n\
--lua-desync=fake:blob=tls_google:repeats=6:ip_ttl=9\n\
\n\
--new\n\
\n\
--filter-udp=443-65535\n\
--lua-desync=fake:blob=quic_google:repeats=10\n";

    #[test]
    fn keeps_values_containing_spaces() {
        let tokens = tokenize_preset(SAMPLE);
        assert!(tokens.iter().any(|t| t == "--name=youtube.com (interface)"));
    }

    #[test]
    fn splits_multiple_args_on_one_line() {
        let tokens = split_preset_args("--filter-tcp=443 --hostlist=lists/a.txt --out-range=-d8");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], "--filter-tcp=443");
        assert_eq!(tokens[2], "--out-range=-d8");
    }

    #[test]
    fn strips_comments_and_quotes() {
        let tokens = tokenize_preset("# comment\n--blob=\"a b\"\n:: skipped\n");
        assert_eq!(tokens, vec!["--blob=a b".to_string()]);
    }

    #[test]
    fn joins_caret_continuations() {
        let tokens = tokenize_preset("--filter-tcp=443 ^\n--new\n");
        assert_eq!(tokens, vec!["--filter-tcp=443".to_string(), "--new".to_string()]);
    }

    #[test]
    fn drops_windivert_flags() {
        let parsed = parse_preset_content(SAMPLE, None);
        assert!(!parsed.args.iter().any(|a| a.starts_with("--wf-")));
        assert!(parsed.args.iter().any(|a| a == "--lua-init=@lua/zapret-lib.lua"));
    }

    #[test]
    fn drops_legacy_v1_desync_flags() {
        let parsed = parse_preset_content(
            "--filter-tcp=443\n--dpi-desync=fake\n--dpi-desync-ttl=5\n--dpi-desync-fwmark=0x40000000\n--lua-desync=pass\n",
            None,
        );
        assert!(!parsed.args.iter().any(|a| a.starts_with("--dpi-desync")));
        assert!(parsed.args.iter().any(|a| a == "--lua-desync=pass"));
    }

    #[test]
    fn merges_overlapping_port_ranges() {
        assert_eq!(merge_ports(&["443-65535,443".to_string()]), "443-65535");
        assert_eq!(
            merge_ports(&["80,443,8443".to_string(), "443-65535".to_string()]),
            "80,443-65535"
        );
        assert_eq!(merge_ports(&["80,81".to_string()]), "80-81");
        assert_eq!(merge_ports(&["0,70000,abc".to_string()]), "");
    }

    #[test]
    fn collects_ports_from_wf_and_filter() {
        let parsed = parse_preset_content(SAMPLE, None);
        assert_eq!(parsed.tcp_ports, "80,443,2053");
        assert_eq!(parsed.udp_ports, "443-65535");
    }

    #[test]
    fn falls_back_to_default_ports() {
        let parsed = parse_preset_content("--lua-init=@lua/x.lua\n", None);
        assert_eq!(parsed.tcp_ports, DEFAULT_PRESET_TCP_PORTS);
        assert_eq!(parsed.udp_ports, DEFAULT_PRESET_UDP_PORTS);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn prepends_linux_queue_args() {
        let parsed = parse_preset_content(SAMPLE, None);
        assert_eq!(parsed.args[0], format!("--qnum={}", crate::firewalls::NFQUEUE_NUM));
        assert_eq!(parsed.args[1], format!("--fwmark={}", crate::firewalls::FWMARK_HEX));
        assert!(!parsed.args.iter().any(|a| a.starts_with("--dpi-desync-fwmark")));
    }

    #[test]
    fn rewrites_ttl_into_every_lua_desync_call() {
        let parsed = parse_preset_content(SAMPLE, Some(5));
        let calls: Vec<&String> = parsed
            .args
            .iter()
            .filter(|a| a.starts_with("--lua-desync="))
            .collect();
        assert_eq!(calls.len(), 2);
        for call in calls {
            assert!(call.ends_with(":ip_ttl=5:ip6_ttl=5"));
        }
        assert!(!parsed.args.iter().any(|a| a.contains("ip_ttl=9")));
    }

    #[test]
    fn keeps_preset_ttl_when_unset() {
        let parsed = parse_preset_content(SAMPLE, None);
        assert!(parsed
            .args
            .iter()
            .any(|a| a == "--lua-desync=fake:blob=tls_google:repeats=6:ip_ttl=9"));
    }

    #[test]
    fn replaces_autottl_params_when_ttl_fixed() {
        let out = apply_ttl_to_lua_desync("--lua-desync=fake:ip_autottl=2:repeats=6", 7);
        assert_eq!(out, "--lua-desync=fake:repeats=6:ip_ttl=7:ip6_ttl=7");
    }

    #[test]
    fn drops_trailing_and_duplicate_group_separators() {
        let parsed = parse_preset_content("--filter-tcp=443\n--new\n--new\n", None);
        assert_eq!(parsed.args.iter().filter(|a| *a == "--new").count(), 0);
        assert_ne!(parsed.args.last().unwrap(), "--new");

        let two_groups = parse_preset_content("--filter-tcp=443\n--new\n--new\n--filter-udp=443\n", None);
        assert_eq!(two_groups.args.iter().filter(|a| *a == "--new").count(), 1);
    }
}
