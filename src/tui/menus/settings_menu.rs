use crate::tui::state::{AppState, EditorKind, SettingsMenuState};
use crate::tui::theme::{presence_marker, Theme};
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

const LABEL_WIDTH: usize = 26;

fn row(label: String, value: Option<Span<'static>>, selected: bool) -> ListItem<'static> {
    let marker = if selected { "▸ " } else { "  " };
    let label_style = if selected {
        Theme::selected_item()
    } else {
        Theme::normal_item()
    };

    let mut spans = vec![Span::styled(
        format!("{}{:<width$}", marker, label, width = LABEL_WIDTH),
        label_style,
    )];
    if let Some(value) = value {
        spans.push(Span::raw(" "));
        spans.push(value);
    }
    ListItem::new(Line::from(spans))
}

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    for (idx, state) in SettingsMenuState::all().iter().enumerate() {
        let selected = *state == app.settings_menu;
        if selected {
            selected_index = idx;
        }

        let item = match state {
            SettingsMenuState::Editor => {
                let label = crate::utils::active_editor_label();
                let available = crate::utils::resolve_editor().is_some();
                let style = if available {
                    Theme::normal_value()
                } else {
                    Theme::inactive_value()
                };
                row(
                    rust_i18n::t!("settings_editor").into_owned(),
                    Some(Span::styled(format!("‹ {} ›", label), style)),
                    selected,
                )
            }
            SettingsMenuState::BackupLists => {
                let value = if app.backup_lists {
                    rust_i18n::t!("val_on").into_owned()
                } else {
                    rust_i18n::t!("val_off").into_owned()
                };
                let style = if app.backup_lists {
                    Theme::active_value()
                } else {
                    Theme::inactive_value()
                };
                row(
                    rust_i18n::t!("settings_backup").into_owned(),
                    Some(Span::styled(format!("‹ {} ›", value), style)),
                    selected,
                )
            }
            SettingsMenuState::ViewLogs => row(
                rust_i18n::t!("settings_logs").into_owned(),
                Some(Span::styled(
                    rust_i18n::t!("settings_logs_open").into_owned(),
                    Theme::hint(),
                )),
                selected,
            ),
            SettingsMenuState::Back => row(rust_i18n::t!("menu_dl_back").into_owned(), None, selected),
        };
        items.push(item);
    }

    (items, rust_i18n::t!("menu_settings_title").into_owned(), selected_index)
}

pub fn render_editor(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let configured = crate::config::load_editor();
    let configured = configured.trim();

    for (idx, entry) in app.editor_entries.iter().enumerate() {
        let selected = idx == app.editor_index;

        let is_active = match entry.kind {
            EditorKind::Auto => configured.is_empty(),
            EditorKind::Binary => configured == entry.command,
            EditorKind::Custom => !configured.is_empty() && !entry.command.is_empty(),
        };

        let name = match entry.kind {
            EditorKind::Binary => format!("{} ({})", entry.label, entry.command),
            EditorKind::Custom => {
                if app.editor_custom_editing && selected {
                    format!("{}: {}_", entry.label, app.editor_custom_buf)
                } else if entry.command.is_empty() {
                    entry.label.clone()
                } else {
                    format!("{}: {}", entry.label, entry.command)
                }
            }
            EditorKind::Auto => entry.label.clone(),
        };

        let status_text = match entry.kind {
            EditorKind::Auto => {
                if entry.installed {
                    rust_i18n::t!("settings_editor_resolved").into_owned()
                } else {
                    rust_i18n::t!("settings_editor_none").into_owned()
                }
            }
            _ => {
                if entry.installed {
                    rust_i18n::t!("settings_editor_installed").into_owned()
                } else {
                    rust_i18n::t!("settings_editor_not_installed").into_owned()
                }
            }
        };

        let status_style = if entry.installed {
            Theme::active_value()
        } else {
            Theme::inactive_value()
        };

        let marker = if selected { "▸ " } else { "  " };
        let active_marker = if is_active { "●" } else { "○" };

        let mut spans = vec![
            Span::styled(
                format!("{}{} {:<28}", marker, active_marker, name),
                if selected {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::raw(" "),
            Span::styled(presence_marker(entry.installed), status_style),
            Span::raw(" "),
            Span::styled(status_text, status_style),
        ];

        if is_active {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                rust_i18n::t!("settings_editor_active").into_owned(),
                Theme::hint(),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    let back_index = app.editor_entries.len();
    let back_selected = app.editor_index >= back_index;
    items.push(
        ListItem::new(format!("{}{}", if back_selected { "▸ " } else { "  " }, rust_i18n::t!("menu_dl_back")))
            .style(if back_selected {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            }),
    );

    let selected_index = app.editor_index.min(back_index);
    (
        items,
        rust_i18n::t!("tui_title_settings_editor").into_owned(),
        selected_index,
    )
}
