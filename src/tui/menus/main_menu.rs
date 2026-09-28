use crate::tui::state::{AppState, MainMenuState};
use crate::tui::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};
use unicode_width::UnicodeWidthChar;

const MIN_LABEL_WIDTH: usize = 12;
const VARIATION_SELECTOR_16: char = '\u{FE0F}';
const ZERO_WIDTH_JOINER: char = '\u{200D}';

fn is_emoji_presentation_codepoint(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1F000..=0x1FFFF
            | 0x2600..=0x27BF
            | 0x2190..=0x21FF
            | 0x2300..=0x23FF
            | 0x25A0..=0x25FF
            | 0x2B00..=0x2BFF
            | 0x2700..=0x27BF
    )
}

fn terminal_char_width(ch: char) -> usize {
    if ch == VARIATION_SELECTOR_16 || ch == ZERO_WIDTH_JOINER {
        return 0;
    }
    if is_emoji_presentation_codepoint(ch) {
        return 2;
    }
    UnicodeWidthChar::width(ch).unwrap_or(0)
}

fn terminal_display_width(text: &str) -> usize {
    text.chars().map(terminal_char_width).sum()
}

pub fn pad_to_display_width(label: &str, width: usize) -> String {
    let current_width = terminal_display_width(label);
    let padding = width.saturating_sub(current_width);
    let mut padded = String::with_capacity(label.len() + padding);
    padded.push_str(label);
    for _ in 0..padding {
        padded.push(' ');
    }
    padded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_label_is_padded_to_exact_width() {
        let padded = pad_to_display_width("Settings", 12);
        assert_eq!(terminal_display_width(&padded), 12);
    }

    #[test]
    fn emoji_with_variation_selector_counts_as_two_cells() {
        let with_selector = terminal_display_width("\u{1F6E0}\u{FE0F} Settings");
        let without_selector = terminal_display_width("\u{1F6E0} Settings");
        assert_eq!(with_selector, without_selector);
    }

    #[test]
    fn labels_with_and_without_variation_selectors_align_identically() {
        let a = pad_to_display_width("\u{1F6E0}\u{FE0F} Settings", 26);
        let b = pad_to_display_width("\u{1F310} Interface", 26);
        assert_eq!(terminal_display_width(&a), terminal_display_width(&b));
    }
}

enum Row {
    Entry {
        selected: bool,
        label: String,
        value: Option<String>,
    },
    Separator,
}

struct Builder {
    rows: Vec<Row>,
    index: usize,
    selected_index: usize,
}

impl Builder {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            index: 0,
            selected_index: 0,
        }
    }

    fn push(&mut self, selected: bool, label: String, value: Option<String>) {
        if selected {
            self.selected_index = self.index;
        }
        self.index += 1;
        self.rows.push(Row::Entry { selected, label, value });
    }

    fn separator(&mut self) {
        self.index += 1;
        self.rows.push(Row::Separator);
    }

    fn label_width(&self) -> usize {
        self.rows
            .iter()
            .filter_map(|row| match row {
                Row::Entry { label, .. } => Some(terminal_display_width(label)),
                Row::Separator => None,
            })
            .max()
            .unwrap_or(MIN_LABEL_WIDTH)
            .max(MIN_LABEL_WIDTH)
    }

    fn finish(self) -> (Vec<ListItem<'static>>, usize) {
        let label_width = self.label_width();
        let mut items = Vec::with_capacity(self.rows.len());

        for row in self.rows {
            match row {
                Row::Entry { selected, label, value } => {
                    let marker = if selected { "▸ " } else { "  " };
                    let label_style = if selected {
                        Theme::selected_item()
                    } else {
                        Theme::normal_item()
                    };

                    let mut spans = vec![Span::styled(
                        format!("{}{}", marker, pad_to_display_width(&label, label_width)),
                        label_style,
                    )];

                    if let Some(value) = value {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(
                            format!("‹ {} ›", value),
                            if selected {
                                Theme::selected_value()
                            } else {
                                Theme::normal_value()
                            },
                        ));
                    }

                    items.push(ListItem::new(Line::from(spans)));
                }
                Row::Separator => {
                    let rule: String = std::iter::repeat('─').take(label_width).collect();
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("  {}", rule),
                        Theme::dim_item(),
                    ))));
                }
            }
        }

        (items, self.selected_index)
    }
}

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut b = Builder::new();

    #[cfg(target_os = "windows")]
    b.push(
        app.main_menu == MainMenuState::DefenderSettings,
        rust_i18n::t!("menu_main_defender").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::DownloadDeps,
        rust_i18n::t!("menu_main_downloader").into_owned(),
        None,
    );

    b.separator();

    b.push(
        app.main_menu == MainMenuState::Engine,
        rust_i18n::t!("menu_main_engine").into_owned(),
        Some(app.engine.to_string()),
    );

    b.push(
        app.main_menu == MainMenuState::Interface,
        rust_i18n::t!("menu_main_interface").into_owned(),
        Some(
            app.interfaces
                .get(app.selected_interface)
                .cloned()
                .unwrap_or_else(|| "any".to_string()),
        ),
    );

    b.push(
        app.main_menu == MainMenuState::Strategy,
        rust_i18n::t!("menu_main_strategy").into_owned(),
        Some(
            app.strategies
                .get(app.selected_strategy)
                .cloned()
                .unwrap_or_else(|| rust_i18n::t!("val_none").into_owned()),
        ),
    );

    if app.engine.supports_game_filter() {
        let gamefilter = {
            let mut parts: Vec<&str> = Vec::new();
            if app.tcp_gamefilter {
                parts.push("TCP");
            }
            if app.udp_gamefilter {
                parts.push("UDP");
            }
            if parts.is_empty() {
                rust_i18n::t!("val_off").into_owned()
            } else {
                parts.join("+")
            }
        };
        b.push(
            app.main_menu == MainMenuState::GamefilterSettings,
            rust_i18n::t!("menu_main_gamefilter").into_owned(),
            Some(gamefilter),
        );
    }

    #[cfg(target_os = "linux")]
    b.push(
        app.main_menu == MainMenuState::BackendSettings,
        rust_i18n::t!("menu_main_backend").into_owned(),
        Some(app.selected_backend.to_string()),
    );

    let ipset_value = if crate::ipset::engine_uses_global_ipset_mode(&app.engine) {
        app.available_ipset_modes
            .get(app.selected_ipset_mode)
            .map(|m| m.to_string())
            .unwrap_or_else(|| rust_i18n::t!("val_none").into_owned())
    } else {
        crate::ipset::legacy_mode_notice()
    };

    b.push(
        app.main_menu == MainMenuState::IpsetMode,
        rust_i18n::t!("menu_main_ipset").into_owned(),
        Some(ipset_value),
    );

    if app.engine.supports_ttl_autopick() {
        b.push(
            app.main_menu == MainMenuState::TtlAutopick,
            rust_i18n::t!("menu_main_ttl").into_owned(),
            Some(match app.dpi_desync_ttl {
                Some(n) => n.to_string(),
                None => rust_i18n::t!("ttl_auto").into_owned(),
            }),
        );
    }

    b.separator();

    b.push(
        app.main_menu == MainMenuState::ListsEditor,
        rust_i18n::t!("menu_main_lists").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::StrategyEditor,
        rust_i18n::t!("menu_main_strategy_editor").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::Autotune,
        rust_i18n::t!("menu_main_autotune").into_owned(),
        None,
    );

    if app.engine.supports_active_fakes() {
        b.push(
            app.main_menu == MainMenuState::FakesSettings,
            rust_i18n::t!("menu_main_fakes").into_owned(),
            None,
        );
    }

    b.push(
        app.main_menu == MainMenuState::Settings,
        rust_i18n::t!("menu_main_settings").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::ServiceSettings,
        rust_i18n::t!("menu_main_service").into_owned(),
        None,
    );

    b.separator();

    b.push(
        app.main_menu == MainMenuState::Run,
        rust_i18n::t!("menu_main_run").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::Quit,
        rust_i18n::t!("menu_main_quit").into_owned(),
        None,
    );

    let title = rust_i18n::t!("menu_main_title").into_owned();
    let (items, selected_index) = b.finish();
    (items, title, selected_index)
}
