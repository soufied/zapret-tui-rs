use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Rgb(122, 162, 247);
    pub const ACCENT_DIM: Color = Color::Rgb(86, 111, 168);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const MUTED: Color = Color::Rgb(122, 132, 160);
    pub const OK: Color = Color::Rgb(158, 206, 106);
    pub const WARN: Color = Color::Rgb(224, 175, 104);
    pub const BAD: Color = Color::Rgb(247, 118, 142);
    pub const VALUE: Color = Color::Rgb(125, 207, 255);

    pub fn selected_item() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_value() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn normal_item() -> Style {
        Style::default().fg(Self::TEXT)
    }

    pub fn normal_value() -> Style {
        Style::default().fg(Self::VALUE).add_modifier(Modifier::BOLD)
    }

    pub fn dim_item() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn active_value() -> Style {
        Style::default().fg(Self::OK).add_modifier(Modifier::BOLD)
    }

    pub fn inactive_value() -> Style {
        Style::default().fg(Self::BAD).add_modifier(Modifier::BOLD)
    }

    pub fn header_style() -> Style {
        Style::default().fg(Self::ACCENT).add_modifier(Modifier::BOLD)
    }

    pub fn block_title() -> Style {
        Style::default().fg(Self::ACCENT).add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::ACCENT_DIM)
    }

    pub fn border_focus() -> Style {
        Style::default().fg(Self::ACCENT)
    }

    pub fn breadcrumb_active() -> Style {
        Style::default().fg(Self::ACCENT).add_modifier(Modifier::BOLD)
    }

    pub fn breadcrumb_parent() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn breadcrumb_separator() -> Style {
        Style::default().fg(Self::ACCENT_DIM)
    }

    pub fn badge_label() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn badge(color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn status_message() -> Style {
        Style::default().fg(Self::VALUE).add_modifier(Modifier::BOLD)
    }

    pub fn hint() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARN).add_modifier(Modifier::BOLD)
    }
}

pub fn toggle_marker(enabled: bool) -> &'static str {
    if enabled {
        "[x]"
    } else {
        "[ ]"
    }
}

pub fn presence_marker(present: bool) -> &'static str {
    if present {
        "[✓]"
    } else {
        "[✗]"
    }
}
