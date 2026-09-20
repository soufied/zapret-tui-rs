use crate::tui::state::{AppState, MainMenuState};
use crate::tui::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

const LABEL_WIDTH: usize = 28;

struct Builder {
    items: Vec<ListItem<'static>>,
    index: usize,
    selected_index: usize,
}

impl Builder {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
            selected_index: 0,
        }
    }

    fn push(&mut self, selected: bool, label: String, value: Option<String>) {
        if selected {
            self.selected_index = self.index;
        }
        self.index += 1;

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
            spans.push(Span::styled(
                format!("‹ {} ›", value),
                if selected {
                    Theme::selected_value()
                } else {
                    Theme::normal_value()
                },
            ));
        }

        self.items.push(ListItem::new(Line::from(spans)));
    }

    fn separator(&mut self) {
        self.index += 1;
        self.items.push(ListItem::new(Line::from(Span::styled(
            "  ───────────────────────────────────────",
            Theme::dim_item(),
        ))));
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

    b.push(
        app.main_menu == MainMenuState::IpsetMode,
        rust_i18n::t!("menu_main_ipset").into_owned(),
        Some(
            app.available_ipset_modes
                .get(app.selected_ipset_mode)
                .map(|m| m.to_string())
                .unwrap_or_else(|| rust_i18n::t!("val_none").into_owned()),
        ),
    );

    b.push(
        app.main_menu == MainMenuState::TtlAutopick,
        rust_i18n::t!("menu_main_ttl").into_owned(),
        Some(match app.dpi_desync_ttl {
            Some(n) => n.to_string(),
            None => rust_i18n::t!("ttl_auto").into_owned(),
        }),
    );

    b.separator();

    b.push(
        app.main_menu == MainMenuState::ListsEditor,
        rust_i18n::t!("menu_main_lists").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::Autotune,
        rust_i18n::t!("menu_main_autotune").into_owned(),
        None,
    );

    b.push(
        app.main_menu == MainMenuState::FakesSettings,
        rust_i18n::t!("menu_main_fakes").into_owned(),
        None,
    );

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

    (
        b.items,
        rust_i18n::t!("menu_main_title").into_owned(),
        b.selected_index,
    )
}
