use crate::config::ZapretEngine;
use crate::strategy::discovery::is_valid_preset_name;
use crate::strategy::parser::parse_preset_content;
use crate::strategy::refs::{collect_list_refs, list_file_present, resolve_within, ListRef, ListRefKind};
use crate::strategy::zapret1_model::{parse_zapret1_content, render_cli_preview, Zapret1Strategy};
use crate::tui::theme::Theme;
use ratatui::widgets::ListItem;
use std::path::{Path, PathBuf};

const KNOWN_BINARIES: &[&str] = &["nfqws2", "nfqws", "tpws", "winws"];
pub const ZAPRET1_CUSTOM_SUBDIR: &str = "custom-strategies";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyFileKind {
    Preset,
    Profile,
    Zapret1Strategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyFileEntry {
    pub kind: StrategyFileKind,
    pub name: String,
    pub path: PathBuf,
    pub read_only: bool,
}

fn is_valid_zapret1_name(name: &str) -> bool {
    !name.starts_with('_') && name.to_lowercase().ends_with(".bat")
}

fn scan_zapret1_dir(dir: &Path, read_only: bool, seen: &mut Vec<String>, out: &mut Vec<StrategyFileEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !is_valid_zapret1_name(&name) {
            continue;
        }
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        out.push(StrategyFileEntry {
            kind: StrategyFileKind::Zapret1Strategy,
            name,
            path: entry.path(),
            read_only,
        });
    }
}

fn list_zapret1_files() -> Vec<StrategyFileEntry> {
    let ws_dir = crate::strategy::repo_dir();
    let custom_dir = ws_dir.join(ZAPRET1_CUSTOM_SUBDIR);

    let mut entries: Vec<StrategyFileEntry> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    scan_zapret1_dir(&ws_dir, true, &mut seen, &mut entries);
    scan_zapret1_dir(&custom_dir, false, &mut seen, &mut entries);

    entries.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
    entries
}

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

fn scan_dir_read_only(dir: &Path, kind: StrategyFileKind, read_only: bool, out: &mut Vec<StrategyFileEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !is_valid_preset_name(&name) {
            continue;
        }
        out.push(StrategyFileEntry {
            kind,
            name,
            path: entry.path(),
            read_only,
        });
    }
}

pub fn list_files(engine: &ZapretEngine) -> Vec<StrategyFileEntry> {
    match engine {
        ZapretEngine::Zapret1 => list_zapret1_files(),
        ZapretEngine::Zapret2 => {
            let mut entries: Vec<StrategyFileEntry> = Vec::new();
            scan_dir_read_only(&engine.presets_dir(), StrategyFileKind::Preset, true, &mut entries);
            scan_dir_read_only(&engine.profiles_dir(), StrategyFileKind::Profile, false, &mut entries);
            entries.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
            entries
        }
    }
}

pub fn normalize_new_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.to_lowercase().ends_with(".txt") {
        trimmed.to_string()
    } else {
        format!("{}.txt", trimmed)
    }
}

pub fn create_preset_file(engine: &ZapretEngine, name: &str) -> Result<PathBuf, String> {
    let normalized = normalize_new_name(name);
    if !is_valid_preset_name(&normalized) {
        return Err(format!("{}: invalid preset name", normalized));
    }

    let dir = engine.presets_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {}", dir.display(), error))?;

    let path = dir.join(&normalized);
    if path.exists() {
        return Err(format!("{}: already exists", normalized));
    }

    std::fs::write(&path, "").map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(path)
}

pub fn normalize_zapret1_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.to_lowercase().ends_with(".bat") {
        trimmed.to_string()
    } else {
        format!("{}.bat", trimmed)
    }
}

const ZAPRET1_NEW_STRATEGY_TEMPLATE: &str = "@echo off\r\nchcp 65001 > nul\r\n\r\ncd /d \"%~dp0\"\r\n\r\nset \"BIN=%~dp0bin\\\"\r\nset \"LISTS=%~dp0lists\\\"\r\ncd /d %BIN%\r\n\r\nstart \"zapret: %~n0\" /min \"%BIN%winws.exe\" --wf-tcp=80,443 --wf-udp=443 ^\r\n--filter-tcp=80,443 --hostlist=\"%LISTS%list-general.txt\" --dpi-desync=fake --dpi-desync-repeats=6 --dpi-desync-fake-tls=\"%BIN%tls_clienthello_www_google_com.bin\"\r\n";

pub fn create_zapret1_strategy_file(name: &str) -> Result<PathBuf, String> {
    let normalized = normalize_zapret1_name(name);
    if !is_valid_zapret1_name(&normalized) {
        return Err(format!("{}: invalid strategy name", normalized));
    }

    let ws_dir = crate::strategy::repo_dir();
    let dir = ws_dir.join(ZAPRET1_CUSTOM_SUBDIR);
    std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {}", dir.display(), error))?;

    let path = dir.join(&normalized);
    if path.exists() {
        return Err(format!("{}: already exists", normalized));
    }

    std::fs::write(&path, ZAPRET1_NEW_STRATEGY_TEMPLATE).map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(path)
}

fn unique_candidate_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let mut candidate_name = format!("{}_copy.{}", stem, extension);
    let mut candidate_path = dir.join(&candidate_name);
    let mut suffix = 2;
    while candidate_path.exists() {
        candidate_name = format!("{}_copy{}.{}", stem, suffix, extension);
        candidate_path = dir.join(&candidate_name);
        suffix += 1;
    }
    candidate_path
}

fn clone_target_dir(entry: &StrategyFileEntry) -> Result<PathBuf, String> {
    if entry.kind == StrategyFileKind::Zapret1Strategy && entry.read_only {
        let ws_dir = crate::strategy::repo_dir();
        let custom_dir = ws_dir.join(ZAPRET1_CUSTOM_SUBDIR);
        std::fs::create_dir_all(&custom_dir).map_err(|error| format!("{}: {}", custom_dir.display(), error))?;
        Ok(custom_dir)
    } else if entry.kind == StrategyFileKind::Preset && entry.read_only {
        let profiles_dir = ZapretEngine::Zapret2.profiles_dir();
        std::fs::create_dir_all(&profiles_dir).map_err(|error| format!("{}: {}", profiles_dir.display(), error))?;
        Ok(profiles_dir)
    } else {
        entry
            .path
            .parent()
            .ok_or_else(|| format!("{}: has no parent directory", entry.path.display()))
            .map(|p| p.to_path_buf())
    }
}

fn entry_extension(entry: &StrategyFileEntry) -> String {
    Path::new(&entry.name)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "txt".to_string())
}

pub fn duplicate_file(entry: &StrategyFileEntry) -> Result<PathBuf, String> {
    let dir = clone_target_dir(entry)?;

    let stem = Path::new(&entry.name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.name.clone());
    let extension = entry_extension(entry);

    let candidate_path = unique_candidate_path(&dir, &stem, &extension);

    std::fs::copy(&entry.path, &candidate_path)
        .map_err(|error| format!("{}: {}", candidate_path.display(), error))?;
    Ok(candidate_path)
}

fn normalize_clone_name(entry: &StrategyFileEntry, raw: &str) -> String {
    let extension = entry_extension(entry);
    let trimmed = raw.trim();
    if trimmed.to_lowercase().ends_with(&format!(".{}", extension.to_lowercase())) {
        trimmed.to_string()
    } else {
        format!("{}.{}", trimmed, extension)
    }
}

pub fn duplicate_file_as(entry: &StrategyFileEntry, new_name: &str) -> Result<PathBuf, String> {
    let normalized = normalize_clone_name(entry, new_name);

    let name_is_valid = if entry.kind == StrategyFileKind::Zapret1Strategy {
        is_valid_zapret1_name(&normalized)
    } else {
        is_valid_preset_name(&normalized)
    };
    if !name_is_valid {
        return Err(format!("{}: invalid strategy name", normalized));
    }

    let dir = clone_target_dir(entry)?;
    let candidate_path = dir.join(&normalized);
    if candidate_path.exists() {
        return Err(format!("{}: already exists", normalized));
    }

    std::fs::copy(&entry.path, &candidate_path)
        .map_err(|error| format!("{}: {}", candidate_path.display(), error))?;
    Ok(candidate_path)
}

pub fn delete_file(entry: &StrategyFileEntry) -> Result<(), String> {
    std::fs::remove_file(&entry.path).map_err(|error| format!("{}: {}", entry.path.display(), error))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencedList {
    pub kind: ListRefKind,
    pub raw: String,
    pub resolved: Option<PathBuf>,
    pub present: bool,
}

impl ReferencedList {
    pub fn label(&self) -> &'static str {
        match self.kind {
            ListRefKind::HostInclude => "hostlist",
            ListRefKind::HostExclude => "hostlist-exclude",
            ListRefKind::HostAuto => "hostlist-auto",
            ListRefKind::IpInclude => "ipset",
            ListRefKind::IpExclude => "ipset-exclude",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrategyInspection {
    pub target_binary: Option<String>,
    pub tcp_ports: String,
    pub udp_ports: String,
    pub desync_methods: Vec<String>,
    pub lua_scripts: Vec<String>,
    pub queue_numbers: Vec<String>,
    pub referenced_lists: Vec<ReferencedList>,
}

impl StrategyInspection {
    pub fn missing_lists(&self) -> Vec<&ReferencedList> {
        self.referenced_lists.iter().filter(|item| !item.present).collect()
    }

    pub fn is_activatable(&self) -> bool {
        self.missing_lists().is_empty()
    }
}

fn detect_binary(content: &str, args: &[String]) -> Option<String> {
    for candidate in KNOWN_BINARIES {
        if args.iter().any(|arg| arg_names_binary(arg, candidate)) {
            return Some((*candidate).to_string());
        }
    }

    for candidate in KNOWN_BINARIES {
        if content
            .to_lowercase()
            .contains(&format!("{}", candidate.to_lowercase()))
        {
            return Some((*candidate).to_string());
        }
    }

    None
}

fn arg_names_binary(arg: &str, candidate: &str) -> bool {
    let normalized = arg.trim().trim_matches('"').to_lowercase();
    let stem = Path::new(&normalized)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or(normalized);
    let stem = stem.trim_end_matches(".exe");
    stem == candidate
}

fn split_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn collect_prefixed(args: &[String], prefixes: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    for arg in args {
        for prefix in prefixes {
            let Some(value) = arg.strip_prefix(prefix) else {
                continue;
            };
            for item in split_values(value) {
                if !found.contains(&item) {
                    found.push(item);
                }
            }
            break;
        }
    }

    found
}

fn resolve_lists(workspace: &Path, refs: Vec<ListRef>) -> Vec<ReferencedList> {
    refs.into_iter()
        .map(|item| {
            let resolved = resolve_within(workspace, &item.path);
            let present = resolved
                .as_deref()
                .map(list_file_present)
                .unwrap_or(false);
            ReferencedList {
                kind: item.kind,
                raw: item.path,
                resolved,
                present,
            }
        })
        .collect()
}

pub fn inspect_content(content: &str, workspace: &Path) -> StrategyInspection {
    let parsed = parse_preset_content(content, None);

    StrategyInspection {
        target_binary: detect_binary(content, &parsed.args),
        tcp_ports: parsed.tcp_ports.clone(),
        udp_ports: parsed.udp_ports.clone(),
        desync_methods: collect_prefixed(
            &parsed.args,
            &["--dpi-desync=", "--dpi-desync-mode=", "--desync="],
        ),
        lua_scripts: collect_prefixed(&parsed.args, &["--lua-file=", "--lua="]),
        queue_numbers: collect_prefixed(&parsed.args, &["--qnum=", "--queue="]),
        referenced_lists: resolve_lists(workspace, collect_list_refs(&parsed.args)),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Zapret1GroupSummary {
    pub label: String,
    pub cli_line: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Zapret1Inspection {
    pub strategy: Zapret1Strategy,
    pub group_summaries: Vec<Zapret1GroupSummary>,
    pub referenced_lists: Vec<ReferencedList>,
    pub domain_refs: Vec<String>,
    pub ip_literal_refs: Vec<String>,
    pub cli_preview: String,
}

impl Zapret1Inspection {
    pub fn missing_lists(&self) -> Vec<&ReferencedList> {
        self.referenced_lists.iter().filter(|item| !item.present).collect()
    }

    pub fn is_activatable(&self) -> bool {
        self.missing_lists().is_empty()
    }
}

fn zapret1_list_refs(strategy: &Zapret1Strategy) -> (Vec<ListRef>, Vec<String>, Vec<String>) {
    use crate::strategy::zapret1_model::TargetListKind;

    let mut file_refs: Vec<ListRef> = Vec::new();
    let mut domain_refs: Vec<String> = Vec::new();
    let mut ip_literal_refs: Vec<String> = Vec::new();

    for reference in strategy.target_lists() {
        match reference.kind {
            TargetListKind::Hostlist => file_refs.push(ListRef {
                kind: ListRefKind::HostInclude,
                path: reference.value,
            }),
            TargetListKind::HostlistExclude => file_refs.push(ListRef {
                kind: ListRefKind::HostExclude,
                path: reference.value,
            }),
            TargetListKind::HostlistAuto => file_refs.push(ListRef {
                kind: ListRefKind::HostAuto,
                path: reference.value,
            }),
            TargetListKind::Ipset => file_refs.push(ListRef {
                kind: ListRefKind::IpInclude,
                path: reference.value,
            }),
            TargetListKind::IpsetExclude => file_refs.push(ListRef {
                kind: ListRefKind::IpExclude,
                path: reference.value,
            }),
            TargetListKind::HostlistDomains | TargetListKind::HostlistExcludeDomains => {
                for domain in reference.value.split(',') {
                    let domain = domain.trim();
                    if !domain.is_empty() {
                        domain_refs.push(domain.to_string());
                    }
                }
            }
            TargetListKind::IpsetIp | TargetListKind::IpsetExcludeIp => {
                for ip in reference.value.split(',') {
                    let ip = ip.trim();
                    if !ip.is_empty() {
                        ip_literal_refs.push(ip.to_string());
                    }
                }
            }
        }
    }

    (file_refs, domain_refs, ip_literal_refs)
}

pub fn inspect_zapret1_content(content: &str, workspace: &Path, binary: &str) -> Zapret1Inspection {
    let strategy = parse_zapret1_content(content, None);

    let group_summaries = strategy
        .groups
        .iter()
        .map(|group| Zapret1GroupSummary {
            label: group.summary_label(),
            cli_line: crate::strategy::zapret1_model::render_group_cli_public(group),
        })
        .collect();

    let (file_refs, domain_refs, ip_literal_refs) = zapret1_list_refs(&strategy);
    let referenced_lists = resolve_lists(workspace, file_refs);
    let cli_preview = render_cli_preview(&strategy, binary);

    Zapret1Inspection {
        strategy,
        group_summaries,
        referenced_lists,
        domain_refs,
        ip_literal_refs,
        cli_preview,
    }
}

pub fn inspect_zapret1_file(path: &Path, workspace: &Path, binary: &str) -> Result<Zapret1Inspection, String> {
    let content = std::fs::read_to_string(path).map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(inspect_zapret1_content(&content, workspace, binary))
}

pub fn inspect_file(path: &Path, workspace: &Path) -> Result<StrategyInspection, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(inspect_content(&content, workspace))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorInspection {
    Zapret2(StrategyInspection),
    Zapret1(Zapret1Inspection),
}

impl EditorInspection {
    pub fn is_activatable(&self) -> bool {
        match self {
            Self::Zapret2(inspection) => inspection.is_activatable(),
            Self::Zapret1(inspection) => inspection.is_activatable(),
        }
    }
}

pub fn inspect_entry(entry: &StrategyFileEntry, engine: &ZapretEngine) -> Result<EditorInspection, String> {
    match entry.kind {
        StrategyFileKind::Zapret1Strategy => {
            let workspace = crate::strategy::repo_dir();
            let binary = ZapretEngine::Zapret1.binary_name();
            inspect_zapret1_file(&entry.path, &workspace, binary).map(EditorInspection::Zapret1)
        }
        StrategyFileKind::Preset | StrategyFileKind::Profile => {
            inspect_file(&entry.path, &engine.workspace_dir()).map(EditorInspection::Zapret2)
        }
    }
}

pub fn render(
    entries: &[StrategyFileEntry],
    selected_index: usize,
    active_preset_name: Option<&str>,
) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items = vec![];
    let mut index = 0;

    for entry in entries {
        let is_sel = index == selected_index;
        let is_active = (entry.kind == StrategyFileKind::Preset || entry.kind == StrategyFileKind::Zapret1Strategy)
            && Some(entry.name.as_str()) == active_preset_name;

        let marker = if is_active { "✅ " } else { "   " };
        let lock = if entry.read_only { " 🔒" } else { "" };
        let label = format!(" {}{}{}", marker, entry.name, lock);

        items.push(ListItem::new(label).style(if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }));
        index += 1;
    }

    let is_sel = index == selected_index;
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_back"))).style(if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_strategy_editor").into_owned(),
        selected_index,
    )
}

pub fn render_inspector(inspection: &StrategyInspection, read_only: bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    if read_only {
        lines.push("Read-only       : stock preset, edit will save as a new profile".to_string());
    }

    lines.push(format!(
        "Target binary   : {}",
        inspection
            .target_binary
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    ));

    let tcp = if inspection.tcp_ports.is_empty() {
        "none".to_string()
    } else {
        inspection.tcp_ports.clone()
    };
    let udp = if inspection.udp_ports.is_empty() {
        "none".to_string()
    } else {
        inspection.udp_ports.clone()
    };

    lines.push(format!("TCP ports       : {}", tcp));
    lines.push(format!("UDP ports       : {}", udp));

    let methods = if inspection.desync_methods.is_empty() {
        "none".to_string()
    } else {
        inspection.desync_methods.join(", ")
    };
    lines.push(format!("Desync methods  : {}", methods));

    if !inspection.queue_numbers.is_empty() {
        lines.push(format!("Queue numbers   : {}", inspection.queue_numbers.join(", ")));
    }

    if !inspection.lua_scripts.is_empty() {
        lines.push(format!("Lua scripts     : {}", inspection.lua_scripts.join(", ")));
    }

    if inspection.referenced_lists.is_empty() {
        lines.push("Referenced lists: none".to_string());
        return lines;
    }

    lines.push("Referenced lists:".to_string());
    for item in &inspection.referenced_lists {
        lines.push(format!(
            "  [{}] {} {}",
            if item.present { "OK" } else { "MISSING" },
            item.label(),
            item.raw
        ));
    }

    let missing = inspection.missing_lists().len();
    if missing > 0 {
        lines.push(format!(
            "Validation      : {} referenced file(s) missing, activation blocked",
            missing
        ));
    } else {
        lines.push("Validation      : every referenced file exists".to_string());
    }

    lines
}

pub fn render_zapret1_inspector(inspection: &Zapret1Inspection, read_only: bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let strategy = &inspection.strategy;

    if read_only {
        lines.push("Read-only       : stock strategy, edit will save as a new file".to_string());
    }

    let tcp = if strategy.global_tcp_ports.is_empty() {
        "none".to_string()
    } else {
        strategy.global_tcp_ports.clone()
    };
    let udp = if strategy.global_udp_ports.is_empty() {
        "none".to_string()
    } else {
        strategy.global_udp_ports.clone()
    };
    lines.push(format!("TCP ports (wf)  : {}", tcp));
    lines.push(format!("UDP ports (wf)  : {}", udp));

    if strategy.uses_game_filter_tcp || strategy.uses_game_filter_udp {
        lines.push(format!(
            "Game filter     : TCP={} UDP={}",
            strategy.uses_game_filter_tcp, strategy.uses_game_filter_udp
        ));
    }

    let methods = strategy.desync_methods();
    let methods = if methods.is_empty() {
        "none".to_string()
    } else {
        methods.join(", ")
    };
    lines.push(format!("Desync methods  : {}", methods));

    lines.push(format!("Filter groups   : {}", inspection.group_summaries.len()));
    for (position, group) in inspection.group_summaries.iter().enumerate() {
        lines.push(format!("  #{} {}", position + 1, group.label));
    }

    if !inspection.domain_refs.is_empty() {
        lines.push(format!("Inline domains  : {}", inspection.domain_refs.join(", ")));
    }
    if !inspection.ip_literal_refs.is_empty() {
        lines.push(format!("Inline IPs      : {}", inspection.ip_literal_refs.join(", ")));
    }

    if inspection.referenced_lists.is_empty() {
        lines.push("Referenced lists: none".to_string());
    } else {
        lines.push("Referenced lists:".to_string());
        for item in &inspection.referenced_lists {
            lines.push(format!(
                "  [{}] {} {}",
                if item.present { "OK" } else { "MISSING" },
                item.label(),
                item.raw
            ));
        }
    }

    let missing = inspection.missing_lists().len();
    if missing > 0 {
        lines.push(format!(
            "Validation      : {} referenced file(s) missing, activation blocked",
            missing
        ));
    } else {
        lines.push("Validation      : every referenced file exists".to_string());
    }

    lines.push(String::new());
    lines.push("Live CLI preview:".to_string());
    for line in inspection.cli_preview.lines() {
        lines.push(format!("  {}", line));
    }

    lines
}

pub fn render_editor_inspector(inspection: &EditorInspection, read_only: bool) -> Vec<String> {
    match inspection {
        EditorInspection::Zapret2(inspection) => render_inspector(inspection, read_only),
        EditorInspection::Zapret1(inspection) => render_zapret1_inspector(inspection, read_only),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_binary_is_detected_from_arguments() {
        let inspection = inspect_content("nfqws2 --qnum=200 --dpi-desync=fake,multisplit", Path::new("/tmp"));
        assert_eq!(inspection.target_binary.as_deref(), Some("nfqws2"));
    }

    #[test]
    fn desync_methods_are_split_on_commas() {
        let inspection = inspect_content("--dpi-desync=fake,split2,disoob", Path::new("/tmp"));
        assert_eq!(
            inspection.desync_methods,
            vec!["fake".to_string(), "split2".to_string(), "disoob".to_string()]
        );
    }

    #[test]
    fn missing_lists_block_activation() {
        let inspection = inspect_content(
            "--hostlist=lists/absent-list.txt --ipset=lists/absent-ipset.txt",
            Path::new("/tmp/zapret-inspect-missing"),
        );

        assert_eq!(inspection.referenced_lists.len(), 2);
        assert_eq!(inspection.missing_lists().len(), 2);
        assert!(!inspection.is_activatable());
    }

    #[test]
    fn queue_numbers_and_lua_scripts_are_extracted() {
        let inspection = inspect_content("--qnum=200 --lua-file=lua/mutate.lua", Path::new("/tmp"));
        assert_eq!(inspection.queue_numbers, vec!["200".to_string()]);
        assert_eq!(inspection.lua_scripts, vec!["lua/mutate.lua".to_string()]);
    }

    #[test]
    fn the_inspector_reports_validation_state() {
        let inspection = inspect_content("nfqws --dpi-desync=fake", Path::new("/tmp"));
        let lines = render_inspector(&inspection, false);
        assert!(lines.iter().any(|line| line.contains("Target binary")));
        assert!(lines.iter().any(|line| line.contains("Referenced lists: none")));
        assert!(!lines.iter().any(|line| line.starts_with("Read-only")));
    }

    #[test]
    fn zapret2_inspector_marks_stock_presets_read_only() {
        let inspection = inspect_content("nfqws2 --dpi-desync=fake", Path::new("/tmp"));
        let read_only_lines = render_inspector(&inspection, true);
        assert!(read_only_lines.iter().any(|line| line.starts_with("Read-only")));

        let read_write_lines = render_inspector(&inspection, false);
        assert!(!read_write_lines.iter().any(|line| line.starts_with("Read-only")));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zapret_strategy_editor_{}_{}", label, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn inspect_entry_dispatches_to_zapret1_for_bat_files() {
        let dir = temp_dir("z1_dispatch");
        let path = dir.join("general.bat");
        std::fs::write(
            &path,
            "start \"zapret\" \"winws.exe\" --wf-tcp=443 --wf-udp=443 --filter-tcp=443 --dpi-desync=fake --dpi-desync-fake-tls=\"bin/tls.bin\"",
        )
        .unwrap();

        let entry = StrategyFileEntry {
            kind: StrategyFileKind::Zapret1Strategy,
            name: "general.bat".to_string(),
            path,
            read_only: true,
        };

        let inspection = inspect_entry(&entry, &ZapretEngine::Zapret1).expect("inspection should succeed");
        match inspection {
            EditorInspection::Zapret1(zapret1) => {
                assert_eq!(zapret1.strategy.global_tcp_ports, "443");
                assert_eq!(zapret1.group_summaries.len(), 1);
            }
            EditorInspection::Zapret2(_) => panic!("expected a zapret1 inspection"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inspect_entry_dispatches_to_zapret2_for_presets() {
        let dir = temp_dir("z2_dispatch");
        let path = dir.join("preset.txt");
        std::fs::write(&path, "nfqws2 --dpi-desync=fake,split2").unwrap();

        let entry = StrategyFileEntry {
            kind: StrategyFileKind::Preset,
            name: "preset.txt".to_string(),
            path,
            read_only: false,
        };

        let inspection = inspect_entry(&entry, &ZapretEngine::Zapret2).expect("inspection should succeed");
        match inspection {
            EditorInspection::Zapret2(zapret2) => {
                assert_eq!(zapret2.target_binary.as_deref(), Some("nfqws2"));
            }
            EditorInspection::Zapret1(_) => panic!("expected a zapret2 inspection"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zapret1_inspector_marks_read_only_and_shows_cli_preview() {
        let strategy = parse_zapret1_content(
            "--wf-tcp=443 --wf-udp=443 --filter-tcp=443 --hostlist=\"lists/list-general.txt\" --dpi-desync=fake --dpi-desync-fake-tls=\"bin/tls.bin\"",
            None,
        );
        let inspection = Zapret1Inspection {
            strategy,
            group_summaries: vec![Zapret1GroupSummary {
                label: "TCP 443 · desync=fake".to_string(),
                cli_line: "--filter-tcp=443 --dpi-desync=fake".to_string(),
            }],
            referenced_lists: Vec::new(),
            domain_refs: Vec::new(),
            ip_literal_refs: Vec::new(),
            cli_preview: "winws --wf-tcp=443 --wf-udp=443 \\\n  --filter-tcp=443 --dpi-desync=fake".to_string(),
        };

        let lines = render_zapret1_inspector(&inspection, true);
        assert!(lines.iter().any(|line| line.starts_with("Read-only")));
        assert!(lines.iter().any(|line| line.contains("Live CLI preview")));
        assert!(lines.iter().any(|line| line.contains("--dpi-desync=fake")));

        let read_write_lines = render_zapret1_inspector(&inspection, false);
        assert!(!read_write_lines.iter().any(|line| line.starts_with("Read-only")));
    }

    #[test]
    fn zapret2_presets_are_read_only_and_profiles_are_not() {
        let mut entries: Vec<StrategyFileEntry> = Vec::new();
        let presets_dir = temp_dir("z2_presets_ro");
        let profiles_dir = temp_dir("z2_profiles_rw");
        std::fs::write(presets_dir.join("stock.txt"), "nfqws2 --dpi-desync=fake").unwrap();
        std::fs::write(profiles_dir.join("mine.txt"), "nfqws2 --dpi-desync=fake").unwrap();

        scan_dir_read_only(&presets_dir, StrategyFileKind::Preset, true, &mut entries);
        scan_dir_read_only(&profiles_dir, StrategyFileKind::Profile, false, &mut entries);

        let stock = entries.iter().find(|e| e.name == "stock.txt").unwrap();
        let mine = entries.iter().find(|e| e.name == "mine.txt").unwrap();
        assert!(stock.read_only);
        assert!(!mine.read_only);

        let _ = std::fs::remove_dir_all(&presets_dir);
        let _ = std::fs::remove_dir_all(&profiles_dir);
    }

    #[test]
    fn duplicating_a_read_only_preset_redirects_into_profiles_dir() {
        let cache_root = temp_dir("z2_dup_cache_root");
        std::env::set_var("ZAPRET_CACHE_DIR", cache_root.to_string_lossy().into_owned());

        let presets_dir = temp_dir("z2_dup_source");
        let source = presets_dir.join("stock.txt");
        std::fs::write(&source, "nfqws2 --dpi-desync=fake").unwrap();

        let entry = StrategyFileEntry {
            kind: StrategyFileKind::Preset,
            name: "stock.txt".to_string(),
            path: source,
            read_only: true,
        };

        let cloned = duplicate_file(&entry).expect("clone should succeed");
        assert_eq!(cloned.parent().unwrap(), ZapretEngine::Zapret2.profiles_dir());
        assert!(cloned.exists());

        std::env::remove_var("ZAPRET_CACHE_DIR");
        let _ = std::fs::remove_dir_all(&presets_dir);
        let _ = std::fs::remove_dir_all(&cache_root);
    }

    #[test]
    fn duplicate_file_as_uses_the_requested_name_in_profiles_dir() {
        let cache_root = temp_dir("z2_dup_named_cache_root");
        std::env::set_var("ZAPRET_CACHE_DIR", cache_root.to_string_lossy().into_owned());

        let presets_dir = temp_dir("z2_dup_named_source");
        let source = presets_dir.join("stock.txt");
        std::fs::write(&source, "nfqws2 --dpi-desync=fake").unwrap();

        let entry = StrategyFileEntry {
            kind: StrategyFileKind::Preset,
            name: "stock.txt".to_string(),
            path: source,
            read_only: true,
        };

        let cloned = duplicate_file_as(&entry, "my custom strategy").expect("clone should succeed");
        assert_eq!(cloned.parent().unwrap(), ZapretEngine::Zapret2.profiles_dir());
        assert_eq!(cloned.file_name().unwrap().to_string_lossy(), "my custom strategy.txt");
        assert!(cloned.exists());

        let collision = duplicate_file_as(&entry, "my custom strategy");
        assert!(collision.is_err());

        std::env::remove_var("ZAPRET_CACHE_DIR");
        let _ = std::fs::remove_dir_all(&presets_dir);
        let _ = std::fs::remove_dir_all(&cache_root);
    }

    #[test]
    fn duplicate_file_as_preserves_the_bat_extension_for_zapret1() {
        let cache_root = temp_dir("z1_dup_named_cache_root");
        std::env::set_var("ZAPRET_CACHE_DIR", cache_root.to_string_lossy().into_owned());

        let dir = temp_dir("z1_dup_named");
        let source = dir.join("general.bat");
        std::fs::write(&source, "start \"zapret\" \"winws.exe\" --dpi-desync=fake").unwrap();

        let entry = StrategyFileEntry {
            kind: StrategyFileKind::Zapret1Strategy,
            name: "general.bat".to_string(),
            path: source,
            read_only: true,
        };

        let cloned = duplicate_file_as(&entry, "my-general").expect("clone should succeed");
        assert_eq!(cloned.file_name().unwrap().to_string_lossy(), "my-general.bat");
        assert_eq!(cloned.parent().unwrap(), crate::strategy::repo_dir().join(ZAPRET1_CUSTOM_SUBDIR));

        std::env::remove_var("ZAPRET_CACHE_DIR");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&cache_root);
    }
}
