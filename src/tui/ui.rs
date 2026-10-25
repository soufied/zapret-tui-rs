#[cfg(target_os = "windows")]
use crossterm::terminal::Clear;
#[cfg(target_os = "windows")]
use crossterm::terminal::ClearType;
use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use crate::tui::menus;
use crate::tui::state::{ActiveScreen, AppState, VersionTarget};
use crate::tui::theme::{presence_marker, Theme};

const TICK_MS: u64 = 200;
const STATUS_REFRESH_TICKS: u32 = 15;

pub fn spawn_event_reader() -> EventReader {
    let (tx, rx) = mpsc::channel();
    let paused = Arc::new(AtomicBool::new(false));
    let paused_reader = Arc::clone(&paused);
    std::thread::spawn(move || loop {
        if paused_reader.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        match crossterm::event::poll(Duration::from_millis(50)) {
            Ok(true) => match crossterm::event::read() {
                Ok(event) => {
                    if tx.send(event).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(20));
                }
            },
            Ok(false) => {}
            Err(_) => {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    });
    EventReader { rx, paused }
}

pub struct EventReader {
    rx: Receiver<Event>,
    paused: Arc<AtomicBool>,
}

impl EventReader {
    pub fn rx(&self) -> &Receiver<Event> {
        &self.rx
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(150));
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
}

fn drain_events(rx: &Receiver<Event>) {
    while rx.try_recv().is_ok() {}
}

fn wait_for_quit(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: &Receiver<Event>,
    title: &str,
    elapsed: u64,
) -> Result<(), io::Error> {
    let hint = rust_i18n::t!("blockcheck_done").into_owned();
    loop {
        terminal.draw(|f| crate::tui::live::draw(f, title, &hint, elapsed))?;
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Key(key)) => {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') || key.code == KeyCode::Esc {
                    break;
                }
            }
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn wait_for_results(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: &Receiver<Event>,
    title: &str,
    columns: &[&str],
    rows: &[Vec<String>],
) -> Result<(), io::Error> {
    let hint = rust_i18n::t!("blockcheck_done").into_owned();
    let mut scroll = 0usize;
    loop {
        let capacity = crate::tui::live::results_capacity(terminal.size()?.height);
        let max_scroll = rows.len().saturating_sub(capacity);
        scroll = scroll.min(max_scroll);
        terminal.draw(|f| crate::tui::live::draw_results(f, title, &hint, columns, rows, scroll))?;
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Key(key)) => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                KeyCode::Up => scroll = scroll.saturating_sub(1),
                KeyCode::Down => scroll = (scroll + 1).min(max_scroll),
                KeyCode::PageUp => scroll = scroll.saturating_sub(capacity),
                KeyCode::PageDown => scroll = (scroll + capacity).min(max_scroll),
                KeyCode::Home => scroll = 0,
                KeyCode::End => scroll = max_scroll,
                _ => {}
            },
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn wait_for_key(rx: &Receiver<Event>) -> Result<(), io::Error> {
    enable_raw_mode()?;
    drain_events(rx);
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Key(_)) => break,
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn begin_external_output(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    execute!(terminal.backend_mut(), Clear(ClearType::All))?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn begin_external_output(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn end_external_output(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: &Receiver<Event>,
) -> Result<(), io::Error> {
    enable_raw_mode()?;
    terminal.clear()?;
    drain_events(rx);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn end_external_output(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: &Receiver<Event>,
) -> Result<(), io::Error> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;
    drain_events(rx);
    Ok(())
}

fn with_terminal_suspended<T>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    reader: &EventReader,
    action: impl FnOnce() -> T,
) -> Result<T, io::Error> {
    let rx = reader.rx();
    reader.pause();

    let suspended = begin_external_output(terminal);
    let outcome = if suspended.is_ok() { Some(action()) } else { None };
    let restored = end_external_output(terminal, rx);

    reader.resume();
    drain_events(rx);

    suspended?;
    restored?;
    outcome.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "terminal suspend failed"))
}

fn run_download(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    rx: &Receiver<Event>,
    download: impl FnOnce() -> Result<(), String>,
) -> Result<Result<(), String>, io::Error> {
    begin_external_output(terminal)?;

    let res = download();

    match &res {
        Ok(_) => println!("{}", rust_i18n::t!("msg_dl_ok")),
        Err(err_msg) => {
            println!("{}{}", rust_i18n::t!("msg_dl_fail"), err_msg);
        }
    }
    println!("{}", rust_i18n::t!("msg_dl_key"));

    wait_for_key(rx)?;
    end_external_output(terminal, rx)?;

    Ok(res)
}

fn breadcrumb_line(app: &AppState) -> Line<'static> {
    let trail = app.breadcrumb();
    let last = trail.len().saturating_sub(1);
    let mut spans: Vec<Span<'static>> = Vec::new();

    for (idx, part) in trail.into_iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" › ", Theme::breadcrumb_separator()));
        }
        let style = if idx == last {
            Theme::breadcrumb_active()
        } else {
            Theme::breadcrumb_parent()
        };
        spans.push(Span::styled(part, style));
    }

    Line::from(spans)
}

fn push_badge(spans: &mut Vec<Span<'static>>, label: &str, value: String, color: Color) {
    if !spans.is_empty() {
        spans.push(Span::styled(" │ ", Theme::breadcrumb_separator()));
    }
    spans.push(Span::styled(format!("{} ", label), Theme::badge_label()));
    spans.push(Span::styled(value, Theme::badge(color)));
}

fn service_kind() -> String {
    #[cfg(target_os = "windows")]
    {
        rust_i18n::t!("status_srv_win").into_owned()
    }
    #[cfg(target_os = "linux")]
    {
        crate::inits::detect_init_system()
            .map(|t| t.as_str().to_string())
            .unwrap_or_else(|| rust_i18n::t!("status_srv_unknown").into_owned())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        rust_i18n::t!("status_srv_unknown").into_owned()
    }
}

fn firewall_name(app: &AppState) -> String {
    #[cfg(target_os = "linux")]
    {
        app.selected_backend.to_string()
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        "WinDivert".to_string()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = app;
        rust_i18n::t!("status_srv_unknown").into_owned()
    }
}

fn badge_line(app: &AppState) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    let engine_color = if app.engine.uses_presets() {
        Theme::ACCENT
    } else {
        Theme::VALUE
    };
    push_badge(
        &mut spans,
        &rust_i18n::t!("badge_engine"),
        app.engine.to_string(),
        engine_color,
    );

    push_badge(
        &mut spans,
        &rust_i18n::t!("badge_firewall"),
        firewall_name(app),
        Theme::VALUE,
    );

    let (svc_text, svc_color) = if !app.service_installed {
        (rust_i18n::t!("status_srv_not_inst").into_owned(), Theme::MUTED)
    } else if app.service_active {
        (rust_i18n::t!("status_srv_active").into_owned(), Theme::OK)
    } else {
        (rust_i18n::t!("status_srv_stopped").into_owned(), Theme::WARN)
    };
    push_badge(
        &mut spans,
        &rust_i18n::t!("badge_service"),
        format!("{} ({})", svc_text, service_kind()),
        svc_color,
    );

    let (dpi_text, dpi_color) = if app.dpi_running {
        (rust_i18n::t!("status_dpi_running").into_owned(), Theme::OK)
    } else {
        (rust_i18n::t!("status_dpi_idle").into_owned(), Theme::MUTED)
    };
    push_badge(&mut spans, &rust_i18n::t!("badge_daemon"), dpi_text, dpi_color);

    let deps_ok = app.nfqws_installed && app.strategies_installed;
    push_badge(
        &mut spans,
        &rust_i18n::t!("badge_deps"),
        format!(
            "{} {} {} {}",
            rust_i18n::t!("badge_deps_bin"),
            presence_marker(app.nfqws_installed),
            rust_i18n::t!("badge_deps_strat"),
            presence_marker(app.strategies_installed),
        ),
        if deps_ok { Theme::OK } else { Theme::BAD },
    );

    Line::from(spans)
}

fn screen_body(app: &AppState) -> (Vec<ratatui::widgets::ListItem<'static>>, String, usize) {
    match app.active_screen {
        ActiveScreen::Main => menus::main_menu::render(app),
        #[cfg(target_os = "windows")]
        ActiveScreen::DefenderSubmenu => menus::defender_menu::render(app),
        ActiveScreen::StrategySubmenu => menus::strategy_menu::render(app),
        ActiveScreen::DownloadDepsSubmenu => menus::download_menu::render(app),
        ActiveScreen::DownloadZapretSubmenu => menus::download_submenu::render(app, true),
        ActiveScreen::DownloadStrategiesSubmenu => menus::download_submenu::render(app, false),
        ActiveScreen::GamefilterSubmenu => menus::gamefilter_menu::render(app),
        ActiveScreen::FakesSubmenu => menus::fakes_menu::render(app),
        ActiveScreen::FakesSelectSubmenu => {
            menus::fakes_menu::render_select(&app.fakes_state, &app.fakes_select_for, app.fakes_select_index)
        }
        ActiveScreen::ZapretTagSelect => menus::tag_menu::render(
            &app.available_nfqws_tags,
            app.nfqws_tag_index,
            &rust_i18n::t!("menu_tag_title_zapret"),
        ),
        ActiveScreen::StrategyTagSelect => menus::tag_menu::render(
            &app.available_strat_tags,
            app.strat_tag_index,
            &rust_i18n::t!("menu_tag_title_strat"),
        ),
        ActiveScreen::ServiceSubmenu => menus::service_menu::render(app),
        ActiveScreen::ServiceConflictSubmenu => menus::service_conflict_menu::render(app),
        ActiveScreen::ListsEditorSubmenu => menus::lists_menu::render(&app.lists_files, app.lists_menu_index),
        ActiveScreen::StrategyEditorSubmenu => {
            let active_name = app.strategies.get(app.selected_strategy).map(|s| s.as_str());
            menus::strategy_editor_menu::render(&app.strategy_editor_files, app.strategy_editor_index, active_name)
        }
        ActiveScreen::AutotuneSubmenu => menus::autotune_menu::render_config(app),
        ActiveScreen::AutotuneEditDomainsSubmenu => menus::autotune_menu::render_domain_files(app),
        ActiveScreen::AutotuneProtocolsSubmenu => {
            menus::autotune_menu::render_protocols(app, app.autotune_protocols_menu)
        }
        ActiveScreen::AutotuneBlockChecksSubmenu => {
            menus::autotune_menu::render_blockchecks(app, app.autotune_block_checks_menu)
        }
        ActiveScreen::AutotunePresetSelectionSubmenu => menus::autotune_menu::render_presets(app),
        ActiveScreen::AutotuneZ2PresetsSubmenu => menus::autotune_menu::render_z2_presets(app),
        ActiveScreen::AutotuneZ2TargetsSubmenu => menus::autotune_menu::render_z2_targets(app),
        ActiveScreen::AutotuneStrategiesSubmenu => {
            menus::autotune_menu::render_strategies(app, app.autotune_strat_index)
        }
        ActiveScreen::AutotuneResultsSubmenu => menus::autotune_menu::render_results(app, app.autotune_results_index),
        ActiveScreen::SettingsSubmenu => menus::settings_menu::render(app),
        ActiveScreen::SettingsEditorSubmenu => menus::settings_menu::render_editor(app),
        ActiveScreen::LogViewer => menus::log_menu::render(&app.log_lines, app.log_scroll),
    }
}

fn draw_split_panel(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    list_title: String,
    items: Vec<ratatui::widgets::ListItem<'static>>,
    selected_index: usize,
    inspector_title: String,
    inspector_lines: Vec<Line<'static>>,
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(area);

    let list_block = Block::default()
        .title(Span::styled(list_title, Theme::block_title()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());
    let list_area = list_block.inner(columns[0]);
    f.render_widget(list_block, columns[0]);

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::ITALIC));
    let mut list_state = ListState::default();
    list_state.select(Some(selected_index));
    f.render_stateful_widget(list, list_area, &mut list_state);

    let inspector_block = Block::default()
        .title(Span::styled(inspector_title, Theme::block_title()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());
    let inspector_area = inspector_block.inner(columns[1]);
    f.render_widget(inspector_block, columns[1]);
    f.render_widget(
        Paragraph::new(inspector_lines).wrap(Wrap { trim: true }),
        inspector_area,
    );
}

fn draw(f: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focus())
        .title(Span::styled(app.screen_title(), Theme::header_style()));
    let header = Paragraph::new(breadcrumb_line(app))
        .alignment(Alignment::Center)
        .block(header_block);
    f.render_widget(header, chunks[0]);

    let badges = Paragraph::new(badge_line(app)).alignment(Alignment::Center);
    f.render_widget(badges, chunks[1]);

    let (items, block_title, selected_index) = screen_body(app);

    let counter = if items.is_empty() {
        String::new()
    } else {
        format!("  ·  {}/{}", selected_index.min(items.len() - 1) + 1, items.len())
    };

    if app.active_screen == ActiveScreen::ListsEditorSubmenu {
        let inspector_lines: Vec<Line<'static>> = match &app.lists_inspection {
            Some(inspection) => menus::lists_menu::render_inspector(inspection)
                .into_iter()
                .map(Line::from)
                .collect(),
            None => vec![Line::from(rust_i18n::t!("tui_lists_inspector_empty").into_owned())],
        };
        draw_split_panel(
            f,
            chunks[2],
            format!("{}{}", block_title, counter),
            items,
            selected_index,
            rust_i18n::t!("tui_title_list_inspector").into_owned(),
            inspector_lines,
        );
    } else if app.active_screen == ActiveScreen::DownloadDepsSubmenu {
        let details_lines: Vec<Line<'static>> = menus::download_menu::details(app)
            .into_iter()
            .map(Line::from)
            .collect();
        draw_split_panel(
            f,
            chunks[2],
            format!("{}{}", block_title, counter),
            items,
            selected_index,
            rust_i18n::t!("tui_title_download_details").into_owned(),
            details_lines,
        );
    } else if app.active_screen == ActiveScreen::StrategyEditorSubmenu {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
            .split(chunks[2]);

        let list_block = Block::default()
            .title(Span::styled(format!("{}{}", block_title, counter), Theme::block_title()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border());
        let list_area = list_block.inner(columns[0]);
        f.render_widget(list_block, columns[0]);

        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::ITALIC));
        let mut list_state = ListState::default();
        list_state.select(Some(selected_index));
        f.render_stateful_widget(list, list_area, &mut list_state);

        let inspector_block = Block::default()
            .title(Span::styled(
                rust_i18n::t!("tui_title_strategy_inspector").into_owned(),
                Theme::block_title(),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border());
        let inspector_area = inspector_block.inner(columns[1]);
        f.render_widget(inspector_block, columns[1]);

        let inspector_lines: Vec<Line> = match &app.strategy_editor_inspection {
            Some(inspection) => {
                let read_only = app
                    .strategy_editor_files
                    .get(app.strategy_editor_index)
                    .map(|entry| entry.read_only)
                    .unwrap_or(false);
                menus::strategy_editor_menu::render_editor_inspector(inspection, read_only)
                    .into_iter()
                    .map(Line::from)
                    .collect()
            }
            None => vec![Line::from(rust_i18n::t!("tui_strategy_inspector_empty").into_owned())],
        };
        let inspector = Paragraph::new(inspector_lines).wrap(Wrap { trim: true });
        f.render_widget(inspector, inspector_area);

        if app.strategy_editor_new_name_editing {
            let prompt_key = if app.strategy_editor_clone_source.is_some() {
                "prompt_strat_editor_clone_name"
            } else {
                "prompt_strat_editor_new_name"
            };
            let prompt = Paragraph::new(Line::from(format!(
                "{}{}",
                rust_i18n::t!(prompt_key),
                app.strategy_editor_new_name_buf
            )))
            .alignment(Alignment::Center)
            .style(Theme::status_message());
            f.render_widget(prompt, list_area);
        }
    } else {
        let list_block = Block::default()
            .title(Span::styled(format!("{}{}", block_title, counter), Theme::block_title()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border());

        let inner = list_block.inner(chunks[2]);
        f.render_widget(list_block, chunks[2]);

        let list_area = if app.active_screen == ActiveScreen::AutotuneSubmenu {
            let sub = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(1)].as_ref())
                .split(inner);
            let warning = Paragraph::new(Line::from(Span::styled(
                rust_i18n::t!("autotune_warning_disable").into_owned(),
                Theme::warning(),
            )))
            .alignment(Alignment::Center);
            f.render_widget(warning, sub[0]);
            sub[1]
        } else {
            inner
        };

        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::ITALIC));
        let mut list_state = ListState::default();
        list_state.select(Some(selected_index));
        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());

    let help_style = if app.status_message.is_some() {
        Theme::status_message()
    } else {
        Theme::hint()
    };

    let help = Paragraph::new(Line::from(Span::styled(app.help_text(), help_style)))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(help_block);
    f.render_widget(help, chunks[3]);
}

fn handle_key(app: &mut AppState, key: KeyEvent) {
    if app.autotune_request_editing {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => app.autotune_request_buf.push(c),
            KeyCode::Backspace => {
                app.autotune_request_buf.pop();
            }
            KeyCode::Enter => {
                if let Ok(n) = app.autotune_request_buf.parse::<usize>() {
                    app.autotune_config.num_requests = n.max(1);
                }
                app.autotune_request_editing = false;
                app.autotune_request_buf.clear();
            }
            KeyCode::Esc => {
                app.autotune_request_editing = false;
                app.autotune_request_buf.clear();
            }
            _ => {}
        }
        return;
    }

    if app.editor_custom_editing {
        match key.code {
            KeyCode::Char(c) => app.editor_custom_buf.push(c),
            KeyCode::Backspace => {
                app.editor_custom_buf.pop();
            }
            KeyCode::Enter => app.commit_custom_editor(),
            KeyCode::Esc => {
                app.editor_custom_editing = false;
                app.editor_custom_buf.clear();
                app.status_message = None;
            }
            _ => {}
        }
        return;
    }

    if app.active_screen == ActiveScreen::StrategyEditorSubmenu {
        if app.strategy_editor_new_name_editing {
            match key.code {
                KeyCode::Char(c) => app.strategy_editor_new_name_buf.push(c),
                KeyCode::Backspace => {
                    app.strategy_editor_new_name_buf.pop();
                }
                KeyCode::Enter => app.commit_new_strategy_file(),
                KeyCode::Esc => app.cancel_new_strategy_file(),
                _ => {}
            }
            return;
        }

        if app.strategy_editor_delete_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_selected_strategy_file(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.strategy_editor_delete_confirm = false;
                    app.status_message = None;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.prev_menu(),
            KeyCode::Down | KeyCode::Char('j') => app.next_menu(),
            KeyCode::Home => {
                app.strategy_editor_index = 0;
                app.refresh_strategy_editor_inspection();
            }
            KeyCode::End => {
                app.strategy_editor_index = app.strategy_editor_files.len();
                app.refresh_strategy_editor_inspection();
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if app.strategy_editor_index < app.strategy_editor_files.len() {
                    app.open_highlighted_strategy_file();
                } else {
                    app.active_screen = ActiveScreen::Main;
                    app.status_message = None;
                }
            }
            KeyCode::Char('a') | KeyCode::Char(' ') => app.activate_selected_strategy_file(),
            KeyCode::Char('s') => app.should_sync_community_strategies = true,
            KeyCode::Char('n') => app.begin_new_strategy_file(),
            KeyCode::Char('d') => app.duplicate_selected_strategy_file(),
            KeyCode::Char('x') | KeyCode::Delete => app.begin_delete_selected_strategy_file(),
            KeyCode::Char('q') | KeyCode::Esc => {
                app.active_screen = ActiveScreen::Main;
                app.status_message = None;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.prev_menu(),
        KeyCode::Down | KeyCode::Char('j') => app.next_menu(),
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.prev_menu();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.next_menu();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.is_ttl_autopick_selected() {
                app.change_ttl(false);
            } else {
                app.cycle_current(false);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.is_ttl_autopick_selected() {
                app.change_ttl(true);
            } else {
                app.cycle_current(true);
            }
        }
        KeyCode::Enter => {
            if app.is_ttl_autopick_selected() {
                if app.check_dependencies() {
                    app.should_run_ttl = true;
                }
            } else {
                app.cycle_current(true);
            }
        }
        KeyCode::Char(' ') => {
            if app.is_ttl_autopick_selected() {
                app.change_ttl(true);
            } else {
                app.cycle_current(true);
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => match app.active_screen {
            ActiveScreen::AutotuneSubmenu => {
                app.active_screen = ActiveScreen::Main;
            }
            ActiveScreen::AutotuneProtocolsSubmenu
            | ActiveScreen::AutotuneBlockChecksSubmenu
            | ActiveScreen::AutotunePresetSelectionSubmenu
            | ActiveScreen::AutotuneStrategiesSubmenu
            | ActiveScreen::AutotuneZ2PresetsSubmenu
            | ActiveScreen::AutotuneZ2TargetsSubmenu
            | ActiveScreen::AutotuneResultsSubmenu
            | ActiveScreen::AutotuneEditDomainsSubmenu => {
                app.active_screen = ActiveScreen::AutotuneSubmenu;
            }
            ActiveScreen::FakesSelectSubmenu => {
                app.active_screen = ActiveScreen::FakesSubmenu;
            }
            ActiveScreen::ServiceConflictSubmenu => {
                app.cancel_service_conflict();
            }
            ActiveScreen::SettingsEditorSubmenu | ActiveScreen::LogViewer => {
                app.active_screen = ActiveScreen::SettingsSubmenu;
            }
            ActiveScreen::Main => {
                app.should_quit = true;
            }
            _ => {
                app.active_screen = ActiveScreen::Main;
            }
        },
        _ => {}
    }

    match app.active_screen {
        ActiveScreen::DownloadDepsSubmenu
        | ActiveScreen::DownloadZapretSubmenu
        | ActiveScreen::DownloadStrategiesSubmenu
        | ActiveScreen::ZapretTagSelect
        | ActiveScreen::StrategyTagSelect => {
            app.refresh_dep_status();
        }
        ActiveScreen::ServiceSubmenu => {
            app.refresh_service_status();
        }
        _ => {}
    }
}

fn handle_event(app: &mut AppState, event: Event) {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            handle_key(app, key);
        }
    }
}

pub fn run_tui(app: &mut AppState, reader: &EventReader) -> Result<(), io::Error> {
    let rx = reader.rx();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    drain_events(rx);

    let mut dirty = true;
    let mut ticks: u32 = 0;

    loop {
        if dirty {
            terminal.draw(|f| draw(f, app))?;
            dirty = false;
        }

        match rx.recv_timeout(Duration::from_millis(TICK_MS)) {
            Ok(event) => {
                handle_event(app, event);
                while let Ok(pending) = rx.try_recv() {
                    handle_event(app, pending);
                }
                dirty = true;
            }
            Err(RecvTimeoutError::Timeout) => {
                ticks += 1;
                if ticks >= STATUS_REFRESH_TICKS {
                    ticks = 0;
                    let before = app.status_fingerprint();
                    app.refresh_service_status();
                    app.refresh_runtime_status();
                    app.refresh_dep_status();
                    if before != app.status_fingerprint() {
                        dirty = true;
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        if app.should_download_zapret {
            app.should_download_zapret = false;

            let nfqws_target_string;
            let nfqws_ver = match &app.nfqws_target {
                VersionTarget::Recommended => crate::common::download::ZAPRET_REC_VER,
                VersionTarget::Latest => "latest",
                VersionTarget::Tag(t) => {
                    nfqws_target_string = t.clone();
                    &nfqws_target_string
                }
            };

            let res = run_download(&mut terminal, rx, || {
                crate::common::download::install_dependencies(nfqws_ver, "skip")
            })?;

            if let Err(e) = res {
                app.show_error(e.to_string());
            } else {
                app.status_message = Some(rust_i18n::t!("msg_dl_zapret_ok").into_owned());
                app.active_screen = ActiveScreen::DownloadZapretSubmenu;
                app.refresh_dep_status();
            }
            dirty = true;
        }

        if app.should_download_zapret2 {
            app.should_download_zapret2 = false;

            let res = run_download(&mut terminal, rx, || crate::common::download::install_zapret2("latest"))?;

            if let Err(e) = res {
                app.show_error(e.to_string());
            } else {
                app.status_message = Some(rust_i18n::t!("msg_dl_zapret2_ok").into_owned());
                app.active_screen = ActiveScreen::DownloadDepsSubmenu;
                app.reload_strategies();
                app.reload_z2_presets();
                app.reload_z2_lists();
                app.refresh_dep_status();
            }
            dirty = true;
        }

        if app.should_download_zapret2_strategies {
            app.should_download_zapret2_strategies = false;

            let res = run_download(&mut terminal, rx, crate::common::download::download_zapret2_strategies)?;

            if let Err(e) = res {
                app.show_error(e.to_string());
            } else {
                app.status_message = Some(rust_i18n::t!("msg_dl_zapret2_strat_ok").into_owned());
                app.active_screen = ActiveScreen::DownloadDepsSubmenu;
                app.reload_strategies();
                app.reload_z2_presets();
                app.reload_z2_lists();
                app.refresh_dep_status();
            }
            dirty = true;
        }

        if app.should_download_strategies {
            app.should_download_strategies = false;

            let strat_target_string;
            let strat_ver = match &app.strat_target {
                VersionTarget::Recommended => "recommended",
                VersionTarget::Latest => "latest",
                VersionTarget::Tag(t) => {
                    strat_target_string = t.clone();
                    &strat_target_string
                }
            };

            let res = run_download(&mut terminal, rx, || {
                crate::common::download::install_dependencies("skip", strat_ver)
            })?;

            if let Err(e) = res {
                app.show_error(e.to_string());
            } else {
                app.status_message = Some(rust_i18n::t!("msg_dl_strat_ok").into_owned());
                app.strategies = crate::strategy::get_strategies();
                app.active_screen = ActiveScreen::DownloadStrategiesSubmenu;
                app.refresh_dep_status();
            }
            dirty = true;
        }

        if app.should_download_defaults {
            app.should_download_defaults = false;

            let res = run_download(&mut terminal, rx, crate::common::download::install_everything)?;

            if let Err(e) = res {
                app.show_error(e.to_string());
            } else {
                app.status_message = Some(rust_i18n::t!("msg_dl_all_ok").into_owned());
                app.active_screen = ActiveScreen::DownloadDepsSubmenu;
            }

            app.reload_strategies();
            app.reload_z2_presets();
            app.reload_z2_lists();
            app.refresh_dep_status();
            dirty = true;
        }

        if let Some(file_path) = app.should_open_editor.take() {
            let return_screen = app.active_screen;

            let resolution = crate::common::utils::resolve_editor_detailed();
            if let Some(skipped) = &resolution.skipped_configured {
                app.status_message = Some(format!(
                    "{}{}{}",
                    rust_i18n::t!("msg_editor_unavailable_prefix"),
                    skipped,
                    rust_i18n::t!("msg_editor_unavailable_suffix")
                ));
            }

            let needs_terminal = !matches!(
                resolution
                    .editor
                    .as_ref()
                    .map(|editor| crate::platform::launcher::classify_editor(&editor.command)),
                Some(crate::platform::launcher::EditorKind::Graphical)
            );

            let opened = if needs_terminal {
                with_terminal_suspended(&mut terminal, reader, || {
                    crate::common::utils::open_editor_detailed(&file_path)
                })?
            } else {
                crate::common::utils::open_editor_detailed(&file_path)
            };

            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            match opened {
                Ok(launch) => match launch.session {
                    crate::common::utils::EditorSession::Detached(command) => {
                        app.status_message = Some(format!("{} -> {}", command, file_name));
                    }
                    crate::common::utils::EditorSession::Closed(_) => {
                        app.status_message =
                            Some(format!("{}{}", rust_i18n::t!("msg_closed_editor"), file_name));
                    }
                },
                Err(error) => {
                    app.show_error(error.to_string());
                }
            }

            app.active_screen = return_screen;
            if return_screen == ActiveScreen::ListsEditorSubmenu {
                app.refresh_ipset_status();
                app.refresh_lists_inspection();
            }
            if return_screen == ActiveScreen::StrategyEditorSubmenu {
                app.refresh_strategy_editor_inspection();
            }
            dirty = true;
        }

        if app.should_sync_community_strategies {
            app.should_sync_community_strategies = false;

            match crate::common::ingest::sync_community_strategies(None) {
                Ok(summary) => {
                    app.status_message = Some(summary.render().join("\n"));
                    app.refresh_strategy_editor_files();
                    app.refresh_strategy_editor_inspection();
                    app.reload_strategies();
                    app.reload_z2_presets();
                    app.reload_z2_lists();
                }
                Err(error) => app.show_error(error),
            }
            dirty = true;
        }

        if app.should_run_blockcheck {
            app.should_run_blockcheck = false;
            crate::diagnostics::blockcheck::CANCEL.store(false, Ordering::SeqCst);
            let interface = app
                .interfaces
                .get(app.selected_interface)
                .cloned()
                .unwrap_or_else(|| "any".to_string());
            #[cfg(target_os = "linux")]
            let backend: &dyn crate::firewalls::FirewallBackend = &app.selected_backend;
            #[cfg(target_os = "windows")]
            let backend: &dyn crate::firewalls::FirewallBackend = &crate::firewalls::windivert::WinDivertBackend;
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            let backend: &dyn crate::firewalls::FirewallBackend = &crate::firewalls::UnsupportedBackend;

            let opts = crate::diagnostics::blockcheck::Options {
                engine: app.engine.clone(),
                interface,
                names: app.blockcheck_names(),
                limit: 0,
                per_list: 5,
                services: Vec::new(),
                domains: app.blockcheck_domains(),
            };
            let start_time = std::time::Instant::now();
            drain_events(rx);
            crate::diagnostics::events::begin_live();

            terminal.clear()?;
            let live_term = std::cell::RefCell::new(&mut terminal);
            let result = crate::diagnostics::blockcheck::run(&opts, backend, &|done, total| {
                crate::diagnostics::events::set_progress(done, total);
                let _ = live_term.borrow_mut().draw(|f| {
                    crate::tui::live::draw(
                        f,
                        &rust_i18n::t!("tui_title_blockcheck"),
                        &rust_i18n::t!("blockcheck_running"),
                        start_time.elapsed().as_secs(),
                    )
                });

                while let Ok(event) = rx.try_recv() {
                    if let Event::Key(key) = event {
                        if key.code == KeyCode::Char('q')
                            || key.code == KeyCode::Char('Q')
                            || key.code == KeyCode::Esc
                        {
                            crate::diagnostics::blockcheck::CANCEL.store(true, Ordering::SeqCst);
                            return false;
                        }
                    }
                }
                true
            });
            drop(live_term);
            let elapsed = start_time.elapsed().as_secs();
            let title = rust_i18n::t!("tui_title_blockcheck").into_owned();
            match result {
                Ok(report) => {
                    let rows: Vec<Vec<String>> = report
                        .records
                        .iter()
                        .map(crate::diagnostics::blockcheck::table_cells)
                        .collect();
                    wait_for_results(
                        &mut terminal,
                        rx,
                        &title,
                        crate::diagnostics::blockcheck::TABLE_COLUMNS,
                        &rows,
                    )?;
                }
                Err(e) => {
                    crate::diagnostics::events::say(format!("{}{}", rust_i18n::t!("msg_err"), e));
                    wait_for_quit(&mut terminal, rx, &title, elapsed)?;
                }
            }
            crate::diagnostics::events::end_live();
        }

        if app.should_run_ttl {
            app.should_run_ttl = false;

            begin_external_output(&mut terminal)?;

            println!("{}", rust_i18n::t!("ttl_running"));
            println!();

            let strategy = app.strategies.get(app.selected_strategy).cloned().unwrap_or_default();
            let interface = app
                .interfaces
                .get(app.selected_interface)
                .map(|s| s.as_str())
                .unwrap_or("any");
            #[cfg(target_os = "linux")]
            let backend: &dyn crate::firewalls::FirewallBackend = &app.selected_backend;
            #[cfg(target_os = "windows")]
            let backend: &dyn crate::firewalls::FirewallBackend = &crate::firewalls::windivert::WinDivertBackend;
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            let backend: &dyn crate::firewalls::FirewallBackend = &crate::firewalls::UnsupportedBackend;

            let result = if strategy.is_empty() {
                Err(rust_i18n::t!("msg_no_strat").into_owned())
            } else {
                crate::common::ttl::autopick_ttl(&strategy, interface, backend)
            };

            println!();
            match &result {
                Ok(ttl) => {
                    let _ = crate::common::config::save_ttl(Some(*ttl));
                    println!("{} {}", rust_i18n::t!("ttl_found"), ttl);
                }
                Err(e) => {
                    println!("{}{}", rust_i18n::t!("msg_err"), e);
                }
            }
            println!();
            println!("{}", rust_i18n::t!("msg_dl_key"));

            wait_for_key(rx)?;
            end_external_output(&mut terminal, rx)?;

            match result {
                Ok(ttl) => {
                    app.dpi_desync_ttl = Some(ttl);
                    app.status_message = Some(format!("{} {}", rust_i18n::t!("ttl_found"), ttl));
                }
                Err(e) => app.show_error(e),
            }
            dirty = true;
        }

        if app.should_run || app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
