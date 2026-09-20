use crate::tui::state::{AppState, DownloadDepsMenuState};
use crate::tui::theme::Theme;
use ratatui::widgets::ListItem;

pub fn render(app: &AppState) -> (Vec<ListItem<'static>>, String, usize) {
    let mut selected_index = 0;
    let mut items = vec![];
    let mut index = 0;

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::ZapretDownloader;
        if is_sel {
            selected_index = index;
        }
        items.push(
            ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_zapret"))).style(if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            }),
        );
        index += 1;
    }

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::Zapret2Downloader;
        if is_sel {
            selected_index = index;
        }
        items.push(ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_zapret2"))).style(if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }));
        index += 1;
    }

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::StrategiesDownloader;
        if is_sel {
            selected_index = index;
        }
        items.push(
            ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_strat"))).style(if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            }),
        );
        index += 1;
    }

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::Zapret2Strategies;
        if is_sel {
            selected_index = index;
        }
        items.push(
            ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_zapret2_strat"))).style(if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            }),
        );
        index += 1;
    }

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::DownloadDefaults;
        if is_sel {
            selected_index = index;
        }
        items.push(
            ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_defaults"))).style(if is_sel {
                Theme::selected_item()
            } else {
                Theme::normal_item()
            }),
        );
        index += 1;
    }

    {
        let is_sel = app.download_deps_menu == DownloadDepsMenuState::Back;
        if is_sel {
            selected_index = index;
        }
        items.push(ListItem::new(format!(" {}", rust_i18n::t!("menu_dl_back"))).style(if is_sel {
            Theme::selected_item()
        } else {
            Theme::normal_item()
        }));
    }

    (items, rust_i18n::t!("menu_dl_title").into_owned(), selected_index)
}
