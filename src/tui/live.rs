
use unicode_width::UnicodeWidthStr;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

use crate::diagnostics::events::{self, Snapshot};
use crate::tui::menus::main_menu::pad_to_display_width;
use crate::tui::theme::Theme;

const COLUMN_WIDTH: usize = 16;
const FIRST_COLUMN_WIDTH: usize = 30;

const RESULTS_CHROME: u16 = 4;

pub fn results_capacity(height: u16) -> usize {
    height.saturating_sub(RESULTS_CHROME) as usize
}

pub fn draw_results(
    f: &mut Frame,
    title: &str,
    hint: &str,
    columns: &[&str],
    rows: &[Vec<String>],
    scroll: usize,
) {
    f.render_widget(Clear, f.size());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref())
        .split(f.size());

    let widths = column_widths(columns, rows);
    let capacity = results_capacity(f.size().height);
    let shown_end = (scroll + capacity).min(rows.len());
    let mut lines: Vec<Line> = Vec::with_capacity(capacity + 1);
    lines.push(Line::from(Span::styled(format_row(columns, &widths), Theme::header_style())));
    for row in rows.iter().skip(scroll).take(capacity) {
        lines.push(Line::from(Span::styled(format_row(row, &widths), Theme::normal_value())));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ({}-{}/{}) ", title, scroll + 1, shown_end, rows.len()))
        .border_style(Theme::border());
    f.render_widget(Paragraph::new(lines).block(block), chunks[0]);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hint.to_string(), Theme::hint()))),
        chunks[1],
    );
}

fn column_widths(header: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    header
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let longest = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| UnicodeWidthStr::width(c.as_str()))
                .max()
                .unwrap_or(0);
            longest.max(UnicodeWidthStr::width(*h))
        })
        .collect()
}

fn format_row<S: AsRef<str>>(cells: &[S], widths: &[usize]) -> String {
    cells
        .iter()
        .zip(widths)
        .map(|(cell, &width)| pad_to_display_width(cell.as_ref(), width))
        .collect::<Vec<_>>()
        .join("  ")
}

pub fn draw(f: &mut Frame, title: &str, hint: &str, elapsed: u64) {
    f.render_widget(Clear, f.size());
    let snap = events::snapshot();
    let table_rows = table_height(&snap);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(table_rows),
                Constraint::Min(3),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_gauge(f, chunks[0], &snap, title, elapsed);
    draw_table(f, chunks[1], &snap);
    draw_log(f, chunks[2], &snap);

    let hint_line = Paragraph::new(Line::from(Span::styled(hint.to_string(), Theme::hint())));
    f.render_widget(hint_line, chunks[3]);
}

fn table_height(snap: &Snapshot) -> u16 {
    if snap.header.is_empty() {
        return 0;
    }
    (snap.rows.len().min(8) as u16) + 3
}

fn draw_gauge(f: &mut Frame, area: ratatui::layout::Rect, snap: &Snapshot, title: &str, elapsed: u64) {
    let ratio = if snap.total == 0 {
        0.0
    } else {
        (snap.done as f64 / snap.total as f64).clamp(0.0, 1.0)
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Theme::border()),
        )
        .gauge_style(Theme::active_value())
        .ratio(ratio)
        .label(format!(
            "{}/{} ({:.0}%)  {} {:02}:{:02}",
            snap.done,
            snap.total,
            ratio * 100.0,
            rust_i18n::t!("autotune_time_elapsed"),
            elapsed / 60,
            elapsed % 60
        ));
    f.render_widget(gauge, area);
}

fn draw_table(f: &mut Frame, area: ratatui::layout::Rect, snap: &Snapshot) {
    if snap.header.is_empty() || area.height == 0 {
        return;
    }
    let visible = area.height.saturating_sub(3) as usize;
    let start = snap.rows.len().saturating_sub(visible);

    let mut lines: Vec<Line> = Vec::with_capacity(visible + 1);
    lines.push(Line::from(Span::styled(
        format_cells(&snap.header),
        Theme::header_style(),
    )));
    for row in &snap.rows[start..] {
        lines.push(Line::from(Span::styled(format_cells(&row.cells), Theme::normal_value())));
    }

    let table = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).border_style(Theme::border()));
    f.render_widget(table, area);
}

fn format_cells(cells: &[String]) -> String {
    cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            let width = if i == 0 { FIRST_COLUMN_WIDTH } else { COLUMN_WIDTH };
            pad_to_display_width(cell, width)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn draw_log(f: &mut Frame, area: ratatui::layout::Rect, snap: &Snapshot) {
    let visible = area.height.saturating_sub(2) as usize;
    let start = snap.lines.len().saturating_sub(visible);
    let lines: Vec<Line> = snap.lines[start..]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Theme::normal_value())))
        .collect();
    let log = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", rust_i18n::t!("autotune_progress")))
            .border_style(Theme::border()),
    );
    f.render_widget(log, area);
}
