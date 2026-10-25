use crate::common::config::ZapretEngine;
use crate::common::download::{self, STRAT_REC_VER, ZAPRET_REC_VER};
use crate::common::manifest;
use crate::tui::state::{AppState, DownloadDepsMenuState, VersionTarget};
use crate::tui::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy)]
enum Component {
    Zapret1Nfqws,
    Zapret2Nfqws,
    Zapret1Strategies,
    Zapret2Strategies,
}

struct Facts {
    title: String,
    installed: bool,
    version: Option<String>,
    target: String,
    recommended: Option<String>,
    path: PathBuf,
    footprint: String,
    modified: Option<SystemTime>,
}

fn entry(label: String, status: Option<bool>, selected: bool) -> ListItem<'static> {
    let label_style = if selected {
        Theme::selected_item()
    } else {
        Theme::normal_item()
    };
    let mut spans = vec![Span::styled(format!(" {}", label), label_style)];
    if let Some(installed) = status {
        let (text, style) = if installed {
            (rust_i18n::t!("settings_editor_installed"), Theme::active_value())
        } else {
            (rust_i18n::t!("settings_editor_not_installed"), Theme::inactive_value())
        };
        spans.push(Span::styled(format!("  [{}]", text), style));
    }
    ListItem::new(Line::from(spans))
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn file_size(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) => human_size(meta.len()),
        Err(_) => "-".to_string(),
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn count_entries(dir: &Path, extension: Option<&str>) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| match extension {
                    Some(ext) => e.path().extension().and_then(|x| x.to_str()) == Some(ext),
                    None => true,
                })
                .count()
        })
        .unwrap_or(0)
}

fn age(modified: SystemTime) -> String {
    let secs = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        .as_secs();
    match secs {
        0..=59 => "less than a minute ago".to_string(),
        60..=3599 => format!("{} min ago", secs / 60),
        3600..=86_399 => format!("{} h ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
}

fn target_label(target: &VersionTarget, recommended: &str) -> String {
    match target {
        VersionTarget::Recommended => format!("recommended ({})", recommended),
        VersionTarget::Latest => "latest".to_string(),
        VersionTarget::Tag(tag) if tag.is_empty() => "no tag selected".to_string(),
        VersionTarget::Tag(tag) => tag.clone(),
    }
}

fn facts(app: &AppState, component: Component) -> Facts {
    match component {
        Component::Zapret1Nfqws => {
            let path = ZapretEngine::Zapret1.binary_path();
            Facts {
                title: rust_i18n::t!("menu_dl_zapret").into_owned(),
                installed: download::check_nfqws_installed_for(&ZapretEngine::Zapret1),
                version: manifest::get("zapret1-nfqws"),
                target: target_label(&app.nfqws_target, ZAPRET_REC_VER),
                recommended: Some(ZAPRET_REC_VER.to_string()),
                footprint: file_size(&path),
                modified: modified_time(&path),
                path,
            }
        }
        Component::Zapret2Nfqws => {
            let path = ZapretEngine::Zapret2.binary_path();
            Facts {
                title: rust_i18n::t!("menu_dl_zapret2").into_owned(),
                installed: download::check_nfqws_installed_for(&ZapretEngine::Zapret2),
                version: manifest::get("zapret2-nfqws"),
                target: "latest release".to_string(),
                recommended: None,
                footprint: file_size(&path),
                modified: modified_time(&path),
                path,
            }
        }
        Component::Zapret1Strategies => {
            let path = crate::strategy::repo_dir();
            Facts {
                title: rust_i18n::t!("menu_dl_strat").into_owned(),
                installed: download::check_strategies_installed_for(&ZapretEngine::Zapret1),
                version: manifest::get("zapret1-strategies"),
                target: target_label(&app.strat_target, &STRAT_REC_VER[..7]),
                recommended: Some(STRAT_REC_VER.to_string()),
                footprint: format!("{} .bat files", count_entries(&path, Some("bat"))),
                modified: modified_time(&path),
                path,
            }
        }
        Component::Zapret2Strategies => {
            let path = ZapretEngine::Zapret2.workspace_dir();
            Facts {
                title: rust_i18n::t!("menu_dl_zapret2_strat").into_owned(),
                installed: download::check_zapret2_payload_installed(),
                version: manifest::get("zapret2-strategies"),
                target: "main branch archive".to_string(),
                recommended: Some("main".to_string()),
                footprint: format!(
                    "{} presets",
                    count_entries(&ZapretEngine::Zapret2.presets_dir(), None)
                ),
                modified: modified_time(&path),
                path,
            }
        }
    }
}

fn status_line(f: &Facts) -> String {
    if !f.installed {
        return "Not installed".to_string();
    }
    match (&f.version, &f.recommended) {
        (None, _) => "Installed, version not recorded (reinstall to record it)".to_string(),
        (Some(version), Some(rec)) if version == rec => "Up to date with the recommended version".to_string(),
        (Some(version), Some(_)) => format!("Differs from recommended (installed {})", version),
        (Some(_), None) => "Installed".to_string(),
    }
}

fn describe(f: &Facts) -> Vec<String> {
    let mut lines = vec![
        format!("Status   : {}", if f.installed { "Installed" } else { "Not installed" }),
        format!("Version  : {}", f.version.as_deref().unwrap_or("unknown")),
        format!("Target   : {}", f.target),
        format!("Check    : {}", status_line(f)),
        format!("Path     : {}", f.path.display()),
        format!("Footprint: {}", f.footprint),
    ];
    match f.modified {
        Some(time) => lines.push(format!("Updated  : {}", age(time))),
        None => lines.push("Updated  : -".to_string()),
    }
    lines
}

pub fn details(app: &AppState) -> Vec<String> {
    match app.download_deps_menu {
        DownloadDepsMenuState::ZapretDownloader => describe(&facts(app, Component::Zapret1Nfqws)),
        DownloadDepsMenuState::Zapret2Downloader => describe(&facts(app, Component::Zapret2Nfqws)),
        DownloadDepsMenuState::StrategiesDownloader => describe(&facts(app, Component::Zapret1Strategies)),
        DownloadDepsMenuState::Zapret2Strategies => describe(&facts(app, Component::Zapret2Strategies)),
        DownloadDepsMenuState::DownloadDefaults => {
            let mut lines = Vec::new();
            for component in [
                Component::Zapret1Nfqws,
                Component::Zapret2Nfqws,
                Component::Zapret1Strategies,
                Component::Zapret2Strategies,
            ] {
                let f = facts(app, component);
                lines.push(format!("== {}", f.title));
                lines.extend(describe(&f));
                lines.push(String::new());
            }
            lines
        }
        DownloadDepsMenuState::Back => Vec::new(),
    }
}

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let nfqws1 = download::check_nfqws_installed_for(&ZapretEngine::Zapret1);
    let nfqws2 = download::check_nfqws_installed_for(&ZapretEngine::Zapret2);
    let strat1 = download::check_strategies_installed_for(&ZapretEngine::Zapret1);
    let strat2 = download::check_zapret2_payload_installed();

    let rows = [
        (
            DownloadDepsMenuState::ZapretDownloader,
            rust_i18n::t!("menu_dl_zapret").into_owned(),
            Some(nfqws1),
        ),
        (
            DownloadDepsMenuState::Zapret2Downloader,
            rust_i18n::t!("menu_dl_zapret2").into_owned(),
            Some(nfqws2),
        ),
        (
            DownloadDepsMenuState::StrategiesDownloader,
            rust_i18n::t!("menu_dl_strat").into_owned(),
            Some(strat1),
        ),
        (
            DownloadDepsMenuState::Zapret2Strategies,
            rust_i18n::t!("menu_dl_zapret2_strat").into_owned(),
            Some(strat2),
        ),
        (
            DownloadDepsMenuState::DownloadDefaults,
            rust_i18n::t!("menu_dl_defaults").into_owned(),
            Some(nfqws1 && nfqws2 && strat1 && strat2),
        ),
        (
            DownloadDepsMenuState::Back,
            rust_i18n::t!("menu_dl_back").into_owned(),
            None,
        ),
    ];

    let mut selected_index = 0;
    let mut items = Vec::with_capacity(rows.len());
    for (index, (state, label, status)) in rows.into_iter().enumerate() {
        let selected = app.download_deps_menu == state;
        if selected {
            selected_index = index;
        }
        items.push(entry(label, status, selected));
    }

    (items, rust_i18n::t!("menu_dl_title").into_owned(), selected_index)
}
