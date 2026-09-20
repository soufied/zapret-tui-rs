use crate::tui::state::AppState;
use crate::tui::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut menu_items = vec![];

    if !app.service_installed {
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_install")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_back")));
    } else if app.service_active {
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_stop")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_restart")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_uninstall")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_back")));
    } else {
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_start")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_uninstall")));
        menu_items.push(format!(" {}", rust_i18n::t!("menu_srv_back")));
    }

    let mut items: Vec<ListItem<'static>> = Vec::new();

    if app.service_conflict.is_some() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" ⚠ {}", rust_i18n::t!("srv_conflict_banner")),
            Theme::warning(),
        ))));
        items.push(ListItem::new(Line::from(Span::styled(
            format!("   {}", rust_i18n::t!("srv_conflict_hint")),
            Theme::dim_item(),
        ))));
        items.push(ListItem::new(Line::from(Span::styled(
            "  ───────────────────────────────────────",
            Theme::dim_item(),
        ))));
    }

    let offset = items.len();
    let selected_index = app.service_menu_index;

    for (i, m) in menu_items.into_iter().enumerate() {
        if i == selected_index {
            items.push(ListItem::new(m).style(Theme::selected_item()));
        } else {
            items.push(ListItem::new(m).style(Theme::normal_item()));
        }
    }

    (
        items,
        rust_i18n::t!("menu_srv_title").into_owned(),
        selected_index + offset,
    )
}
