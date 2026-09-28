use crate::strategy::parser::{tokenize_preset, GameFilterPorts};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterSpec {
    pub tcp_ports: String,
    pub udp_ports: String,
    pub l7_protocols: Vec<String>,
    pub l3_protocols: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FakePayload {
    pub kind: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesyncSpec {
    pub methods: Vec<String>,
    pub fooling: Vec<String>,
    pub cutoff: Option<String>,
    pub repeats: Option<String>,
    pub any_protocol: bool,
    pub start: Option<String>,
    pub autottl: Option<String>,
    pub ttl: Option<String>,
    pub ttl6: Option<String>,
    pub split_pos: Vec<String>,
    pub split_seqovl: Option<String>,
    pub split_seqovl_pattern: Option<String>,
    pub fakedsplit_pattern: Option<String>,
    pub badseq_increment: Option<String>,
    pub badack_increment: Option<String>,
    pub fake_tls_mod: Vec<String>,
    pub hostfakesplit_mod: Vec<String>,
    pub fake_payloads: Vec<FakePayload>,
}

impl DesyncSpec {
    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
            && self.fooling.is_empty()
            && self.cutoff.is_none()
            && self.repeats.is_none()
            && !self.any_protocol
            && self.start.is_none()
            && self.autottl.is_none()
            && self.ttl.is_none()
            && self.ttl6.is_none()
            && self.split_pos.is_empty()
            && self.split_seqovl.is_none()
            && self.split_seqovl_pattern.is_none()
            && self.fakedsplit_pattern.is_none()
            && self.badseq_increment.is_none()
            && self.badack_increment.is_none()
            && self.fake_tls_mod.is_empty()
            && self.hostfakesplit_mod.is_empty()
            && self.fake_payloads.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetListKind {
    Hostlist,
    HostlistExclude,
    HostlistAuto,
    HostlistDomains,
    HostlistExcludeDomains,
    Ipset,
    IpsetExclude,
    IpsetIp,
    IpsetExcludeIp,
}

impl TargetListKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hostlist => "hostlist",
            Self::HostlistExclude => "hostlist-exclude",
            Self::HostlistAuto => "hostlist-auto",
            Self::HostlistDomains => "hostlist-domains",
            Self::HostlistExcludeDomains => "hostlist-exclude-domains",
            Self::Ipset => "ipset",
            Self::IpsetExclude => "ipset-exclude",
            Self::IpsetIp => "ipset-ip",
            Self::IpsetExcludeIp => "ipset-exclude-ip",
        }
    }

    pub fn is_file_ref(&self) -> bool {
        matches!(
            self,
            Self::Hostlist | Self::HostlistExclude | Self::HostlistAuto | Self::Ipset | Self::IpsetExclude
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetListRef {
    pub kind: TargetListKind,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterGroup {
    pub index: usize,
    pub comment: Option<String>,
    pub filter: FilterSpec,
    pub desync: DesyncSpec,
    pub target_lists: Vec<TargetListRef>,
    pub ip_id: Option<String>,
    pub other_flags: Vec<String>,
}

impl FilterGroup {
    pub fn summary_label(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(comment) = &self.comment {
            if !comment.is_empty() {
                parts.push(comment.clone());
            }
        }

        if !self.filter.tcp_ports.is_empty() {
            parts.push(format!("TCP {}", self.filter.tcp_ports));
        }
        if !self.filter.udp_ports.is_empty() {
            parts.push(format!("UDP {}", self.filter.udp_ports));
        }
        if !self.filter.l7_protocols.is_empty() {
            parts.push(format!("L7 {}", self.filter.l7_protocols.join(",")));
        }
        if parts.is_empty() {
            parts.push("all traffic".to_string());
        }

        if !self.desync.methods.is_empty() {
            parts.push(format!("desync={}", self.desync.methods.join(",")));
        }

        parts.join(" · ")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Zapret1Strategy {
    pub global_tcp_ports: String,
    pub global_udp_ports: String,
    pub groups: Vec<FilterGroup>,
    pub uses_game_filter_tcp: bool,
    pub uses_game_filter_udp: bool,
}

impl Zapret1Strategy {
    pub fn desync_methods(&self) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for group in &self.groups {
            for method in &group.desync.methods {
                if !found.contains(method) {
                    found.push(method.clone());
                }
            }
        }
        found
    }

    pub fn target_lists(&self) -> Vec<TargetListRef> {
        let mut found: Vec<TargetListRef> = Vec::new();
        for group in &self.groups {
            for reference in &group.target_lists {
                if !found.contains(reference) {
                    found.push(reference.clone());
                }
            }
        }
        found
    }
}

fn value_of<'a>(token: &'a str, prefix: &str) -> Option<&'a str> {
    token.strip_prefix(prefix)
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn strip_quotes(value: &str) -> String {
    value.trim_matches('"').trim_matches('\'').to_string()
}

fn parse_group(tokens: &[String], index: usize) -> FilterGroup {
    let mut group = FilterGroup {
        index,
        ..Default::default()
    };

    for token in tokens {
        if token == "--new" {
            continue;
        }

        if let Some(value) = value_of(token, "--comment=") {
            group.comment = Some(strip_quotes(value));
        } else if let Some(value) = token.strip_prefix("--comment ") {
            group.comment = Some(value.trim().to_string());
        } else if let Some(value) = value_of(token, "--filter-tcp=") {
            group.filter.tcp_ports = value.to_string();
        } else if let Some(value) = value_of(token, "--filter-udp=") {
            group.filter.udp_ports = value.to_string();
        } else if let Some(value) = value_of(token, "--filter-l7=") {
            group.filter.l7_protocols = split_csv(value);
        } else if let Some(value) = value_of(token, "--filter-l3=") {
            group.filter.l3_protocols = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync=") {
            group.desync.methods = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync-fooling=") {
            group.desync.fooling = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync-cutoff=") {
            group.desync.cutoff = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-repeats=") {
            group.desync.repeats = Some(value.to_string());
        } else if token == "--dpi-desync-any-protocol" {
            group.desync.any_protocol = true;
        } else if let Some(value) = value_of(token, "--dpi-desync-any-protocol=") {
            group.desync.any_protocol = value != "0";
        } else if let Some(value) = value_of(token, "--dpi-desync-start=") {
            group.desync.start = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-autottl=") {
            group.desync.autottl = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-ttl6=") {
            group.desync.ttl6 = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-ttl=") {
            group.desync.ttl = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-split-pos=") {
            group.desync.split_pos = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync-split-seqovl-pattern=") {
            group.desync.split_seqovl_pattern = Some(strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-split-seqovl=") {
            group.desync.split_seqovl = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-fakedsplit-pattern=") {
            group.desync.fakedsplit_pattern = Some(strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-badseq-increment=") {
            group.desync.badseq_increment = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-badack-increment=") {
            group.desync.badack_increment = Some(value.to_string());
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-tls-mod=") {
            group.desync.fake_tls_mod = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync-hostfakesplit-mod=") {
            group.desync.hostfakesplit_mod = split_csv(value);
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-tls=") {
            push_fake(&mut group.desync.fake_payloads, "tls", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-http=") {
            push_fake(&mut group.desync.fake_payloads, "http", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-quic=") {
            push_fake(&mut group.desync.fake_payloads, "quic", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-discord=") {
            push_fake(&mut group.desync.fake_payloads, "discord", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-stun=") {
            push_fake(&mut group.desync.fake_payloads, "stun", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-wireguard=") {
            push_fake(&mut group.desync.fake_payloads, "wireguard", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-syndata=") {
            push_fake(&mut group.desync.fake_payloads, "syndata", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-unknown-udp=") {
            push_fake(&mut group.desync.fake_payloads, "unknown-udp", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--dpi-desync-fake-unknown=") {
            push_fake(&mut group.desync.fake_payloads, "unknown", strip_quotes(value));
        } else if let Some(value) = value_of(token, "--hostlist-exclude-domains=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::HostlistExcludeDomains,
                value: value.to_string(),
            });
        } else if let Some(value) = value_of(token, "--hostlist-exclude=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::HostlistExclude,
                value: strip_quotes(value),
            });
        } else if let Some(value) = value_of(token, "--hostlist-auto=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::HostlistAuto,
                value: strip_quotes(value),
            });
        } else if let Some(value) = value_of(token, "--hostlist-domains=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::HostlistDomains,
                value: value.to_string(),
            });
        } else if let Some(value) = value_of(token, "--hostlist=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::Hostlist,
                value: strip_quotes(value),
            });
        } else if let Some(value) = value_of(token, "--ipset-exclude-ip=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::IpsetExcludeIp,
                value: value.to_string(),
            });
        } else if let Some(value) = value_of(token, "--ipset-exclude=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::IpsetExclude,
                value: strip_quotes(value),
            });
        } else if let Some(value) = value_of(token, "--ipset-ip=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::IpsetIp,
                value: value.to_string(),
            });
        } else if let Some(value) = value_of(token, "--ipset=") {
            group.target_lists.push(TargetListRef {
                kind: TargetListKind::Ipset,
                value: strip_quotes(value),
            });
        } else if let Some(value) = value_of(token, "--ip-id=") {
            group.ip_id = Some(value.to_string());
        } else if token.starts_with("--wf-") {
            continue;
        } else {
            group.other_flags.push(token.clone());
        }
    }

    group
}

fn push_fake(payloads: &mut Vec<FakePayload>, kind: &str, value: String) {
    if value.is_empty() {
        return;
    }
    if let Some(existing) = payloads.iter_mut().find(|p| p.kind == kind) {
        existing.values.push(value);
    } else {
        payloads.push(FakePayload {
            kind: kind.to_string(),
            values: vec![value],
        });
    }
}

pub fn parse_zapret1_content(content: &str, game_filter: Option<&GameFilterPorts>) -> Zapret1Strategy {
    let mut normalized = content.replace('\r', "");
    normalized = normalized.replace("%BIN%", "bin/");
    normalized = normalized.replace("%LISTS%", "lists/");

    if let Some(gf) = game_filter {
        normalized = normalized.replace("%GameFilter%", &gf.ports);
        normalized = normalized.replace("%GameFilterTCP%", &gf.tcp_ports);
        normalized = normalized.replace("%GameFilterUDP%", &gf.udp_ports);
    }

    let uses_game_filter_tcp = normalized.contains("%GameFilterTCP%") || normalized.contains("%GameFilter%");
    let uses_game_filter_udp = normalized.contains("%GameFilterUDP%") || normalized.contains("%GameFilter%");

    for placeholder in &[
        ",%GameFilter%",
        "%GameFilter%,",
        "%GameFilter%",
        ",%GameFilterTCP%",
        "%GameFilterTCP%,",
        "%GameFilterTCP%",
        ",%GameFilterUDP%",
        "%GameFilterUDP%,",
        "%GameFilterUDP%",
    ] {
        normalized = normalized.replace(placeholder, "");
    }

    let tokens = tokenize_preset(&normalized);

    let mut global_tcp_ports = String::new();
    let mut global_udp_ports = String::new();

    for token in &tokens {
        if let Some(value) = value_of(token, "--wf-tcp=") {
            global_tcp_ports = value.to_string();
        } else if let Some(value) = value_of(token, "--wf-udp=") {
            global_udp_ports = value.to_string();
        }
    }

    let mut groups: Vec<FilterGroup> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut group_index = 0usize;
    let mut seen_filter_flag = false;

    for token in tokens {
        if token.starts_with("--wf-") {
            continue;
        }
        if token == "--new" {
            if seen_filter_flag {
                groups.push(parse_group(&current, group_index));
                group_index += 1;
                current.clear();
                seen_filter_flag = false;
            }
            continue;
        }
        if token.starts_with("--filter-tcp=")
            || token.starts_with("--filter-udp=")
            || token.starts_with("--filter-l7=")
            || token.starts_with("--filter-l3=")
        {
            seen_filter_flag = true;
        }
        current.push(token);
    }

    if seen_filter_flag && !current.is_empty() {
        groups.push(parse_group(&current, group_index));
    }

    Zapret1Strategy {
        global_tcp_ports,
        global_udp_ports,
        groups,
        uses_game_filter_tcp,
        uses_game_filter_udp,
    }
}

pub fn parse_zapret1_file(path: &Path, game_filter: Option<&GameFilterPorts>) -> Result<Zapret1Strategy, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(parse_zapret1_content(&content, game_filter))
}

fn quote_if_needed(value: &str) -> String {
    if value.contains(' ') || value.contains(',') {
        format!("\"{}\"", value)
    } else {
        value.to_string()
    }
}

pub fn render_group_cli(group: &FilterGroup) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(comment) = &group.comment {
        if !comment.is_empty() {
            parts.push(format!("--comment {}", comment));
        }
    }

    if !group.filter.tcp_ports.is_empty() {
        parts.push(format!("--filter-tcp={}", group.filter.tcp_ports));
    }
    if !group.filter.udp_ports.is_empty() {
        parts.push(format!("--filter-udp={}", group.filter.udp_ports));
    }
    if !group.filter.l7_protocols.is_empty() {
        parts.push(format!("--filter-l7={}", group.filter.l7_protocols.join(",")));
    }
    if !group.filter.l3_protocols.is_empty() {
        parts.push(format!("--filter-l3={}", group.filter.l3_protocols.join(",")));
    }

    for reference in &group.target_lists {
        parts.push(format!(
            "--{}={}",
            reference.kind.label(),
            quote_if_needed(&reference.value)
        ));
    }

    if let Some(value) = &group.ip_id {
        parts.push(format!("--ip-id={}", value));
    }

    if !group.desync.methods.is_empty() {
        parts.push(format!("--dpi-desync={}", group.desync.methods.join(",")));
    }
    if group.desync.any_protocol {
        parts.push("--dpi-desync-any-protocol=1".to_string());
    }
    if let Some(value) = &group.desync.cutoff {
        parts.push(format!("--dpi-desync-cutoff={}", value));
    }
    if let Some(value) = &group.desync.start {
        parts.push(format!("--dpi-desync-start={}", value));
    }
    if let Some(value) = &group.desync.repeats {
        parts.push(format!("--dpi-desync-repeats={}", value));
    }
    if !group.desync.split_pos.is_empty() {
        parts.push(format!("--dpi-desync-split-pos={}", group.desync.split_pos.join(",")));
    }
    if let Some(value) = &group.desync.split_seqovl {
        parts.push(format!("--dpi-desync-split-seqovl={}", value));
    }
    if let Some(value) = &group.desync.split_seqovl_pattern {
        parts.push(format!("--dpi-desync-split-seqovl-pattern={}", quote_if_needed(value)));
    }
    if let Some(value) = &group.desync.fakedsplit_pattern {
        parts.push(format!("--dpi-desync-fakedsplit-pattern={}", quote_if_needed(value)));
    }
    if !group.desync.fooling.is_empty() {
        parts.push(format!("--dpi-desync-fooling={}", group.desync.fooling.join(",")));
    }
    if let Some(value) = &group.desync.badseq_increment {
        parts.push(format!("--dpi-desync-badseq-increment={}", value));
    }
    if let Some(value) = &group.desync.badack_increment {
        parts.push(format!("--dpi-desync-badack-increment={}", value));
    }
    if let Some(value) = &group.desync.autottl {
        parts.push(format!("--dpi-desync-autottl={}", value));
    }
    if let Some(value) = &group.desync.ttl {
        parts.push(format!("--dpi-desync-ttl={}", value));
    }
    if let Some(value) = &group.desync.ttl6 {
        parts.push(format!("--dpi-desync-ttl6={}", value));
    }

    for payload in &group.desync.fake_payloads {
        for value in &payload.values {
            parts.push(format!("--dpi-desync-fake-{}={}", payload.kind, quote_if_needed(value)));
        }
    }

    if !group.desync.fake_tls_mod.is_empty() {
        parts.push(format!("--dpi-desync-fake-tls-mod={}", group.desync.fake_tls_mod.join(",")));
    }
    if !group.desync.hostfakesplit_mod.is_empty() {
        parts.push(format!(
            "--dpi-desync-hostfakesplit-mod={}",
            group.desync.hostfakesplit_mod.join(",")
        ));
    }

    for flag in &group.other_flags {
        parts.push(flag.clone());
    }

    parts.join(" ")
}

pub fn render_group_cli_public(group: &FilterGroup) -> String {
    render_group_cli(group)
}

pub fn render_cli_preview(strategy: &Zapret1Strategy, binary: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{} --wf-tcp={} --wf-udp={} \\",
        binary, strategy.global_tcp_ports, strategy.global_udp_ports
    ));

    let last_index = strategy.groups.len().saturating_sub(1);
    for (position, group) in strategy.groups.iter().enumerate() {
        let body = render_group_cli(group);
        let mut line = format!("  {}", body);
        if position != last_index {
            line.push_str(" \\");
        }
        lines.push(line);
        if position != last_index {
            lines.push("  --new \\".to_string());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "--wf-tcp=80,443,2053 --wf-udp=443,50000-50100 \
--filter-udp=443 --hostlist=\"lists/list-general.txt\" --dpi-desync=fake --dpi-desync-repeats=6 \
--dpi-desync-fake-quic=\"bin/quic_initial_www_google_com.bin\" --new \
--filter-tcp=80,443 --hostlist=\"lists/list-general.txt\" --dpi-desync=fake,multidisorder \
--dpi-desync-split-pos=1,midsld --dpi-desync-fooling=badseq --dpi-desync-fake-tls=0x00000000 \
--dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com";

    #[test]
    fn parses_global_ports() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        assert_eq!(parsed.global_tcp_ports, "80,443,2053");
        assert_eq!(parsed.global_udp_ports, "443,50000-50100");
    }

    #[test]
    fn splits_into_filter_groups_on_new() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        assert_eq!(parsed.groups.len(), 2);
        assert_eq!(parsed.groups[0].filter.udp_ports, "443");
        assert_eq!(parsed.groups[1].filter.tcp_ports, "80,443");
    }

    #[test]
    fn parses_desync_methods_and_fooling() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        assert_eq!(
            parsed.groups[1].desync.methods,
            vec!["fake".to_string(), "multidisorder".to_string()]
        );
        assert_eq!(parsed.groups[1].desync.fooling, vec!["badseq".to_string()]);
        assert_eq!(
            parsed.groups[1].desync.split_pos,
            vec!["1".to_string(), "midsld".to_string()]
        );
    }

    #[test]
    fn parses_fake_tls_mod_and_hex_payload() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        let fake = &parsed.groups[1].desync.fake_payloads;
        assert_eq!(fake.len(), 1);
        assert_eq!(fake[0].kind, "tls");
        assert_eq!(fake[0].values, vec!["0x00000000".to_string()]);
        assert_eq!(
            parsed.groups[1].desync.fake_tls_mod,
            vec!["rnd".to_string(), "dupsid".to_string(), "sni=www.google.com".to_string()]
        );
    }

    #[test]
    fn collects_target_lists() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        assert_eq!(parsed.groups[0].target_lists.len(), 1);
        assert_eq!(parsed.groups[0].target_lists[0].kind, TargetListKind::Hostlist);
        assert_eq!(parsed.groups[0].target_lists[0].value, "lists/list-general.txt");
    }

    #[test]
    fn splits_groups_defined_only_by_filter_l7() {
        let content = "--wf-tcp=443 --wf-udp=443,19294-19344,50000-50100 \
--filter-l7=quic --hostlist=\"lists/list-general.txt\" --dpi-desync=fake --dpi-desync-repeats=11 \
--dpi-desync-fake-quic=\"bin/quic_initial_www_google_com.bin\" --new \
--filter-udp=19294-19344,50000-50100 --filter-l7=discord,stun --dpi-desync=fake \
--dpi-desync-fake-discord=\"bin/ACTIVE_DISCORD_UDP.bin\" --dpi-desync-repeats=4";
        let parsed = parse_zapret1_content(content, None);

        assert_eq!(parsed.groups.len(), 2);

        assert!(parsed.groups[0].filter.udp_ports.is_empty());
        assert!(parsed.groups[0].filter.tcp_ports.is_empty());
        assert_eq!(parsed.groups[0].filter.l7_protocols, vec!["quic".to_string()]);
        assert_eq!(parsed.groups[0].target_lists.len(), 1);
        assert_eq!(parsed.groups[0].desync.fake_payloads.len(), 1);
        assert_eq!(parsed.groups[0].desync.fake_payloads[0].kind, "quic");

        assert_eq!(parsed.groups[1].filter.udp_ports, "19294-19344,50000-50100");
        assert_eq!(
            parsed.groups[1].filter.l7_protocols,
            vec!["discord".to_string(), "stun".to_string()]
        );
        assert_eq!(parsed.groups[1].target_lists.len(), 0);
        assert_eq!(parsed.groups[1].desync.fake_payloads.len(), 1);
        assert_eq!(parsed.groups[1].desync.fake_payloads[0].kind, "discord");
    }

    #[test]
    fn substitutes_bin_and_lists_macros_from_real_bat_content() {
        let content = "--wf-tcp=443 --wf-udp=443 \
--filter-tcp=443 --hostlist=\"%LISTS%list-general.txt\" --ipset-exclude=\"%LISTS%ipset-exclude.txt\" \
--dpi-desync=fake --dpi-desync-fake-tls=\"%BIN%tls_clienthello_www_google_com.bin\"";
        let parsed = parse_zapret1_content(content, None);

        assert_eq!(parsed.groups[0].target_lists[0].value, "lists/list-general.txt");
        assert_eq!(parsed.groups[0].target_lists[1].value, "lists/ipset-exclude.txt");
        assert_eq!(
            parsed.groups[0].desync.fake_payloads[0].values[0],
            "bin/tls_clienthello_www_google_com.bin"
        );

        assert!(!parsed.groups[0]
            .target_lists
            .iter()
            .any(|reference| reference.value.contains('%')));
    }

    #[test]
    fn applies_game_filter_substitution() {
        let content = "--wf-tcp=80,%GameFilterTCP% --wf-udp=443,%GameFilterUDP% \
--filter-tcp=%GameFilterTCP% --dpi-desync=fake --new \
--filter-udp=%GameFilterUDP% --dpi-desync=fake";
        let gf = GameFilterPorts {
            ports: "1024-65535".to_string(),
            tcp_ports: "1024-65535".to_string(),
            udp_ports: "1024-65535".to_string(),
        };
        let parsed = parse_zapret1_content(content, Some(&gf));
        assert_eq!(parsed.global_tcp_ports, "80,1024-65535");
        assert!(parsed.uses_game_filter_tcp);
        assert!(parsed.uses_game_filter_udp);
        assert_eq!(parsed.groups[0].filter.tcp_ports, "1024-65535");
    }

    #[test]
    fn parses_comment_with_spaces_and_parens_as_group_label() {
        let content = "--wf-tcp=443,853 --wf-udp=443 \
--comment Cloudflare WARP Gateway(1.1.1.1, 1.0.0.1) --filter-tcp=443,853 --ipset-ip=162.159.36.1 \
--dpi-desync=syndata --dpi-desync-fake-syndata=0x00 --dpi-desync-cutoff=n2 --new \
--comment WireGuard handshake --filter-udp=53-65535 --filter-l7=wireguard --dpi-desync=fake \
--dpi-desync-cutoff=n2 --dpi-desync-repeats=4";
        let parsed = parse_zapret1_content(content, None);

        assert_eq!(parsed.groups.len(), 2);
        assert_eq!(
            parsed.groups[0].comment.as_deref(),
            Some("Cloudflare WARP Gateway(1.1.1.1, 1.0.0.1)")
        );
        assert_eq!(parsed.groups[1].comment.as_deref(), Some("WireGuard handshake"));

        assert!(parsed.groups[0]
            .other_flags
            .iter()
            .all(|flag| !flag.starts_with("--comment")));

        assert!(parsed.groups[0].summary_label().starts_with("Cloudflare WARP Gateway"));

        let group_cli = render_group_cli_public(&parsed.groups[1]);
        assert!(group_cli.starts_with("--comment WireGuard handshake"));

        let preview = render_cli_preview(&parsed, "winws");
        assert!(preview.contains("--comment Cloudflare WARP Gateway(1.1.1.1, 1.0.0.1)"));
        assert!(preview.contains("--comment WireGuard handshake"));
    }

    #[test]
    fn keeps_unknown_flags_in_other_bucket() {
        let content = "--wf-tcp=443 --wf-udp=443 --filter-tcp=443 --dpi-desync=fake --dpi-desync-skip-nosni=1";
        let parsed = parse_zapret1_content(content, None);
        assert!(parsed.groups[0]
            .other_flags
            .iter()
            .any(|flag| flag == "--dpi-desync-skip-nosni=1"));
    }

    #[test]
    fn cli_preview_reconstructs_new_separated_groups() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        let preview = render_cli_preview(&parsed, "nfqws");
        assert!(preview.contains("nfqws --wf-tcp=80,443,2053 --wf-udp=443,50000-50100"));
        assert!(preview.contains("--new"));
        assert!(preview.contains("--dpi-desync=fake,multidisorder"));
    }

    #[test]
    fn desync_methods_are_deduplicated_across_groups() {
        let parsed = parse_zapret1_content(SAMPLE, None);
        let methods = parsed.desync_methods();
        assert!(methods.contains(&"fake".to_string()));
        assert!(methods.contains(&"multidisorder".to_string()));
    }
}
