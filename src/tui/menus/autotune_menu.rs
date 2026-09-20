use crate::autotune::PRESETS;
use crate::tui::state::{AppState, AutotuneBlockChecksState, AutotuneMenuState, AutotuneProtocolsState};
use crate::tui::theme::{toggle_marker, Theme};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
};

fn on_off(v: bool) -> &'static str {
    if v {
        "ON"
    } else {
        "OFF"
    }
}

pub fn render_config(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    if app.engine.uses_presets() {
        return render_config_zapret2(app);
    }
    let mut items: Vec<ListItem<'static>> = Vec::new();

    let is_sel = app.autotune_menu == AutotuneMenuState::PresetSelection;
    let preset_names: Vec<&str> = app
        .autotune_config
        .preset_indices
        .iter()
        .filter_map(|&i| if i < PRESETS.len() { Some(PRESETS[i].name) } else { None })
        .collect();
    let preset_label = if preset_names.is_empty() {
        rust_i18n::t!("menu_autotune_preset_none").into_owned()
    } else {
        format!("[ {} ]", preset_names.join(", "))
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", rust_i18n::t!("menu_autotune_domains")),
            if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        ),
        Span::styled(
            format!("< {} >", preset_label),
            if is_sel {
                Theme::selected_value()
            } else {
                Theme::normal_value()
            },
        ),
    ])));

    let is_sel = app.autotune_menu == AutotuneMenuState::NumRequests;
    items.push(ListItem::new({
        let value = if app.autotune_request_editing {
            let cursor = if (app.autotune_request_buf.len() as u64).is_multiple_of(2) {
                "_"
            } else {
                " "
            };
            format!("< {} >", app.autotune_request_buf.clone() + cursor)
        } else {
            format!("< {} >", app.autotune_config.num_requests)
        };
        Line::from(vec![
            Span::styled(
                format!(" {}: ", rust_i18n::t!("menu_autotune_requests")),
                if is_sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                value,
                if is_sel {
                    Theme::selected_value()
                } else {
                    Theme::normal_value()
                },
            ),
        ])
    }));

    let is_sel = app.autotune_menu == AutotuneMenuState::Strategies;
    let strat_count = app.autotune_config.strategy_indices.len();
    let strat_label: String = if strat_count == 0 && !app.strategies.is_empty() {
        rust_i18n::t!("menu_autotune_strat_none").into_owned()
    } else {
        format!("{} / {}", strat_count, app.strategies.len())
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", rust_i18n::t!("menu_autotune_strategies")),
            if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        ),
        Span::styled(
            format!("< {} >", strat_label),
            if is_sel {
                Theme::selected_value()
            } else {
                Theme::normal_value()
            },
        ),
    ])));

    let is_sel = app.autotune_menu == AutotuneMenuState::Protocols;
    let proto_status = format!(
        "HTTP:{} TLS1.2:{} TLS1.3:{} QUIC:{}",
        on_off(app.autotune_config.check_http),
        on_off(app.autotune_config.check_tls12),
        on_off(app.autotune_config.check_tls13),
        on_off(app.autotune_config.check_quic),
    );
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", rust_i18n::t!("menu_autotune_protocols")),
            if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        ),
        Span::styled(
            format!("< {} >", proto_status),
            if is_sel {
                Theme::selected_value()
            } else {
                Theme::normal_value()
            },
        ),
    ])));

    let is_sel = app.autotune_menu == AutotuneMenuState::BlockChecks;
    let enabled_count = app.autotune_config.block_checks.count_enabled();
    let bc_status = if enabled_count == 0 {
        rust_i18n::t!("val_off").into_owned()
    } else if enabled_count == 6 {
        rust_i18n::t!("val_on").into_owned()
    } else {
        format!("{}/6", enabled_count)
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", rust_i18n::t!("menu_autotune_blockchecks")),
            if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        ),
        Span::styled(
            format!("< {} >", bc_status),
            if is_sel {
                Theme::selected_value()
            } else {
                Theme::normal_value()
            },
        ),
    ])));

    let is_sel = app.autotune_menu == AutotuneMenuState::EditDomains;
    items.push(ListItem::new(Line::from(vec![Span::styled(
        format!(" {}: ", rust_i18n::t!("menu_autotune_edit_domains")),
        if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        },
    )])));

    let is_sel = app.autotune_menu == AutotuneMenuState::Results;
    let has_file = app.has_autotune_results_file;
    let results_label = if has_file {
        rust_i18n::t!("menu_autotune_view")
    } else {
        rust_i18n::t!("menu_autotune_no_results")
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!(" {}: ", rust_i18n::t!("menu_autotune_results")),
            if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        ),
        Span::styled(
            if has_file {
                format!("< {} >", results_label)
            } else {
                results_label.to_string()
            },
            if is_sel {
                Theme::selected_value()
            } else {
                Theme::normal_value()
            },
        ),
    ])));

    let is_sel = app.autotune_menu == AutotuneMenuState::Run;
    items.push(ListItem::new(Line::from(vec![Span::styled(
        format!(" {}", rust_i18n::t!("menu_autotune_run")),
        if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        },
    )])));

    let is_sel = app.autotune_menu == AutotuneMenuState::Back;
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    let selected_index = if app.autotune_menu_index < items.len() {
        app.autotune_menu_index
    } else {
        0
    };
    (items, rust_i18n::t!("menu_autotune_title").into_owned(), selected_index)
}

pub fn render_domain_files(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    for (idx, (label, _path)) in app.domain_files.iter().enumerate() {
        let sel = idx == app.domain_files_index;
        if sel {
            selected_index = idx;
        }
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(" {}: ", label),
            if sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            },
        )])));
    }

    let back_idx = app.domain_files.len();
    let sel_back = app.domain_files_index >= back_idx;
    if sel_back {
        selected_index = back_idx;
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_autotune_edit_domains").into_owned(),
        selected_index,
    )
}

pub fn render_protocols(app: &AppState, proto_menu: AutotuneProtocolsState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    let checks = [
        (
            AutotuneProtocolsState::Http,
            rust_i18n::t!("menu_autotune_http"),
            app.autotune_config.check_http,
        ),
        (
            AutotuneProtocolsState::Tls12,
            rust_i18n::t!("menu_autotune_tls12"),
            app.autotune_config.check_tls12,
        ),
        (
            AutotuneProtocolsState::Tls13,
            rust_i18n::t!("menu_autotune_tls13"),
            app.autotune_config.check_tls13,
        ),
        (
            AutotuneProtocolsState::Quic,
            rust_i18n::t!("menu_autotune_quic"),
            app.autotune_config.check_quic,
        ),
    ];

    for (idx, (state, label, enabled)) in checks.iter().enumerate() {
        let sel = *state == proto_menu;
        if sel {
            selected_index = idx;
        }
        let toggle = if *enabled {
            rust_i18n::t!("val_on")
        } else {
            rust_i18n::t!("val_off")
        };
        let toggle_style = if *enabled {
            Theme::active_value()
        } else {
            Theme::inactive_value()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {}: ", label),
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                format!("[ {} ]", toggle),
                if sel { Theme::selected_value() } else { toggle_style },
            ),
        ])));
    }

    let sel_back = AutotuneProtocolsState::Back == proto_menu;
    if sel_back {
        selected_index = 5;
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_autotune_proto").into_owned(),
        selected_index,
    )
}

pub fn render_blockchecks(
    app: &AppState,
    bc_menu: AutotuneBlockChecksState,
) -> (Vec<ListItem<'static>>, String, usize) {
    use crate::autotune::BlockCheckType;
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    let all_types = BlockCheckType::all();
    for (idx, ty) in all_types.iter().enumerate() {
        let state = match *ty {
            BlockCheckType::DnsSpoof => AutotuneBlockChecksState::DnsSpoof,
            BlockCheckType::TcpRst => AutotuneBlockChecksState::TcpRst,
            BlockCheckType::SniBlock => AutotuneBlockChecksState::SniBlock,
            BlockCheckType::SiberianBlock => AutotuneBlockChecksState::SiberianBlock,
            BlockCheckType::QuicBlock => AutotuneBlockChecksState::QuicBlock,
            BlockCheckType::CidrWhitelist => AutotuneBlockChecksState::CidrWhitelist,
        };
        let sel = state == bc_menu;
        if sel {
            selected_index = idx;
        }
        let enabled = app.autotune_config.block_checks.get(idx);
        let toggle = if enabled {
            rust_i18n::t!("val_on")
        } else {
            rust_i18n::t!("val_off")
        };
        let toggle_style = if enabled {
            Theme::active_value()
        } else {
            Theme::inactive_value()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {}: ", ty.name()),
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                format!("[ {} ]", toggle),
                if sel { Theme::selected_value() } else { toggle_style },
            ),
        ])));
    }

    let sel_back = AutotuneBlockChecksState::Back == bc_menu;
    if sel_back {
        selected_index = all_types.len();
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_autotune_bc").into_owned(),
        selected_index,
    )
}

pub fn render_presets(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    for (idx, preset) in PRESETS.iter().enumerate() {
        let sel = idx == app.autotune_preset_index;
        if sel {
            selected_index = idx;
        }
        let is_selected = app.autotune_config.preset_indices.contains(&idx);
        let toggle = if is_selected {
            rust_i18n::t!("val_on")
        } else {
            rust_i18n::t!("val_off")
        };
        let toggle_style = if is_selected {
            Theme::active_value()
        } else {
            Theme::inactive_value()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {}: ", preset.name),
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                format!("[ {} ]", toggle),
                if sel { Theme::selected_value() } else { toggle_style },
            ),
        ])));
    }

    let sel_back = app.autotune_preset_index >= PRESETS.len();
    if sel_back {
        selected_index = PRESETS.len();
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_autotune_presets").into_owned(),
        selected_index,
    )
}

pub fn render_strategies(app: &AppState, selected: usize) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = selected.min(app.strategies.len());

    for (idx, name) in app.strategies.iter().enumerate() {
        let sel = idx == selected;
        let checked = app.autotune_config.strategy_indices.contains(&idx);
        let toggle = if checked {
            rust_i18n::t!("val_on")
        } else {
            rust_i18n::t!("val_off")
        };
        let toggle_style = if checked {
            Theme::active_value()
        } else {
            Theme::inactive_value()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {}: ", name),
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                format!("[ {} ]", toggle),
                if sel { Theme::selected_value() } else { toggle_style },
            ),
        ])));
    }

    let sel_back = selected >= app.strategies.len();
    if sel_back {
        selected_index = app.strategies.len();
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (
        items,
        rust_i18n::t!("tui_title_autotune_strat").into_owned(),
        selected_index,
    )
}

pub fn render_results(_app: &AppState, scroll: usize) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();

    if let Some(cached) = crate::autotune::load_results_file() {
        for line in cached.lines() {
            items.push(ListItem::new(Line::from(Span::raw(format!(" {}", line)))));
        }
    } else {
        items.push(ListItem::new(Line::from(Span::raw(format!(
            " {}",
            rust_i18n::t!("autotune_no_results")
        )))));
    }

    let back_idx = items.len();
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if scroll == back_idx {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    let selected = scroll.min(back_idx);
    (
        items,
        rust_i18n::t!("tui_title_autotune_results").into_owned(),
        selected,
    )
}

pub fn render_header() -> (Vec<ListItem<'static>>, String, usize) {
    let items: Vec<ListItem<'static>> = vec![ListItem::new(Line::from(vec![Span::styled(
        rust_i18n::t!("autotune_running"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]))];
    (items, rust_i18n::t!("menu_autotune_title").into_owned(), 0)
}

fn menu_line(label: String, value: Option<String>, selected: bool) -> ListItem<'static> {
    let item_style = if selected {
        Theme::selected_item()
    } else {
        Theme::normal_item()
    };
    match value {
        Some(value) => ListItem::new(Line::from(vec![
            Span::styled(label, item_style),
            Span::styled(
                value,
                if selected {
                    Theme::selected_value()
                } else {
                    Theme::normal_value()
                },
            ),
        ])),
        None => ListItem::new(Line::from(vec![Span::styled(label, item_style)])),
    }
}

pub fn render_config_zapret2(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let states = app.autotune_menu_states();
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;
    let (preset_count, target_count) = app.z2_planned_counts();

    for (idx, state) in states.iter().enumerate() {
        let selected = *state == app.autotune_menu;
        if selected {
            selected_index = idx;
        }
        let item = match state {
            AutotuneMenuState::Z2Bundle => menu_line(
                format!(" {}: ", rust_i18n::t!("menu_autotune_z2_bundle")),
                Some(format!("‹ {} ›", app.z2_bundle.label())),
                selected,
            ),
            AutotuneMenuState::Z2Targets => {
                let lists = app.resolved_z2_lists();
                let label = if lists.is_empty() {
                    format!(
                        "{} ({})",
                        rust_i18n::t!("autotune_z2_default_targets"),
                        target_count
                    )
                } else {
                    format!("{} · {} {}", app.z2_bundle_summary(), target_count, rust_i18n::t!("autotune_z2_domains_suffix"))
                };
                menu_line(
                    format!(" {}: ", rust_i18n::t!("menu_autotune_z2_targets")),
                    Some(format!("‹ {} ›", label)),
                    selected,
                )
            }
            AutotuneMenuState::Z2Presets => {
                let label = if app.z2_selected_presets.is_empty() {
                    format!("{} ({})", rust_i18n::t!("autotune_z2_all_presets"), preset_count)
                } else {
                    format!(
                        "{} / {}",
                        app.z2_selected_presets.len(),
                        app.z2_presets.len().max(preset_count)
                    )
                };
                menu_line(
                    format!(" {}: ", rust_i18n::t!("menu_autotune_z2_presets")),
                    Some(format!("‹ {} ›", label)),
                    selected,
                )
            }
            AutotuneMenuState::Results => {
                let label = if app.has_autotune_results_file {
                    format!("‹ {} ›", rust_i18n::t!("menu_autotune_view"))
                } else {
                    rust_i18n::t!("menu_autotune_no_results").into_owned()
                };
                menu_line(
                    format!(" {}: ", rust_i18n::t!("menu_autotune_results")),
                    Some(label),
                    selected,
                )
            }
            AutotuneMenuState::Run => menu_line(
                format!(
                    " {}  ({} x {})",
                    rust_i18n::t!("menu_autotune_run"),
                    preset_count,
                    target_count
                ),
                None,
                selected,
            ),
            _ => menu_line(format!(" {}", rust_i18n::t!("menu_autotune_back")), None, selected),
        };
        items.push(item);
    }

    let mut preview: Vec<String> = app.z2_targets.iter().take(6).cloned().collect();
    if app.z2_targets.len() > preview.len() {
        preview.push(format!("+{}", app.z2_targets.len() - preview.len()));
    }
    if !preview.is_empty() {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("   {}: ", rust_i18n::t!("autotune_z2_targets_used")),
                Theme::dim_item(),
            ),
            Span::styled(preview.join(", "), Theme::hint()),
        ])));
    }

    (
        items,
        rust_i18n::t!("menu_autotune_z2_title").into_owned(),
        selected_index,
    )
}

fn render_toggle_list(
    entries: &[String],
    selected: &[usize],
    cursor: usize,
    title: String,
) -> (Vec<ListItem<'static>>, String, usize) {
    let mut items: Vec<ListItem<'static>> = Vec::new();
    let mut selected_index = 0;

    for (idx, name) in entries.iter().enumerate() {
        let sel = idx == cursor;
        if sel {
            selected_index = idx;
        }
        let is_on = selected.contains(&idx);
        let toggle_style = if is_on {
            Theme::active_value()
        } else {
            Theme::dim_item()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if sel { "▸ " } else { "  " },
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
            Span::styled(
                toggle_marker(is_on),
                if sel { Theme::selected_value() } else { toggle_style },
            ),
            Span::styled(
                format!(" {}", name),
                if sel {
                    Theme::selected_item()
                } else {
                    Theme::normal_item()
                },
            ),
        ])));
    }

    let sel_back = cursor >= entries.len();
    if sel_back {
        selected_index = entries.len();
    }
    items.push(
        ListItem::new(format!(" {}", rust_i18n::t!("menu_autotune_back"))).style(if sel_back {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }),
    );

    (items, title, selected_index)
}

pub fn render_z2_presets(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    render_toggle_list(
        &app.z2_presets,
        &app.z2_selected_presets,
        app.z2_preset_index,
        rust_i18n::t!("tui_title_autotune_z2_presets").into_owned(),
    )
}

pub fn render_z2_targets(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    render_toggle_list(
        &app.z2_lists,
        &app.z2_selected_lists,
        app.z2_list_index,
        rust_i18n::t!("tui_title_autotune_z2_targets").into_owned(),
    )
}
