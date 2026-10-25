use std::collections::HashSet;
use std::net::IpAddr;
use std::path::Path;

use crate::common::config::ZapretEngine;
use crate::diagnostics::blockcheck::targets::clean_domain;
use crate::tui::theme::Theme;
use ratatui::widgets::ListItem;

#[derive(Debug, Clone, Default)]
pub struct ListInspection {
    pub name: String,
    pub bytes: u64,
    pub domains: usize,
    pub addresses: usize,
    pub comments: usize,
    pub blank: usize,
    pub duplicates: usize,
    pub invalid_total: usize,
    pub invalid_examples: Vec<String>,
    pub used_by: Vec<String>,
}

fn is_address(line: &str) -> bool {
    line.split('/').next().unwrap_or(line).parse::<IpAddr>().is_ok()
}

fn referencing_strategies(file_name: &str) -> Vec<String> {
    let mut used = Vec::new();
    for engine in [ZapretEngine::Zapret1, ZapretEngine::Zapret2] {
        for strategy in crate::strategy::get_strategies_for(&engine) {
            let dirs = match engine {
                ZapretEngine::Zapret1 => vec![crate::strategy::repo_dir()],
                ZapretEngine::Zapret2 => vec![engine.presets_dir(), engine.profiles_dir()],
            };
            let referenced = dirs.iter().any(|dir| {
                std::fs::read_to_string(dir.join(&strategy))
                    .map(|text| text.contains(file_name))
                    .unwrap_or(false)
            });
            if referenced {
                used.push(strategy);
            }
        }
    }
    used
}

pub fn inspect_list(path: &Path) -> ListInspection {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut inspection = ListInspection {
        bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        used_by: referencing_strategies(&name),
        name,
        ..ListInspection::default()
    };
    let mut seen: HashSet<String> = HashSet::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            inspection.blank += 1;
        } else if line.starts_with('#') {
            inspection.comments += 1;
        } else if !seen.insert(line.to_ascii_lowercase()) {
            inspection.duplicates += 1;
        } else if is_address(line) {
            inspection.addresses += 1;
        } else if clean_domain(line).is_some() {
            inspection.domains += 1;
        } else {
            inspection.invalid_total += 1;
            if inspection.invalid_examples.len() < 3 {
                inspection.invalid_examples.push(line.to_string());
            }
        }
    }
    inspection
}

pub fn render_inspector(ins: &ListInspection) -> Vec<String> {
    let mut lines = vec![
        format!("File           : {}", ins.name),
        format!("Size           : {} bytes", ins.bytes),
        format!("Domains        : {}", ins.domains),
        format!("Addresses      : {}", ins.addresses),
        format!("Comments       : {}", ins.comments),
        format!("Blank lines    : {}", ins.blank),
        format!("Duplicates     : {}", ins.duplicates),
    ];
    if ins.invalid_total == 0 {
        lines.push("Invalid lines  : none".to_string());
    } else {
        lines.push(format!(
            "Invalid lines  : {} (e.g. {})",
            ins.invalid_total,
            ins.invalid_examples.join(", ")
        ));
    }
    if ins.domains + ins.addresses == 0 {
        lines.push("Warning        : the list has no usable entries".to_string());
    }
    if ins.used_by.is_empty() {
        lines.push("Used by        : no built-in strategy".to_string());
    } else {
        lines.push(format!("Used by        : {}", ins.used_by.join(", ")));
    }
    lines
}

pub fn render(lists_files: &[String], selected_index: usize) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items = vec![];
    let mut index = 0;

    for file in lists_files {
        let is_sel = index == selected_index;

        let filename = std::path::Path::new(file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        items.push(ListItem::new(format!(" {}", filename)).style(if is_sel {
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

    (items, rust_i18n::t!("tui_title_lists").into_owned(), selected_index)
}
