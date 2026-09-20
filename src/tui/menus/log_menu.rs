use crate::tui::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::ListItem,
};

fn line_style(line: &str) -> Style {
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("failed") || lower.contains("cannot") {
        Theme::inactive_value()
    } else if lower.contains("warn") {
        Theme::warning()
    } else if lower.contains("started") || lower.contains("ok") {
        Theme::active_value()
    } else {
        Theme::normal_item()
    }
}

pub fn render(lines: &[String], scroll: usize) -> (Vec<ListItem<'static>>, String, usize) {
    let items: Vec<ListItem<'static>> = lines
        .iter()
        .map(|line| ListItem::new(Line::from(Span::styled(format!(" {}", line), line_style(line)))))
        .collect();

    let selected = if items.is_empty() {
        0
    } else {
        scroll.min(items.len() - 1)
    };

    let title = format!(
        "{} ({}/{})",
        rust_i18n::t!("tui_title_logs"),
        selected + 1,
        items.len().max(1)
    );

    (items, title, selected)
}
