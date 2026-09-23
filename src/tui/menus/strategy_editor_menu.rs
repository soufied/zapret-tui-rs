use crate::config::ZapretEngine;
use crate::strategy::discovery::is_valid_preset_name;
use crate::strategy::parser::parse_preset_content;
use crate::strategy::refs::{collect_list_refs, list_file_present, resolve_within, ListRef, ListRefKind};
use crate::tui::theme::Theme;
use ratatui::widgets::ListItem;
use std::path::{Path, PathBuf};

const KNOWN_BINARIES: &[&str] = &["nfqws2", "nfqws", "tpws", "winws"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyFileKind {
    Preset,
    Profile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyFileEntry {
    pub kind: StrategyFileKind,
    pub name: String,
    pub path: PathBuf,
}

fn scan_dir(dir: &Path, kind: StrategyFileKind, out: &mut Vec<StrategyFileEntry>) {
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
        });
    }
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

pub fn list_files(engine: &ZapretEngine) -> Vec<StrategyFileEntry> {
    let mut entries: Vec<StrategyFileEntry> = Vec::new();
    scan_dir(&engine.presets_dir(), StrategyFileKind::Preset, &mut entries);
    scan_dir(&engine.profiles_dir(), StrategyFileKind::Profile, &mut entries);
    entries.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
    entries
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

pub fn duplicate_file(entry: &StrategyFileEntry) -> Result<PathBuf, String> {
    let dir = entry
        .path
        .parent()
        .ok_or_else(|| format!("{}: has no parent directory", entry.path.display()))?;

    let stem = Path::new(&entry.name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.name.clone());
    let extension = Path::new(&entry.name)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "txt".to_string());

    let mut candidate_name = format!("{}_copy.{}", stem, extension);
    let mut candidate_path = dir.join(&candidate_name);
    let mut suffix = 2;
    while candidate_path.exists() {
        candidate_name = format!("{}_copy{}.{}", stem, suffix, extension);
        candidate_path = dir.join(&candidate_name);
        suffix += 1;
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

pub fn inspect_file(path: &Path, workspace: &Path) -> Result<StrategyInspection, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: {}", path.display(), error))?;
    Ok(inspect_content(&content, workspace))
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
        let is_active = entry.kind == StrategyFileKind::Preset && Some(entry.name.as_str()) == active_preset_name;

        let marker = if is_active { "✅ " } else { "   " };
        let label = format!(" {}{}", marker, entry.name);

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

pub fn render_inspector(inspection: &StrategyInspection) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

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
        let lines = render_inspector(&inspection);
        assert!(lines.iter().any(|line| line.contains("Target binary")));
        assert!(lines.iter().any(|line| line.contains("Referenced lists: none")));
    }
}
