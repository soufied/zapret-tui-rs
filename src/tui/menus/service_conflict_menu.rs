use crate::inits::ServiceConflict;
use crate::tui::state::AppState;
use crate::tui::theme::Theme;
use ratatui::{
    text::{Line, Span},
    widgets::ListItem,
};

fn conflict_state(conflict: ServiceConflict) -> String {
    let mut parts: Vec<String> = Vec::new();
    if conflict.active {
        parts.push(rust_i18n::t!("srv_conflict_state_active").into_owned());
    }
    if conflict.enabled {
        parts.push(rust_i18n::t!("srv_conflict_state_enabled").into_owned());
    }
    parts.join(", ")
}

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();

    items.push(ListItem::new(Line::from(Span::styled(
        format!(" ⚠ {}", rust_i18n::t!("srv_conflict_banner")),
        Theme::warning(),
    ))));

    if let Some(conflict) = app.service_conflict {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("   {} {}", rust_i18n::t!("srv_conflict_state"), conflict_state(conflict)),
            Theme::dim_item(),
        ))));
    }

    items.push(ListItem::new(Line::from(Span::styled(
        format!(
            "   {}",
            rust_i18n::t!("srv_conflict_detail")
                .replace("{queue}", &crate::firewalls::NFQUEUE_NUM.to_string())
                .replace("{mark}", crate::firewalls::FWMARK_HEX)
        ),
        Theme::dim_item(),
    ))));

    items.push(ListItem::new(Line::from(Span::styled(
        "  ───────────────────────────────────────",
        Theme::dim_item(),
    ))));

    let offset = items.len();
    let choices = [
        format!(" {}", rust_i18n::t!("srv_conflict_remove")),
        format!(" {}", rust_i18n::t!("srv_conflict_cancel")),
    ];

    for (i, choice) in choices.into_iter().enumerate() {
        if i == app.service_conflict_index {
            items.push(ListItem::new(choice).style(Theme::selected_item()));
        } else {
            items.push(ListItem::new(choice).style(Theme::normal_item()));
        }
    }

    (
        items,
        rust_i18n::t!("tui_title_service_conflict").into_owned(),
        app.service_conflict_index + offset,
    )
}
