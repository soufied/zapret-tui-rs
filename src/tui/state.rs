#[cfg(target_os = "linux")]
use crate::firewalls::LinuxBackend;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ActiveScreen {
    Main,
    #[cfg(target_os = "windows")]
    DefenderSubmenu,
    StrategySubmenu,
    DownloadDepsSubmenu,
    DownloadZapretSubmenu,
    DownloadStrategiesSubmenu,
    GamefilterSubmenu,
    FakesSubmenu,
    FakesSelectSubmenu,
    ZapretTagSelect,
    StrategyTagSelect,
    ServiceSubmenu,
    ServiceConflictSubmenu,
    ListsEditorSubmenu,
    StrategyEditorSubmenu,
    AutotuneSubmenu,
    AutotuneEditDomainsSubmenu,
    AutotuneProtocolsSubmenu,
    AutotuneBlockChecksSubmenu,
    AutotunePresetSelectionSubmenu,
    AutotuneStrategiesSubmenu,
    AutotuneZ2PresetsSubmenu,
    AutotuneZ2TargetsSubmenu,
    AutotuneResultsSubmenu,
    AutotuneServiceMatrixSubmenu,
    SettingsSubmenu,
    SettingsEditorSubmenu,
    LogViewer,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MainMenuState {
    #[cfg(target_os = "windows")]
    DefenderSettings,
    DownloadDeps,
    Engine,
    Interface,
    Strategy,
    StrategyEditor,
    GamefilterSettings,
    #[cfg(target_os = "linux")]
    BackendSettings,
    IpsetMode,
    TtlAutopick,
    ListsEditor,
    Autotune,
    FakesSettings,
    Settings,
    ServiceSettings,
    Run,
    Quit,
}

impl MainMenuState {
    pub fn next(self) -> Self {
        match self {
            #[cfg(target_os = "windows")]
            Self::DefenderSettings => Self::DownloadDeps,
            Self::DownloadDeps => Self::Engine,
            Self::Engine => Self::Interface,
            Self::Interface => Self::Strategy,
            Self::Strategy => Self::GamefilterSettings,
            #[cfg(target_os = "linux")]
            Self::GamefilterSettings => Self::BackendSettings,
            #[cfg(target_os = "linux")]
            Self::BackendSettings => Self::IpsetMode,
            #[cfg(not(target_os = "linux"))]
            Self::GamefilterSettings => Self::IpsetMode,
            Self::IpsetMode => Self::TtlAutopick,
            Self::TtlAutopick => Self::ListsEditor,
            Self::ListsEditor => Self::StrategyEditor,
            Self::StrategyEditor => Self::Autotune,
            Self::Autotune => Self::FakesSettings,
            Self::FakesSettings => Self::Settings,
            Self::Settings => Self::ServiceSettings,
            Self::ServiceSettings => Self::Run,
            Self::Run => Self::Quit,
            #[cfg(target_os = "windows")]
            Self::Quit => Self::DefenderSettings,
            #[cfg(not(target_os = "windows"))]
            Self::Quit => Self::DownloadDeps,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            #[cfg(target_os = "windows")]
            Self::DefenderSettings => Self::Quit,
            #[cfg(target_os = "windows")]
            Self::DownloadDeps => Self::DefenderSettings,
            #[cfg(not(target_os = "windows"))]
            Self::DownloadDeps => Self::Quit,
            Self::Engine => Self::DownloadDeps,
            Self::Interface => Self::Engine,
            Self::Strategy => Self::Interface,
            Self::GamefilterSettings => Self::Strategy,
            #[cfg(target_os = "linux")]
            Self::BackendSettings => Self::GamefilterSettings,
            #[cfg(target_os = "linux")]
            Self::IpsetMode => Self::BackendSettings,
            #[cfg(not(target_os = "linux"))]
            Self::IpsetMode => Self::GamefilterSettings,
            Self::TtlAutopick => Self::IpsetMode,
            Self::ListsEditor => Self::TtlAutopick,
            Self::Autotune => Self::StrategyEditor,
            Self::StrategyEditor => Self::ListsEditor,
            Self::FakesSettings => Self::Autotune,
            Self::Settings => Self::FakesSettings,
            Self::ServiceSettings => Self::Settings,
            Self::Run => Self::ServiceSettings,
            Self::Quit => Self::Run,
        }
    }

    pub fn is_visible_for(self, engine: &crate::config::ZapretEngine) -> bool {
        if self == Self::GamefilterSettings && !engine.supports_game_filter() {
            return false;
        }
        if self == Self::StrategyEditor && !engine.uses_presets() {
            return false;
        }
        true
    }

    pub fn next_visible(self, engine: &crate::config::ZapretEngine) -> Self {
        let mut candidate = self.next();
        while !candidate.is_visible_for(engine) {
            candidate = candidate.next();
        }
        candidate
    }

    pub fn prev_visible(self, engine: &crate::config::ZapretEngine) -> Self {
        let mut candidate = self.prev();
        while !candidate.is_visible_for(engine) {
            candidate = candidate.prev();
        }
        candidate
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PendingServiceAction {
    Install,
    Start,
    Restart,
}

#[derive(PartialEq, Clone, Copy)]
pub enum GamefilterMenuState {
    Tcp,
    Udp,
    Back,
}

impl GamefilterMenuState {
    pub fn next(self) -> Self {
        match self {
            Self::Tcp => Self::Udp,
            Self::Udp => Self::Back,
            Self::Back => Self::Tcp,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Tcp => Self::Back,
            Self::Udp => Self::Tcp,
            Self::Back => Self::Udp,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FakesMenuState {
    DiscordUdp,
    GameUdp,
    Back,
}

impl FakesMenuState {
    pub fn next(self) -> Self {
        match self {
            Self::DiscordUdp => Self::GameUdp,
            Self::GameUdp => Self::Back,
            Self::Back => Self::DiscordUdp,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::DiscordUdp => Self::Back,
            Self::GameUdp => Self::DiscordUdp,
            Self::Back => Self::GameUdp,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum FakesSelectTarget {
    DiscordUdp,
    GameUdp,
}

#[cfg(target_os = "windows")]
#[derive(PartialEq, Clone, Copy)]
pub enum DefenderMenuState {
    Add,
    Remove,
    Back,
}

#[cfg(target_os = "windows")]
impl DefenderMenuState {
    pub fn next(self) -> Self {
        match self {
            Self::Add => Self::Remove,
            Self::Remove => Self::Back,
            Self::Back => Self::Add,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Add => Self::Back,
            Self::Remove => Self::Add,
            Self::Back => Self::Remove,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SettingsMenuState {
    Editor,
    BackupLists,
    ViewLogs,
    Back,
}

impl SettingsMenuState {
    pub fn all() -> &'static [Self] {
        &[Self::Editor, Self::BackupLists, Self::ViewLogs, Self::Back]
    }

    pub fn index(self) -> usize {
        match self {
            Self::Editor => 0,
            Self::BackupLists => 1,
            Self::ViewLogs => 2,
            Self::Back => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::BackupLists,
            2 => Self::ViewLogs,
            3 => Self::Back,
            _ => Self::Editor,
        }
    }

    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % Self::all().len())
    }

    pub fn prev(self) -> Self {
        let len = Self::all().len();
        Self::from_index((self.index() + len - 1) % len)
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditorKind {
    Auto,
    Binary,
    Custom,
}

#[derive(Clone, Debug)]
pub struct EditorEntry {
    pub kind: EditorKind,
    pub command: String,
    pub label: String,
    pub installed: bool,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AutotuneMenuState {
    PresetSelection,
    Z2Bundle,
    Z2Presets,
    Z2Targets,
    NumRequests,
    Strategies,
    Protocols,
    BlockChecks,
    EditDomains,
    ServiceMatrix,
    Results,
    Run,
    Back,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AutotuneProtocolsState {
    Http,
    Tls12,
    Tls13,
    Quic,
    Back,
}

impl AutotuneProtocolsState {
    pub fn next(self) -> Self {
        match self {
            Self::Http => Self::Tls12,
            Self::Tls12 => Self::Tls13,
            Self::Tls13 => Self::Quic,
            Self::Quic => Self::Back,
            Self::Back => Self::Http,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Http => Self::Back,
            Self::Tls12 => Self::Http,
            Self::Tls13 => Self::Tls12,
            Self::Quic => Self::Tls13,
            Self::Back => Self::Quic,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum AutotuneBlockChecksState {
    DnsSpoof,
    TcpRst,
    SniBlock,
    SiberianBlock,
    QuicBlock,
    CidrWhitelist,
    Back,
}

impl AutotuneBlockChecksState {
    pub fn next(self) -> Self {
        match self {
            Self::DnsSpoof => Self::TcpRst,
            Self::TcpRst => Self::SniBlock,
            Self::SniBlock => Self::SiberianBlock,
            Self::SiberianBlock => Self::QuicBlock,
            Self::QuicBlock => Self::CidrWhitelist,
            Self::CidrWhitelist => Self::Back,
            Self::Back => Self::DnsSpoof,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::DnsSpoof => Self::Back,
            Self::TcpRst => Self::DnsSpoof,
            Self::SniBlock => Self::TcpRst,
            Self::SiberianBlock => Self::SniBlock,
            Self::QuicBlock => Self::SiberianBlock,
            Self::CidrWhitelist => Self::QuicBlock,
            Self::Back => Self::CidrWhitelist,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::DnsSpoof => 0,
            Self::TcpRst => 1,
            Self::SniBlock => 2,
            Self::SiberianBlock => 3,
            Self::QuicBlock => 4,
            Self::CidrWhitelist => 5,
            Self::Back => 6,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum DownloadDepsMenuState {
    ZapretDownloader,
    Zapret2Downloader,
    StrategiesDownloader,
    Zapret2Strategies,
    DownloadDefaults,
    Back,
}

impl DownloadDepsMenuState {
    pub fn next(self) -> Self {
        match self {
            Self::ZapretDownloader => Self::Zapret2Downloader,
            Self::Zapret2Downloader => Self::StrategiesDownloader,
            Self::StrategiesDownloader => Self::Zapret2Strategies,
            Self::Zapret2Strategies => Self::DownloadDefaults,
            Self::DownloadDefaults => Self::Back,
            Self::Back => Self::ZapretDownloader,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::ZapretDownloader => Self::Back,
            Self::Zapret2Downloader => Self::ZapretDownloader,
            Self::StrategiesDownloader => Self::Zapret2Downloader,
            Self::Zapret2Strategies => Self::StrategiesDownloader,
            Self::DownloadDefaults => Self::Zapret2Strategies,
            Self::Back => Self::DownloadDefaults,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum DownloadSubmenuState {
    Version,
    SelectTag,
    Start,
    Back,
}

impl DownloadSubmenuState {
    pub fn next(self) -> Self {
        match self {
            Self::Version => Self::SelectTag,
            Self::SelectTag => Self::Start,
            Self::Start => Self::Back,
            Self::Back => Self::Version,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Version => Self::Back,
            Self::SelectTag => Self::Version,
            Self::Start => Self::SelectTag,
            Self::Back => Self::Start,
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum VersionTarget {
    Recommended,
    Latest,
    Tag(String),
}

impl VersionTarget {
    pub fn cycle(&self, forward: bool) -> Self {
        if forward {
            match self {
                Self::Recommended => Self::Latest,
                Self::Latest | Self::Tag(_) => Self::Recommended,
            }
        } else {
            match self {
                Self::Recommended | Self::Tag(_) => Self::Latest,
                Self::Latest => Self::Recommended,
            }
        }
    }
}

pub struct AppState {
    pub engine: crate::config::ZapretEngine,
    pub interfaces: Vec<String>,
    pub selected_interface: usize,

    #[cfg(target_os = "linux")]
    pub selected_backend: LinuxBackend,

    pub available_ipset_modes: Vec<crate::ipset::IpsetMode>,
    pub selected_ipset_mode: usize,

    pub strategies: Vec<String>,
    pub selected_strategy: usize,
    pub strategy_menu_index: usize,

    pub tcp_gamefilter: bool,
    pub udp_gamefilter: bool,

    pub active_screen: ActiveScreen,
    pub main_menu: MainMenuState,

    #[cfg(target_os = "windows")]
    pub defender_menu: DefenderMenuState,
    #[cfg(target_os = "windows")]
    pub defender_status_cache: Option<bool>,

    pub download_deps_menu: DownloadDepsMenuState,
    pub download_zapret_menu: DownloadSubmenuState,
    pub download_strategies_menu: DownloadSubmenuState,
    pub gamefilter_menu: GamefilterMenuState,
    pub fakes_state: crate::fakes::FakesState,
    pub fakes_menu: FakesMenuState,
    pub fakes_select_index: usize,
    pub fakes_select_for: FakesSelectTarget,
    pub nfqws_target: VersionTarget,
    pub strat_target: VersionTarget,

    pub available_nfqws_tags: Vec<String>,
    pub available_strat_tags: Vec<String>,
    pub nfqws_tag_index: usize,
    pub strat_tag_index: usize,

    pub should_run: bool,
    pub should_quit: bool,
    pub should_download_zapret: bool,
    pub should_download_zapret2: bool,
    pub should_download_zapret2_strategies: bool,
    pub should_download_strategies: bool,
    pub should_download_defaults: bool,
    pub status_message: Option<String>,

    pub nfqws_installed: bool,
    pub strategies_installed: bool,

    pub service_installed: bool,
    pub service_active: bool,
    pub service_menu_index: usize,
    pub service_conflict: Option<crate::inits::ServiceConflict>,
    pub service_conflict_index: usize,
    pub pending_service_action: Option<PendingServiceAction>,

    pub lists_files: Vec<String>,
    pub lists_menu_index: usize,
    pub should_open_editor: Option<String>,
    pub domain_files: Vec<(String, String)>,
    pub domain_files_index: usize,

    pub should_sync_community_strategies: bool,
    pub strategy_editor_files: Vec<crate::tui::menus::strategy_editor_menu::StrategyFileEntry>,
    pub strategy_editor_index: usize,
    pub strategy_editor_inspection: Option<crate::tui::menus::strategy_editor_menu::StrategyInspection>,
    pub strategy_editor_new_name_editing: bool,
    pub strategy_editor_new_name_buf: String,
    pub strategy_editor_delete_confirm: bool,

    pub autotune_config: crate::autotune::AutotuneConfig,
    pub autotune_results: Option<crate::autotune::AutotuneResults>,
    pub has_autotune_results_file: bool,
    pub autotune_menu: AutotuneMenuState,
    pub autotune_menu_index: usize,
    pub autotune_protocols_menu: AutotuneProtocolsState,
    pub autotune_block_checks_menu: AutotuneBlockChecksState,
    pub autotune_preset_index: usize,
    pub z2_presets: Vec<String>,
    pub z2_selected_presets: Vec<usize>,
    pub z2_preset_index: usize,
    pub z2_lists: Vec<String>,
    pub z2_selected_lists: Vec<usize>,
    pub z2_list_index: usize,
    pub z2_targets: Vec<String>,
    pub z2_bundle: crate::autotune::TargetBundle,
    pub autotune_strat_index: usize,
    pub autotune_results_index: usize,
    pub service_matrix_rows: Vec<crate::autotune::MatrixRow>,
    pub service_matrix_reports: Vec<crate::autotune::ServiceReport>,
    pub service_matrix_index: usize,
    pub service_matrix_elapsed_ms: u128,
    pub should_run_service_matrix: bool,
    pub should_run_autotune: bool,
    pub autotune_running: bool,
    pub autotune_request_editing: bool,
    pub autotune_request_buf: String,
    pub should_run_ttl: bool,
    pub dpi_desync_ttl: Option<u8>,

    pub settings_menu: SettingsMenuState,
    pub editor_entries: Vec<EditorEntry>,
    pub editor_index: usize,
    pub editor_custom_editing: bool,
    pub editor_custom_buf: String,
    pub backup_lists: bool,
    pub log_lines: Vec<String>,
    pub log_scroll: usize,
    pub dpi_running: bool,
}

impl AppState {
    pub fn show_error(&mut self, msg: String) {
        self.status_message = Some(format!("{}{}", rust_i18n::t!("msg_err"), msg));
    }

    pub fn new(interfaces: Vec<String>, strategies: Vec<String>) -> Self {
        let _ = crate::config::ensure_default_config();
        let _ = crate::autotune::ensure_domain_files();

        let domain_files: Vec<(String, String)> = {
            let mut files: Vec<(String, String)> = Vec::new();
            for (idx, preset) in crate::autotune::PRESETS.iter().enumerate() {
                let is_custom = idx == crate::autotune::PRESETS.len() - 1;
                let label = if is_custom {
                    rust_i18n::t!("menu_domain_custom").into_owned()
                } else {
                    preset.name.to_string()
                };
                files.push((
                    label,
                    crate::autotune::preset_domains_file_path(idx)
                        .to_string_lossy()
                        .into_owned(),
                ));
            }
            files.push((
                rust_i18n::t!("menu_domain_ttl").into_owned(),
                crate::ttl::ttl_domains_file_path().to_string_lossy().into_owned(),
            ));
            files
        };

        let saved_cfg = crate::config::load_config(&crate::config::config_path().to_string_lossy()).ok();
        let engine = saved_cfg
            .as_ref()
            .map_or(crate::config::ZapretEngine::Zapret1, |cfg| cfg.engine.clone());
        std::env::set_var("REPO_DIR", engine.workspace_dir());
        crate::runner::set_engine(engine.clone());
        let strategies = if strategies.is_empty() || engine.uses_presets() {
            crate::strategy::get_strategies_for(&engine)
        } else {
            strategies
        };

        let selected_interface = saved_cfg.as_ref().map_or(0, |cfg| {
            interfaces.iter().position(|i| i == &cfg.interface).unwrap_or(0)
        });
        let selected_strategy = saved_cfg
            .as_ref()
            .map_or(0, |cfg| strategies.iter().position(|s| s == &cfg.strategy).unwrap_or(0));
        let tcp_gamefilter = saved_cfg.as_ref().is_some_and(|cfg| cfg.gamefilter_tcp);
        let udp_gamefilter = saved_cfg.as_ref().is_some_and(|cfg| cfg.gamefilter_udp);

        #[cfg(target_os = "linux")]
        let selected_backend = saved_cfg.as_ref().map_or_else(
            || LinuxBackend::from_config("nftables"),
            |cfg| LinuxBackend::from_config(&cfg.backend),
        );

        let available_ipset_modes = crate::ipset::get_available_modes_for(&engine);
        let current_ipset_mode = crate::ipset::determine_current_mode();
        let selected_ipset_mode = available_ipset_modes
            .iter()
            .position(|m| m == &current_ipset_mode)
            .unwrap_or(0);

        let engine_for_checks = engine.clone();
        let mut app = Self {
            engine,
            interfaces,
            selected_interface,
            #[cfg(target_os = "linux")]
            selected_backend,
            available_ipset_modes,
            selected_ipset_mode,
            strategies,
            selected_strategy,
            strategy_menu_index: selected_strategy,
            tcp_gamefilter,
            udp_gamefilter,
            active_screen: ActiveScreen::Main,

            #[cfg(target_os = "windows")]
            main_menu: MainMenuState::DefenderSettings,
            #[cfg(not(target_os = "windows"))]
            main_menu: MainMenuState::DownloadDeps,

            #[cfg(target_os = "windows")]
            defender_menu: DefenderMenuState::Add,
            #[cfg(target_os = "windows")]
            defender_status_cache: crate::defender::check_defender_exclusion().ok(),

            download_deps_menu: DownloadDepsMenuState::ZapretDownloader,
            download_zapret_menu: DownloadSubmenuState::Version,
            download_strategies_menu: DownloadSubmenuState::Version,
            gamefilter_menu: GamefilterMenuState::Tcp,
            fakes_state: crate::fakes::load_fakes_state(),
            fakes_menu: FakesMenuState::DiscordUdp,
            fakes_select_index: 0,
            fakes_select_for: FakesSelectTarget::DiscordUdp,
            nfqws_target: VersionTarget::Recommended,
            strat_target: VersionTarget::Recommended,

            available_nfqws_tags: Vec::new(),
            available_strat_tags: Vec::new(),
            nfqws_tag_index: 0,
            strat_tag_index: 0,

            should_run: false,
            should_quit: false,
            should_download_zapret: false,
            should_download_zapret2: false,
            should_download_zapret2_strategies: false,
            should_download_strategies: false,
            should_download_defaults: false,
            status_message: None,

            nfqws_installed: crate::download::check_nfqws_installed_for(&engine_for_checks),
            strategies_installed: crate::download::check_strategies_installed_for(&engine_for_checks),

            service_installed: false,
            service_active: false,
            service_menu_index: 0,
            service_conflict: None,
            service_conflict_index: 0,
            pending_service_action: None,

            lists_files: Vec::new(),
            lists_menu_index: 0,
            should_open_editor: None,
            domain_files,
            domain_files_index: 0,

            should_sync_community_strategies: false,
            strategy_editor_files: Vec::new(),
            strategy_editor_index: 0,
            strategy_editor_inspection: None,
            strategy_editor_new_name_editing: false,
            strategy_editor_new_name_buf: String::new(),
            strategy_editor_delete_confirm: false,

            autotune_config: crate::autotune::AutotuneConfig::default(),
            autotune_results: None,
            has_autotune_results_file: crate::autotune::load_results_file().is_some(),
            autotune_menu: AutotuneMenuState::PresetSelection,
            autotune_menu_index: 0,
            autotune_protocols_menu: AutotuneProtocolsState::Http,
            autotune_block_checks_menu: AutotuneBlockChecksState::DnsSpoof,
            autotune_preset_index: 0,
            z2_presets: Vec::new(),
            z2_selected_presets: Vec::new(),
            z2_preset_index: 0,
            z2_lists: Vec::new(),
            z2_selected_lists: Vec::new(),
            z2_list_index: 0,
            z2_targets: Vec::new(),
            z2_bundle: crate::autotune::TargetBundle::default(),
            autotune_strat_index: 0,
            autotune_results_index: 0,
            service_matrix_rows: Vec::new(),
            service_matrix_reports: Vec::new(),
            service_matrix_index: 0,
            service_matrix_elapsed_ms: 0,
            should_run_service_matrix: false,
            should_run_autotune: false,
            autotune_running: false,
            autotune_request_editing: false,
            autotune_request_buf: String::new(),
            should_run_ttl: false,
            dpi_desync_ttl: crate::config::load_ttl(),

            settings_menu: SettingsMenuState::Editor,
            editor_entries: Vec::new(),
            editor_index: 0,
            editor_custom_editing: false,
            editor_custom_buf: String::new(),
            backup_lists: crate::config::load_backup_lists(),
            log_lines: Vec::new(),
            log_scroll: 0,
            dpi_running: false,
        };
        app.refresh_editors();
        app.reload_z2_lists();
        app.refresh_service_status();
        app.refresh_runtime_status();
        app
    }

    pub fn refresh_runtime_status(&mut self) {
        self.dpi_running = crate::platform::is_nfqws_running();
    }

    pub fn status_fingerprint(&self) -> (bool, bool, bool, bool, bool, bool) {
        (
            self.service_installed,
            self.service_active,
            self.dpi_running,
            self.nfqws_installed,
            self.strategies_installed,
            self.service_conflict.is_some(),
        )
    }

    pub fn refresh_editors(&mut self) {
        let configured = crate::config::load_editor();
        let mut entries: Vec<EditorEntry> = Vec::new();

        entries.push(EditorEntry {
            kind: EditorKind::Auto,
            command: String::new(),
            label: rust_i18n::t!("settings_editor_auto_label").into_owned(),
            installed: crate::utils::resolve_editor().is_some(),
        });

        for candidate in crate::utils::EDITOR_CANDIDATES {
            entries.push(EditorEntry {
                kind: EditorKind::Binary,
                command: candidate.command.to_string(),
                label: candidate.label.to_string(),
                installed: crate::utils::editor_is_installed(candidate.command),
            });
        }

        let known = entries
            .iter()
            .any(|e| e.kind == EditorKind::Binary && e.command == configured.trim());
        let custom_value = if configured.trim().is_empty() || known {
            String::new()
        } else {
            configured.trim().to_string()
        };
        let custom_installed = custom_value
            .split_whitespace()
            .next()
            .map(crate::utils::editor_is_installed)
            .unwrap_or(false);

        entries.push(EditorEntry {
            kind: EditorKind::Custom,
            command: custom_value,
            label: rust_i18n::t!("settings_editor_custom_label").into_owned(),
            installed: custom_installed,
        });

        let selected = self.selected_editor_position(&entries, &configured);
        self.editor_entries = entries;
        self.editor_index = selected;
    }

    fn selected_editor_position(&self, entries: &[EditorEntry], configured: &str) -> usize {
        let configured = configured.trim();
        if configured.is_empty() {
            return 0;
        }
        entries
            .iter()
            .position(|e| e.kind == EditorKind::Binary && e.command == configured)
            .or_else(|| entries.iter().position(|e| e.kind == EditorKind::Custom))
            .unwrap_or(0)
    }

    pub fn apply_editor_selection(&mut self, index: usize) {
        let Some(entry) = self.editor_entries.get(index).cloned() else {
            return;
        };

        match entry.kind {
            EditorKind::Auto => {
                let _ = crate::config::save_editor("");
                self.refresh_editors();
                self.status_message = Some(format!(
                    "{} {}",
                    rust_i18n::t!("settings_editor_saved"),
                    crate::utils::active_editor_label()
                ));
            }
            EditorKind::Binary => {
                if !entry.installed {
                    self.show_error(format!("{} {}", rust_i18n::t!("settings_editor_missing"), entry.command));
                    return;
                }
                let _ = crate::config::save_editor(&entry.command);
                self.refresh_editors();
                self.status_message = Some(format!(
                    "{} {}",
                    rust_i18n::t!("settings_editor_saved"),
                    entry.command
                ));
            }
            EditorKind::Custom => {
                self.editor_custom_buf = entry.command.clone();
                self.editor_custom_editing = true;
                self.status_message = Some(rust_i18n::t!("settings_editor_custom_hint").into_owned());
            }
        }
    }

    pub fn commit_custom_editor(&mut self) {
        let value = self.editor_custom_buf.trim().to_string();
        self.editor_custom_editing = false;
        self.editor_custom_buf.clear();

        if value.is_empty() {
            let _ = crate::config::save_editor("");
            self.refresh_editors();
            self.status_message = Some(rust_i18n::t!("settings_editor_cleared").into_owned());
            return;
        }

        let program = value.split_whitespace().next().unwrap_or("").to_string();
        let _ = crate::config::save_editor(&value);
        self.refresh_editors();

        if crate::utils::editor_is_installed(&program) {
            self.status_message = Some(format!("{} {}", rust_i18n::t!("settings_editor_saved"), value));
        } else {
            self.show_error(format!("{} {}", rust_i18n::t!("settings_editor_missing"), program));
        }
    }

    pub fn toggle_backup_lists(&mut self) {
        self.backup_lists = !self.backup_lists;
        let _ = crate::config::save_backup_lists(self.backup_lists);
    }

    pub fn load_logs(&mut self) {
        let path = crate::config::get_cache_dir().join("logs").join("zapret.log");
        self.log_lines = crate::utils::read_log_tail(&path, 800);
        if self.log_lines.is_empty() {
            self.log_lines
                .push(rust_i18n::t!("settings_logs_empty").into_owned());
        }
        self.log_scroll = self.log_lines.len().saturating_sub(1);
    }

    pub fn refresh_dep_status(&mut self) {
        self.nfqws_installed = crate::download::check_nfqws_installed_for(&self.engine);
        self.strategies_installed = crate::download::check_strategies_installed_for(&self.engine);
    }

    pub fn switch_engine(&mut self) {
        self.engine = match self.engine {
            crate::config::ZapretEngine::Zapret1 => crate::config::ZapretEngine::Zapret2,
            crate::config::ZapretEngine::Zapret2 => crate::config::ZapretEngine::Zapret1,
        };
        std::env::set_var("REPO_DIR", self.engine.workspace_dir());
        crate::runner::set_engine(self.engine.clone());
        self.strategies = crate::strategy::get_strategies_for(&self.engine);
        self.selected_strategy = 0;
        self.strategy_menu_index = 0;
        self.autotune_config.strategy_indices.clear();
        self.autotune_strat_index = 0;
        self.autotune_results = None;
        self.set_autotune_menu_index(0);
        self.reload_z2_lists();
        self.save_current_config();
        self.refresh_dep_status();
        self.refresh_service_status();
        self.refresh_ipset_status();
        if !self.main_menu.is_visible_for(&self.engine) {
            self.main_menu = self.main_menu.next_visible(&self.engine);
        }
        self.status_message = if self.engine.supports_game_filter() {
            None
        } else {
            Some(rust_i18n::t!("msg_gf_preset_managed").into_owned())
        };
    }

    pub fn reload_strategies(&mut self) {
        let current = self.strategies.get(self.selected_strategy).cloned().unwrap_or_default();
        self.strategies = crate::strategy::get_strategies_for(&self.engine);
        self.selected_strategy = self.strategies.iter().position(|s| *s == current).unwrap_or(0);
        self.strategy_menu_index = self.selected_strategy;
    }

    pub fn reload_z2_presets(&mut self) {
        let previously: Vec<String> = self
            .z2_selected_presets
            .iter()
            .filter_map(|&i| self.z2_presets.get(i).cloned())
            .collect();

        self.z2_presets = crate::strategy::zapret2_presets();
        self.z2_selected_presets = self
            .z2_presets
            .iter()
            .enumerate()
            .filter(|(_, name)| previously.contains(name))
            .map(|(i, _)| i)
            .collect();

        if self.z2_preset_index > self.z2_presets.len() {
            self.z2_preset_index = 0;
        }
    }

    pub fn refresh_strategy_editor_files(&mut self) {
        let previously = self
            .strategy_editor_files
            .get(self.strategy_editor_index)
            .map(|entry| entry.path.clone());

        self.strategy_editor_files = crate::tui::menus::strategy_editor_menu::list_files(&self.engine);

        self.strategy_editor_index = previously
            .and_then(|path| self.strategy_editor_files.iter().position(|entry| entry.path == path))
            .unwrap_or(0)
            .min(self.strategy_editor_files.len());
    }

    pub fn refresh_strategy_editor_inspection(&mut self) {
        self.strategy_editor_inspection = self
            .strategy_editor_files
            .get(self.strategy_editor_index)
            .and_then(|entry| {
                crate::tui::menus::strategy_editor_menu::inspect_file(&entry.path, &self.engine.workspace_dir()).ok()
            });
    }

    pub fn open_highlighted_strategy_file(&mut self) {
        if let Some(entry) = self.strategy_editor_files.get(self.strategy_editor_index) {
            self.should_open_editor = Some(entry.path.to_string_lossy().into_owned());
        }
    }

    pub fn activate_selected_strategy_file(&mut self) {
        let Some(entry) = self.strategy_editor_files.get(self.strategy_editor_index).cloned() else {
            return;
        };

        if entry.kind != crate::tui::menus::strategy_editor_menu::StrategyFileKind::Preset {
            self.status_message = Some(rust_i18n::t!("msg_strat_editor_not_preset").into_owned());
            return;
        }

        let Ok(inspection) =
            crate::tui::menus::strategy_editor_menu::inspect_file(&entry.path, &self.engine.workspace_dir())
        else {
            self.show_error(rust_i18n::t!("msg_strat_editor_inspect_failed").into_owned());
            return;
        };

        if !inspection.is_activatable() {
            self.status_message = Some(rust_i18n::t!("msg_strat_editor_missing_lists").into_owned());
            return;
        }

        self.reload_strategies();
        match self.strategies.iter().position(|s| *s == entry.name) {
            Some(index) => {
                self.selected_strategy = index;
                self.strategy_menu_index = index;
                self.save_current_config();
                self.status_message = Some(format!("{}{}", rust_i18n::t!("msg_strat_sel"), entry.name));
            }
            None => {
                self.show_error(rust_i18n::t!("msg_strat_editor_inspect_failed").into_owned());
            }
        }
    }

    pub fn begin_new_strategy_file(&mut self) {
        self.strategy_editor_new_name_editing = true;
        self.strategy_editor_new_name_buf.clear();
        self.status_message = None;
    }

    pub fn commit_new_strategy_file(&mut self) {
        let name = self.strategy_editor_new_name_buf.trim().to_string();
        self.strategy_editor_new_name_editing = false;
        self.strategy_editor_new_name_buf.clear();

        if name.is_empty() {
            return;
        }

        match crate::tui::menus::strategy_editor_menu::create_preset_file(&self.engine, &name) {
            Ok(path) => {
                self.refresh_strategy_editor_files();
                if let Some(index) = self
                    .strategy_editor_files
                    .iter()
                    .position(|entry| entry.path == path)
                {
                    self.strategy_editor_index = index;
                }
                self.refresh_strategy_editor_inspection();
                self.status_message = Some(rust_i18n::t!("msg_strat_editor_new_ok").into_owned());
            }
            Err(error) => self.show_error(error),
        }
    }

    pub fn duplicate_selected_strategy_file(&mut self) {
        let Some(entry) = self.strategy_editor_files.get(self.strategy_editor_index).cloned() else {
            return;
        };

        match crate::tui::menus::strategy_editor_menu::duplicate_file(&entry) {
            Ok(path) => {
                self.refresh_strategy_editor_files();
                if let Some(index) = self
                    .strategy_editor_files
                    .iter()
                    .position(|item| item.path == path)
                {
                    self.strategy_editor_index = index;
                }
                self.refresh_strategy_editor_inspection();
                self.status_message = Some(rust_i18n::t!("msg_strat_editor_clone_ok").into_owned());
            }
            Err(error) => self.show_error(error),
        }
    }

    pub fn begin_delete_selected_strategy_file(&mut self) {
        if self.strategy_editor_index < self.strategy_editor_files.len() {
            self.strategy_editor_delete_confirm = true;
            self.status_message = Some(rust_i18n::t!("msg_strat_editor_delete_confirm").into_owned());
        }
    }

    pub fn confirm_delete_selected_strategy_file(&mut self) {
        self.strategy_editor_delete_confirm = false;

        let Some(entry) = self.strategy_editor_files.get(self.strategy_editor_index).cloned() else {
            return;
        };

        match crate::tui::menus::strategy_editor_menu::delete_file(&entry) {
            Ok(()) => {
                self.refresh_strategy_editor_files();
                self.refresh_strategy_editor_inspection();
                self.status_message = Some(rust_i18n::t!("msg_strat_editor_delete_ok").into_owned());
            }
            Err(error) => self.show_error(error),
        }
    }

    pub fn reload_z2_lists(&mut self) {
        let previously: Vec<String> = self
            .z2_selected_lists
            .iter()
            .filter_map(|&i| self.z2_lists.get(i).cloned())
            .collect();

        self.z2_lists = crate::autotune::available_lists();
        self.z2_selected_lists = self
            .z2_lists
            .iter()
            .enumerate()
            .filter(|(_, name)| previously.contains(name))
            .map(|(i, _)| i)
            .collect();

        if self.z2_list_index > self.z2_lists.len() {
            self.z2_list_index = 0;
        }

        self.refresh_z2_targets();
    }

    pub fn resolved_z2_lists(&self) -> Vec<String> {
        if self.z2_bundle.is_manual() {
            self.z2_selected_lists
                .iter()
                .filter_map(|&i| self.z2_lists.get(i).cloned())
                .collect()
        } else {
            self.z2_bundle.resolve(&self.z2_lists)
        }
    }

    pub fn refresh_z2_targets(&mut self) {
        let lists = self.resolved_z2_lists();

        self.z2_targets = if lists.is_empty() {
            Vec::new()
        } else {
            crate::autotune::domains_from_lists(&lists, crate::autotune::Z2_MAX_TARGETS)
        };
    }

    pub fn cycle_z2_bundle(&mut self, forward: bool) {
        if self.z2_lists.is_empty() {
            self.reload_z2_lists();
        }
        self.z2_bundle = self.z2_bundle.cycle(forward);
        self.refresh_z2_targets();
    }

    pub fn z2_bundle_summary(&self) -> String {
        if self.z2_bundle.is_manual() {
            let count = self.z2_selected_lists.len();
            if count == 0 {
                return rust_i18n::t!("autotune_z2_default_targets").into_owned();
            }
            return format!("{} {}", count, rust_i18n::t!("autotune_z2_lists_suffix"));
        }
        let resolved = self.resolved_z2_lists();
        format!("{} {}", resolved.len(), rust_i18n::t!("autotune_z2_lists_suffix"))
    }

    pub fn z2_selected_preset_names(&self) -> Vec<String> {
        self.z2_selected_presets
            .iter()
            .filter_map(|&i| self.z2_presets.get(i).cloned())
            .collect()
    }

    pub fn z2_config(&self) -> crate::autotune::Z2Config {
        crate::autotune::Z2Config {
            presets: self.z2_selected_preset_names(),
            targets: self.z2_targets.clone(),
            bundle: self.z2_bundle,
            lists: self.resolved_z2_lists(),
        }
    }

    pub fn z2_planned_counts(&self) -> (usize, usize) {
        let presets = if self.z2_selected_presets.is_empty() {
            if self.z2_presets.is_empty() {
                self.strategies.len()
            } else {
                self.z2_presets.len()
            }
        } else {
            self.z2_selected_presets.len()
        };

        let targets = if self.z2_targets.is_empty() {
            crate::autotune::Z2_TARGETS.len()
        } else {
            self.z2_targets.len()
        };

        (presets, targets)
    }

    #[cfg(target_os = "windows")]
    pub fn refresh_defender_status(&mut self) {
        self.defender_status_cache = crate::defender::check_defender_exclusion().ok();
    }

    pub fn refresh_service_status(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if let Some(mgr) = crate::inits::get_detected_manager_for(&self.engine) {
                self.service_installed = mgr.is_installed();
                self.service_active = mgr.is_active();
            } else {
                self.service_installed = false;
                self.service_active = false;
            }
        }
        #[cfg(target_os = "windows")]
        {
            use crate::inits::ServiceManager;
            let mgr = crate::inits::winservice::WindowsServiceManager;
            self.service_installed = mgr.is_installed();
            self.service_active = mgr.is_active();
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            self.service_installed = false;
            self.service_active = false;
        }

        self.service_conflict = crate::inits::legacy_service_conflict(&self.engine);

        let count = self.get_service_menu_count();
        if count > 0 && self.service_menu_index >= count {
            self.service_menu_index = count - 1;
        }
    }

    pub fn refresh_ipset_status(&mut self) {
        self.available_ipset_modes = crate::ipset::get_available_modes_for(&self.engine);
        let current_ipset_mode = crate::ipset::determine_current_mode();
        self.selected_ipset_mode = self
            .available_ipset_modes
            .iter()
            .position(|m| m == &current_ipset_mode)
            .unwrap_or(0);
    }

    fn save_current_config(&self) {
        let interface = self
            .interfaces
            .get(self.selected_interface)
            .map(|s| s.as_str())
            .unwrap_or("any");
        let strategy = self
            .strategies
            .get(self.selected_strategy)
            .map(|s| s.as_str())
            .unwrap_or("");
        #[cfg(target_os = "linux")]
        let backend = self.selected_backend.to_config();
        #[cfg(not(target_os = "linux"))]
        let backend = "nftables";
        let _ = crate::config::save_tui_state(
            &self.engine,
            interface,
            strategy,
            self.tcp_gamefilter,
            self.udp_gamefilter,
            backend,
        );
    }

    fn service_manager(&self) -> Option<Box<dyn crate::inits::ServiceManager>> {
        #[cfg(target_os = "linux")]
        {
            crate::inits::get_detected_manager_for(&self.engine)
        }
        #[cfg(target_os = "windows")]
        {
            Some(Box::new(crate::inits::winservice::WindowsServiceManager))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    fn open_service_conflict(&mut self, action: PendingServiceAction) -> bool {
        if self.service_conflict.is_none() {
            return false;
        }
        self.pending_service_action = Some(action);
        self.service_conflict_index = 1;
        self.active_screen = ActiveScreen::ServiceConflictSubmenu;
        self.status_message = Some(rust_i18n::t!("srv_conflict_status").into_owned());
        true
    }

    pub fn cancel_service_conflict(&mut self) {
        self.pending_service_action = None;
        self.active_screen = ActiveScreen::ServiceSubmenu;
        self.status_message = Some(rust_i18n::t!("srv_conflict_cancelled").into_owned());
    }

    pub fn resolve_service_conflict(&mut self) {
        let action = self.pending_service_action.take();
        self.active_screen = ActiveScreen::ServiceSubmenu;
        self.service_menu_index = 0;

        if let Err(e) = crate::inits::remove_legacy_service() {
            self.refresh_service_status();
            self.show_error(e);
            return;
        }

        self.refresh_service_status();
        match action {
            Some(action) => self.continue_service_action(action),
            None => {
                self.status_message = Some(rust_i18n::t!("msg_op_ok").into_owned());
            }
        }
    }

    fn continue_service_action(&mut self, action: PendingServiceAction) {
        let Some(mgr) = self.service_manager() else {
            self.status_message = Some(rust_i18n::t!("msg_err_init").into_owned());
            return;
        };

        let res = match action {
            PendingServiceAction::Install => match std::env::current_exe() {
                Ok(exe) => {
                    let config_path = crate::config::config_path();
                    let cache_dir = crate::config::get_cache_dir();
                    mgr.install(&exe, &config_path, &cache_dir).and_then(|_| mgr.start())
                }
                Err(e) => Err(e.to_string()),
            },
            PendingServiceAction::Start => mgr.start(),
            PendingServiceAction::Restart => mgr.restart(),
        };

        self.refresh_service_status();
        match res {
            Ok(_) => {
                self.service_menu_index = 0;
                self.status_message = Some(rust_i18n::t!("msg_op_ok").into_owned());
            }
            Err(e) => self.show_error(e),
        }
    }

    pub fn get_service_menu_count(&self) -> usize {
        if !self.service_installed {
            2
        } else if self.service_active {
            4
        } else {
            3
        }
    }

    pub fn check_dependencies(&mut self) -> bool {
        self.refresh_dep_status();
        if !self.nfqws_installed || !self.strategies_installed {
            let msg = if !self.nfqws_installed && !self.strategies_installed {
                rust_i18n::t!("msg_err_both_missing").into_owned()
            } else if !self.nfqws_installed {
                rust_i18n::t!("msg_err_nfqws_missing").into_owned()
            } else {
                rust_i18n::t!("msg_err_strat_missing").into_owned()
            };
            self.show_error(msg);
            false
        } else {
            true
        }
    }

    pub fn next_menu(&mut self) {
        self.move_menu(true);
    }

    pub fn prev_menu(&mut self) {
        self.move_menu(false);
    }

    pub fn move_menu(&mut self, forward: bool) {
        self.status_message = None;
        match self.active_screen {
            ActiveScreen::Main => {
                self.main_menu = if forward {
                    self.main_menu.next_visible(&self.engine)
                } else {
                    self.main_menu.prev_visible(&self.engine)
                }
            }
            #[cfg(target_os = "windows")]
            ActiveScreen::DefenderSubmenu => {
                self.defender_menu = if forward {
                    self.defender_menu.next()
                } else {
                    self.defender_menu.prev()
                };
            }
            ActiveScreen::StrategySubmenu => {
                if !self.strategies.is_empty() {
                    let max = self.strategies.len() + 1;
                    self.strategy_menu_index = Self::cycle_index(self.strategy_menu_index, max, forward);
                }
            }
            ActiveScreen::DownloadDepsSubmenu => {
                self.download_deps_menu = if forward {
                    self.download_deps_menu.next()
                } else {
                    self.download_deps_menu.prev()
                };
            }
            ActiveScreen::DownloadZapretSubmenu => {
                self.download_zapret_menu = if forward {
                    self.download_zapret_menu.next()
                } else {
                    self.download_zapret_menu.prev()
                };
            }
            ActiveScreen::DownloadStrategiesSubmenu => {
                self.download_strategies_menu = if forward {
                    self.download_strategies_menu.next()
                } else {
                    self.download_strategies_menu.prev()
                };
            }
            ActiveScreen::GamefilterSubmenu => {
                self.gamefilter_menu = if forward {
                    self.gamefilter_menu.next()
                } else {
                    self.gamefilter_menu.prev()
                };
            }
            ActiveScreen::FakesSubmenu => {
                self.fakes_menu = if forward {
                    self.fakes_menu.next()
                } else {
                    self.fakes_menu.prev()
                }
            }
            ActiveScreen::FakesSelectSubmenu => {
                let max = self.fakes_state.available.len() + 2;
                if max > 0 {
                    self.fakes_select_index = Self::cycle_index(self.fakes_select_index, max, forward);
                }
            }
            ActiveScreen::ZapretTagSelect => {
                if !self.available_nfqws_tags.is_empty() {
                    let max = self.available_nfqws_tags.len() + 1;
                    self.nfqws_tag_index = Self::cycle_index(self.nfqws_tag_index, max, forward);
                }
            }
            ActiveScreen::StrategyTagSelect => {
                if !self.available_strat_tags.is_empty() {
                    let max = self.available_strat_tags.len() + 1;
                    self.strat_tag_index = Self::cycle_index(self.strat_tag_index, max, forward);
                }
            }
            ActiveScreen::ServiceSubmenu => {
                let count = self.get_service_menu_count();
                if count > 0 {
                    self.service_menu_index = Self::cycle_index(self.service_menu_index, count, forward);
                }
            }
            ActiveScreen::ServiceConflictSubmenu => {
                self.service_conflict_index = Self::cycle_index(self.service_conflict_index, 2, forward);
            }
            ActiveScreen::ListsEditorSubmenu => {
                let max = self.lists_files.len() + 1;
                self.lists_menu_index = Self::cycle_index(self.lists_menu_index, max, forward);
            }
            ActiveScreen::StrategyEditorSubmenu => {
                let max = self.strategy_editor_files.len() + 1;
                self.strategy_editor_index = Self::cycle_index(self.strategy_editor_index, max, forward);
                self.refresh_strategy_editor_inspection();
            }
            ActiveScreen::AutotuneEditDomainsSubmenu => {
                let max = self.domain_files.len() + 1;
                self.domain_files_index = Self::cycle_index(self.domain_files_index, max, forward);
            }
            ActiveScreen::AutotuneSubmenu => {
                let count = self.autotune_menu_states().len();
                self.set_autotune_menu_index(Self::cycle_index(self.autotune_menu_index, count, forward));
            }
            ActiveScreen::AutotuneProtocolsSubmenu => {
                self.autotune_protocols_menu = if forward {
                    self.autotune_protocols_menu.next()
                } else {
                    self.autotune_protocols_menu.prev()
                };
            }
            ActiveScreen::AutotunePresetSelectionSubmenu => {
                let total = crate::autotune::PRESETS.len() + 1;
                if total > 0 {
                    self.autotune_preset_index = Self::cycle_index(self.autotune_preset_index, total, forward);
                }
            }
            ActiveScreen::AutotuneBlockChecksSubmenu => {
                self.autotune_block_checks_menu = if forward {
                    self.autotune_block_checks_menu.next()
                } else {
                    self.autotune_block_checks_menu.prev()
                };
            }
            ActiveScreen::AutotuneStrategiesSubmenu => {
                let max = self.strategies.len() + 1;
                if max > 0 {
                    self.autotune_strat_index = Self::cycle_index(self.autotune_strat_index, max, forward);
                }
            }
            ActiveScreen::AutotuneZ2PresetsSubmenu => {
                let max = self.z2_presets.len() + 1;
                self.z2_preset_index = Self::cycle_index(self.z2_preset_index, max, forward);
            }
            ActiveScreen::AutotuneZ2TargetsSubmenu => {
                let max = self.z2_lists.len() + 1;
                self.z2_list_index = Self::cycle_index(self.z2_list_index, max, forward);
            }
            ActiveScreen::AutotuneResultsSubmenu => {
                let total = self.count_results_items();
                if total > 0 {
                    if forward {
                        if self.autotune_results_index + 1 < total {
                            self.autotune_results_index += 1;
                        }
                    } else if self.autotune_results_index > 0 {
                        self.autotune_results_index -= 1;
                    }
                }
            }
            ActiveScreen::AutotuneServiceMatrixSubmenu => {
                let total = self.count_service_matrix_items();
                if total > 0 {
                    if forward {
                        if self.service_matrix_index + 1 < total {
                            self.service_matrix_index += 1;
                        }
                    } else if self.service_matrix_index > 0 {
                        self.service_matrix_index -= 1;
                    }
                }
            }
            ActiveScreen::SettingsSubmenu => {
                self.settings_menu = if forward {
                    self.settings_menu.next()
                } else {
                    self.settings_menu.prev()
                };
            }
            ActiveScreen::SettingsEditorSubmenu => {
                let max = self.editor_entries.len() + 1;
                self.editor_index = Self::cycle_index(self.editor_index, max, forward);
            }
            ActiveScreen::LogViewer => {
                let total = self.log_lines.len();
                if total > 0 {
                    if forward {
                        if self.log_scroll + 1 < total {
                            self.log_scroll += 1;
                        }
                    } else if self.log_scroll > 0 {
                        self.log_scroll -= 1;
                    }
                }
            }
        }
    }

    fn cycle_index(current: usize, max: usize, forward: bool) -> usize {
        if max == 0 {
            return current;
        }
        if forward {
            (current + 1) % max
        } else {
            (current + max - 1) % max
        }
    }

    pub fn autotune_menu_states(&self) -> Vec<AutotuneMenuState> {
        if self.engine.uses_presets() {
            vec![
                AutotuneMenuState::Z2Bundle,
                AutotuneMenuState::Z2Targets,
                AutotuneMenuState::Z2Presets,
                AutotuneMenuState::ServiceMatrix,
                AutotuneMenuState::Results,
                AutotuneMenuState::Run,
                AutotuneMenuState::Back,
            ]
        } else {
            vec![
                AutotuneMenuState::PresetSelection,
                AutotuneMenuState::NumRequests,
                AutotuneMenuState::Strategies,
                AutotuneMenuState::Protocols,
                AutotuneMenuState::BlockChecks,
                AutotuneMenuState::EditDomains,
                AutotuneMenuState::ServiceMatrix,
                AutotuneMenuState::Results,
                AutotuneMenuState::Run,
                AutotuneMenuState::Back,
            ]
        }
    }

    fn set_autotune_menu_index(&mut self, index: usize) {
        let states = self.autotune_menu_states();
        if states.is_empty() {
            return;
        }
        self.autotune_menu_index = index % states.len();
        self.autotune_menu = states[self.autotune_menu_index];
    }

    fn toggle_block_check(&mut self, index: usize) {
        let mut bc = self.autotune_config.block_checks.clone();
        bc.set(index, !bc.get(index));
        self.autotune_config.block_checks = bc;
    }

    pub fn toggle_current(&mut self) {
        match self.active_screen {
            ActiveScreen::Main => match self.main_menu {
                #[cfg(target_os = "windows")]
                MainMenuState::DefenderSettings => {
                    self.active_screen = ActiveScreen::DefenderSubmenu;
                    self.refresh_defender_status();
                    self.status_message = None;
                }
                MainMenuState::DownloadDeps => {
                    self.active_screen = ActiveScreen::DownloadDepsSubmenu;
                    self.status_message = None;
                }
                MainMenuState::Engine => {
                    self.switch_engine();
                }
                MainMenuState::Interface => {
                    if !self.interfaces.is_empty() {
                        self.selected_interface = (self.selected_interface + 1) % self.interfaces.len();
                        self.save_current_config();
                    }
                }
                #[cfg(target_os = "linux")]
                MainMenuState::BackendSettings => {
                    let backends = LinuxBackend::variants();
                    if !backends.is_empty() {
                        let current_idx = backends.iter().position(|b| *b == self.selected_backend).unwrap_or(0);
                        let new_idx = (current_idx + 1) % backends.len();
                        self.selected_backend = backends[new_idx];
                        self.save_current_config();
                    }
                }
                MainMenuState::IpsetMode => {
                    if !self.available_ipset_modes.is_empty() {
                        let old_mode = self.available_ipset_modes[self.selected_ipset_mode];
                        self.selected_ipset_mode = (self.selected_ipset_mode + 1) % self.available_ipset_modes.len();
                        let new_mode = self.available_ipset_modes[self.selected_ipset_mode];
                        crate::ipset::apply_ipset_mode(old_mode, new_mode);
                        self.available_ipset_modes = crate::ipset::get_available_modes_for(&self.engine);
                        self.selected_ipset_mode = self
                            .available_ipset_modes
                            .iter()
                            .position(|m| m == &new_mode)
                            .unwrap_or(0);
                    }
                }
                MainMenuState::Strategy => {
                    self.active_screen = ActiveScreen::StrategySubmenu;
                    self.strategy_menu_index = self.selected_strategy;
                    self.status_message = None;
                }
                MainMenuState::GamefilterSettings => {
                    if self.engine.supports_game_filter() {
                        self.active_screen = ActiveScreen::GamefilterSubmenu;
                        self.gamefilter_menu = GamefilterMenuState::Tcp;
                        self.status_message = None;
                    } else {
                        self.status_message = Some(rust_i18n::t!("msg_gf_preset_managed").into_owned());
                    }
                }
                MainMenuState::ServiceSettings => {
                    self.active_screen = ActiveScreen::ServiceSubmenu;
                    self.service_menu_index = 0;
                    self.refresh_service_status();
                    self.status_message = None;
                }
                MainMenuState::ListsEditor => {
                    if !crate::download::check_strategies_installed_for(&self.engine) {
                        self.show_error(rust_i18n::t!("err_no_strats").into_owned());
                    } else {
                        self.lists_files = crate::utils::get_lists_files();
                        self.lists_menu_index = 0;
                        self.active_screen = ActiveScreen::ListsEditorSubmenu;
                        self.status_message = None;
                    }
                }
                MainMenuState::StrategyEditor => {
                    if !self.engine.uses_presets() {
                        self.status_message = Some(rust_i18n::t!("msg_gf_preset_managed").into_owned());
                    } else {
                        self.refresh_strategy_editor_files();
                        self.strategy_editor_index = 0;
                        self.refresh_strategy_editor_inspection();
                        self.active_screen = ActiveScreen::StrategyEditorSubmenu;
                        self.status_message = None;
                    }
                }
                MainMenuState::Autotune => {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.set_autotune_menu_index(0);
                    self.has_autotune_results_file = crate::autotune::load_results_file().is_some();
                    self.status_message = None;
                }
                MainMenuState::FakesSettings => {
                    self.fakes_state = crate::fakes::load_fakes_state();
                    self.active_screen = ActiveScreen::FakesSubmenu;
                    self.fakes_menu = FakesMenuState::DiscordUdp;
                    self.status_message = None;
                }
                MainMenuState::Settings => {
                    self.refresh_editors();
                    self.backup_lists = crate::config::load_backup_lists();
                    self.settings_menu = SettingsMenuState::Editor;
                    self.active_screen = ActiveScreen::SettingsSubmenu;
                    self.status_message = None;
                }
                MainMenuState::TtlAutopick => {
                    if self.check_dependencies() {
                        self.should_run_ttl = true;
                    }
                }
                MainMenuState::Run => {
                    if self.check_dependencies() {
                        self.should_run = true;
                    }
                }
                MainMenuState::Quit => self.should_quit = true,
            },
            #[cfg(target_os = "windows")]
            ActiveScreen::DefenderSubmenu => match self.defender_menu {
                DefenderMenuState::Add => match crate::defender::add_defender_exclusion() {
                    Ok(_) => {
                        self.status_message = Some(rust_i18n::t!("msg_def_add_ok").into_owned());
                        self.refresh_defender_status();
                    }
                    Err(e) => self.status_message = Some(format!("{}{}", rust_i18n::t!("msg_err"), e)),
                },
                DefenderMenuState::Remove => match crate::defender::remove_defender_exclusion() {
                    Ok(_) => {
                        self.status_message = Some(rust_i18n::t!("msg_def_rm_ok").into_owned());
                        self.refresh_defender_status();
                    }
                    Err(e) => self.status_message = Some(format!("{}{}", rust_i18n::t!("msg_err"), e)),
                },
                DefenderMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::StrategySubmenu => {
                if self.strategy_menu_index < self.strategies.len() {
                    self.selected_strategy = self.strategy_menu_index;
                    self.save_current_config();
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = Some(format!(
                        "{}{}",
                        rust_i18n::t!("msg_strat_sel"),
                        self.strategies[self.selected_strategy]
                    ));
                } else {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            }
            ActiveScreen::DownloadDepsSubmenu => match self.download_deps_menu {
                DownloadDepsMenuState::ZapretDownloader => {
                    self.active_screen = ActiveScreen::DownloadZapretSubmenu;
                    self.download_zapret_menu = DownloadSubmenuState::Version;
                    self.status_message = None;
                }
                DownloadDepsMenuState::Zapret2Downloader => {
                    self.should_download_zapret2 = true;
                }
                DownloadDepsMenuState::Zapret2Strategies => {
                    self.should_download_zapret2_strategies = true;
                }
                DownloadDepsMenuState::StrategiesDownloader => {
                    self.active_screen = ActiveScreen::DownloadStrategiesSubmenu;
                    self.download_strategies_menu = DownloadSubmenuState::Version;
                    self.status_message = None;
                }
                DownloadDepsMenuState::DownloadDefaults => {
                    self.should_download_defaults = true;
                }
                DownloadDepsMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::DownloadZapretSubmenu => match self.download_zapret_menu {
                DownloadSubmenuState::Version => {
                    self.nfqws_target = self.nfqws_target.cycle(true);
                }
                DownloadSubmenuState::SelectTag => {
                    self.status_message = Some(rust_i18n::t!("msg_fetch_zapret_tags").into_owned());
                    match crate::download::fetch_repo_tags("bol-van/zapret") {
                        Ok(tags) => {
                            self.available_nfqws_tags = tags;
                            self.nfqws_tag_index = 0;
                            self.active_screen = ActiveScreen::ZapretTagSelect;
                            self.status_message = None;
                        }
                        Err(e) => {
                            self.show_error(format!("{}{}", rust_i18n::t!("msg_err_fetch_tags"), e));
                        }
                    }
                }
                DownloadSubmenuState::Start => {
                    self.should_download_zapret = true;
                }
                DownloadSubmenuState::Back => {
                    self.active_screen = ActiveScreen::DownloadDepsSubmenu;
                    self.status_message = None;
                }
            },
            ActiveScreen::DownloadStrategiesSubmenu => match self.download_strategies_menu {
                DownloadSubmenuState::Version => {
                    self.strat_target = self.strat_target.cycle(true);
                }
                DownloadSubmenuState::SelectTag => {
                    self.status_message = Some(rust_i18n::t!("msg_fetch_strat_tags").into_owned());
                    match crate::download::fetch_repo_tags("Flowseal/zapret-discord-youtube") {
                        Ok(tags) => {
                            self.available_strat_tags = tags;
                            self.strat_tag_index = 0;
                            self.active_screen = ActiveScreen::StrategyTagSelect;
                            self.status_message = None;
                        }
                        Err(e) => {
                            self.show_error(format!("{}{}", rust_i18n::t!("msg_err_fetch_tags"), e));
                        }
                    }
                }
                DownloadSubmenuState::Start => {
                    self.should_download_strategies = true;
                }
                DownloadSubmenuState::Back => {
                    self.active_screen = ActiveScreen::DownloadDepsSubmenu;
                    self.status_message = None;
                }
            },
            ActiveScreen::ZapretTagSelect => {
                if self.nfqws_tag_index < self.available_nfqws_tags.len() {
                    let selected = self.available_nfqws_tags[self.nfqws_tag_index].clone();
                    self.nfqws_target = VersionTarget::Tag(selected);
                    self.active_screen = ActiveScreen::DownloadZapretSubmenu;
                    self.status_message = Some(rust_i18n::t!("msg_zapret_tag_sel").into_owned());
                } else {
                    self.active_screen = ActiveScreen::DownloadZapretSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::StrategyTagSelect => {
                if self.strat_tag_index < self.available_strat_tags.len() {
                    let selected = self.available_strat_tags[self.strat_tag_index].clone();
                    self.strat_target = VersionTarget::Tag(selected);
                    self.active_screen = ActiveScreen::DownloadStrategiesSubmenu;
                    self.status_message = Some(rust_i18n::t!("msg_strat_tag_sel").into_owned());
                } else {
                    self.active_screen = ActiveScreen::DownloadStrategiesSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::GamefilterSubmenu => match self.gamefilter_menu {
                GamefilterMenuState::Tcp => {
                    self.tcp_gamefilter = !self.tcp_gamefilter;
                    self.save_current_config();
                }
                GamefilterMenuState::Udp => {
                    self.udp_gamefilter = !self.udp_gamefilter;
                    self.save_current_config();
                }
                GamefilterMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::FakesSubmenu => match self.fakes_menu {
                FakesMenuState::DiscordUdp => {
                    self.fakes_select_for = FakesSelectTarget::DiscordUdp;
                    self.fakes_select_index = if self.fakes_state.available.is_empty() { 0 } else { 1 };
                    self.active_screen = ActiveScreen::FakesSelectSubmenu;
                    self.status_message = None;
                }
                FakesMenuState::GameUdp => {
                    self.fakes_select_for = FakesSelectTarget::GameUdp;
                    self.fakes_select_index = if self.fakes_state.available.is_empty() { 0 } else { 1 };
                    self.active_screen = ActiveScreen::FakesSelectSubmenu;
                    self.status_message = None;
                }
                FakesMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::FakesSelectSubmenu => {
                let file_count = self.fakes_state.available.len();
                if self.fakes_select_index >= 1 && self.fakes_select_index <= file_count {
                    let source_idx = self.fakes_select_index - 1;
                    let source = self.fakes_state.available[source_idx].clone();
                    let target: crate::fakes::FakeTarget = match self.fakes_select_for {
                        FakesSelectTarget::DiscordUdp => crate::fakes::FakeTarget::DiscordUdp,
                        FakesSelectTarget::GameUdp => crate::fakes::FakeTarget::GameUdp,
                    };
                    match crate::fakes::replace_active_fake(&self.fakes_state, &target, &source) {
                        Ok(()) => {
                            self.fakes_state = crate::fakes::load_fakes_state();
                            self.active_screen = ActiveScreen::FakesSubmenu;
                            self.status_message = Some(rust_i18n::t!("msg_fakes_replaced").into_owned());
                        }
                        Err(e) => {
                            self.show_error(e);
                        }
                    }
                } else {
                    self.active_screen = ActiveScreen::FakesSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::ServiceSubmenu => {
                let mgr_opt = self.service_manager();

                if let Some(mgr) = mgr_opt {
                    let mut action_taken = true;
                    let res = if !self.service_installed {
                        match self.service_menu_index {
                            0 => {
                                if !self.check_dependencies() {
                                    action_taken = false;
                                    Ok(())
                                } else if self.open_service_conflict(PendingServiceAction::Install) {
                                    action_taken = false;
                                    Ok(())
                                } else {
                                    let exe_path = std::env::current_exe().map_err(|e| e.to_string());
                                    match exe_path {
                                        Ok(p) => {
                                            let config_path = crate::config::config_path();
                                            let cache_dir = crate::config::get_cache_dir();
                                            mgr.install(&p, &config_path, &cache_dir).and_then(|_| mgr.start())
                                        }
                                        Err(e) => Err(e),
                                    }
                                }
                            }
                            1 => {
                                self.active_screen = ActiveScreen::Main;
                                self.status_message = None;
                                action_taken = false;
                                Ok(())
                            }
                            _ => {
                                action_taken = false;
                                Ok(())
                            }
                        }
                    } else if self.service_active {
                        match self.service_menu_index {
                            0 => mgr.stop(),
                            1 => {
                                if !self.check_dependencies() {
                                    action_taken = false;
                                    Ok(())
                                } else if self.open_service_conflict(PendingServiceAction::Restart) {
                                    action_taken = false;
                                    Ok(())
                                } else {
                                    mgr.restart()
                                }
                            }
                            2 => mgr.uninstall(),
                            3 => {
                                self.active_screen = ActiveScreen::Main;
                                self.status_message = None;
                                action_taken = false;
                                Ok(())
                            }
                            _ => {
                                action_taken = false;
                                Ok(())
                            }
                        }
                    } else {
                        match self.service_menu_index {
                            0 => {
                                if self.open_service_conflict(PendingServiceAction::Start) {
                                    action_taken = false;
                                    Ok(())
                                } else {
                                    mgr.start()
                                }
                            }
                            1 => mgr.uninstall(),
                            2 => {
                                self.active_screen = ActiveScreen::Main;
                                self.status_message = None;
                                action_taken = false;
                                Ok(())
                            }
                            _ => {
                                action_taken = false;
                                Ok(())
                            }
                        }
                    };

                    if action_taken {
                        match res {
                            Ok(_) => {
                                self.refresh_service_status();
                                self.service_menu_index = 0;
                                self.status_message = Some(rust_i18n::t!("msg_op_ok").into_owned());
                            }
                            Err(e) => {
                                self.refresh_service_status();
                                self.show_error(e);
                            }
                        }
                    }
                } else {
                    self.status_message = Some(rust_i18n::t!("msg_err_init").into_owned());
                }
            }
            ActiveScreen::ServiceConflictSubmenu => match self.service_conflict_index {
                0 => self.resolve_service_conflict(),
                _ => self.cancel_service_conflict(),
            },
            ActiveScreen::ListsEditorSubmenu => {
                if self.lists_menu_index < self.lists_files.len() {
                    let file = self.lists_files[self.lists_menu_index].clone();
                    self.should_open_editor = Some(file);
                } else {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            }
            ActiveScreen::StrategyEditorSubmenu => {
                if self.strategy_editor_index < self.strategy_editor_files.len() {
                    self.open_highlighted_strategy_file();
                } else {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            }
            ActiveScreen::AutotuneSubmenu => match self.autotune_menu {
                AutotuneMenuState::PresetSelection => {}
                AutotuneMenuState::Z2Bundle => {
                    self.cycle_z2_bundle(true);
                }
                AutotuneMenuState::Z2Presets => {
                    self.reload_z2_presets();
                    if self.z2_presets.is_empty() {
                        self.show_error(rust_i18n::t!("err_no_strats").into_owned());
                    } else {
                        self.z2_preset_index = 0;
                        self.active_screen = ActiveScreen::AutotuneZ2PresetsSubmenu;
                        self.status_message = None;
                    }
                }
                AutotuneMenuState::Z2Targets => {
                    self.reload_z2_lists();
                    if self.z2_lists.is_empty() {
                        self.show_error(rust_i18n::t!("autotune_z2_no_lists").into_owned());
                    } else {
                        if !self.z2_bundle.is_manual() {
                            let preselected = self.resolved_z2_lists();
                            self.z2_selected_lists = self
                                .z2_lists
                                .iter()
                                .enumerate()
                                .filter(|(_, name)| preselected.contains(name))
                                .map(|(i, _)| i)
                                .collect();
                            self.z2_bundle = crate::autotune::TargetBundle::Manual;
                            self.refresh_z2_targets();
                        }
                        self.z2_list_index = 0;
                        self.active_screen = ActiveScreen::AutotuneZ2TargetsSubmenu;
                        self.status_message = Some(rust_i18n::t!("autotune_z2_manual_switch").into_owned());
                    }
                }
                AutotuneMenuState::NumRequests => {}
                AutotuneMenuState::Strategies => {
                    if !self.strategies.is_empty() {
                        self.active_screen = ActiveScreen::AutotuneStrategiesSubmenu;
                        self.autotune_strat_index = 0;
                        self.status_message = None;
                    } else {
                        self.show_error(rust_i18n::t!("err_no_strats").into_owned());
                    }
                }
                AutotuneMenuState::Protocols => {
                    self.active_screen = ActiveScreen::AutotuneProtocolsSubmenu;
                    self.autotune_protocols_menu = AutotuneProtocolsState::Http;
                    self.status_message = None;
                }
                AutotuneMenuState::BlockChecks => {
                    self.active_screen = ActiveScreen::AutotuneBlockChecksSubmenu;
                    self.autotune_block_checks_menu = AutotuneBlockChecksState::DnsSpoof;
                    self.status_message = None;
                }
                AutotuneMenuState::EditDomains => {
                    self.active_screen = ActiveScreen::AutotuneEditDomainsSubmenu;
                    self.domain_files_index = 0;
                    self.status_message = None;
                }
                AutotuneMenuState::ServiceMatrix => {
                    self.should_run_service_matrix = true;
                    self.status_message = None;
                }
                AutotuneMenuState::Results => {
                    self.active_screen = ActiveScreen::AutotuneResultsSubmenu;
                    self.autotune_results_index = 0;
                    self.status_message = None;
                }
                AutotuneMenuState::Run => {
                    if crate::platform::is_nfqws_running() {
                        self.status_message = Some(rust_i18n::t!("autotune_err_nfqws_running").into_owned());
                    } else {
                        self.should_run_autotune = true;
                    }
                }
                AutotuneMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::AutotuneEditDomainsSubmenu => {
                if self.domain_files_index < self.domain_files.len() {
                    let file = self.domain_files[self.domain_files_index].1.clone();
                    self.should_open_editor = Some(file);
                } else {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::AutotuneProtocolsSubmenu => match self.autotune_protocols_menu {
                AutotuneProtocolsState::Http => {
                    self.autotune_config.check_http = !self.autotune_config.check_http;
                }
                AutotuneProtocolsState::Tls12 => {
                    self.autotune_config.check_tls12 = !self.autotune_config.check_tls12;
                }
                AutotuneProtocolsState::Tls13 => {
                    self.autotune_config.check_tls13 = !self.autotune_config.check_tls13;
                }
                AutotuneProtocolsState::Quic => {
                    self.autotune_config.check_quic = !self.autotune_config.check_quic;
                }
                AutotuneProtocolsState::Back => {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                }
            },
            ActiveScreen::AutotuneBlockChecksSubmenu => {
                if self.autotune_block_checks_menu == AutotuneBlockChecksState::Back {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                } else {
                    self.toggle_block_check(self.autotune_block_checks_menu.index());
                }
            }
            ActiveScreen::AutotunePresetSelectionSubmenu => {
                let idx = self.autotune_preset_index;
                if idx >= crate::autotune::PRESETS.len() {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                    return;
                }
                let was_selected = self.autotune_config.preset_indices.contains(&idx);
                if was_selected {
                    self.autotune_config.preset_indices.retain(|&i| i != idx);
                } else {
                    self.autotune_config.preset_indices.push(idx);
                }
                self.autotune_config.preset_indices.sort();
                self.autotune_config.preset_indices.dedup();
                let names: Vec<&str> = self
                    .autotune_config
                    .preset_indices
                    .iter()
                    .filter_map(|&i| {
                        if i < crate::autotune::PRESETS.len() {
                            Some(crate::autotune::PRESETS[i].name)
                        } else {
                            None
                        }
                    })
                    .collect();
                let label = if names.is_empty() {
                    rust_i18n::t!("menu_autotune_preset_none").into_owned()
                } else {
                    names.join(", ")
                };
                self.status_message = Some(format!("{}: {}", rust_i18n::t!("autotune_preset_sel"), label));
            }
            ActiveScreen::AutotuneZ2PresetsSubmenu => {
                let idx = self.z2_preset_index;
                if idx >= self.z2_presets.len() {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                    return;
                }
                if let Some(pos) = self.z2_selected_presets.iter().position(|&i| i == idx) {
                    self.z2_selected_presets.remove(pos);
                } else {
                    self.z2_selected_presets.push(idx);
                }
                self.z2_selected_presets.sort();
                self.z2_selected_presets.dedup();
            }
            ActiveScreen::AutotuneZ2TargetsSubmenu => {
                let idx = self.z2_list_index;
                if idx >= self.z2_lists.len() {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                    return;
                }
                if let Some(pos) = self.z2_selected_lists.iter().position(|&i| i == idx) {
                    self.z2_selected_lists.remove(pos);
                } else {
                    self.z2_selected_lists.push(idx);
                }
                self.z2_selected_lists.sort();
                self.z2_selected_lists.dedup();
                self.refresh_z2_targets();
            }
            ActiveScreen::AutotuneStrategiesSubmenu => {
                let max = self.strategies.len();
                if self.autotune_strat_index < max {
                    let idx = self.autotune_strat_index;
                    if let Some(pos) = self.autotune_config.strategy_indices.iter().position(|&i| i == idx) {
                        self.autotune_config.strategy_indices.remove(pos);
                    } else {
                        self.autotune_config.strategy_indices.push(idx);
                    }
                } else {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::AutotuneResultsSubmenu => {
                self.active_screen = ActiveScreen::AutotuneSubmenu;
                self.status_message = None;
            }
            ActiveScreen::AutotuneServiceMatrixSubmenu => {
                self.should_run_service_matrix = true;
                self.status_message = None;
            }
            ActiveScreen::SettingsSubmenu => match self.settings_menu {
                SettingsMenuState::Editor => {
                    self.refresh_editors();
                    self.active_screen = ActiveScreen::SettingsEditorSubmenu;
                    self.status_message = None;
                }
                SettingsMenuState::BackupLists => {
                    self.toggle_backup_lists();
                }
                SettingsMenuState::ViewLogs => {
                    self.load_logs();
                    self.active_screen = ActiveScreen::LogViewer;
                    self.status_message = None;
                }
                SettingsMenuState::Back => {
                    self.active_screen = ActiveScreen::Main;
                    self.status_message = None;
                }
            },
            ActiveScreen::SettingsEditorSubmenu => {
                if self.editor_index < self.editor_entries.len() {
                    self.apply_editor_selection(self.editor_index);
                } else {
                    self.active_screen = ActiveScreen::SettingsSubmenu;
                    self.status_message = None;
                }
            }
            ActiveScreen::LogViewer => {
                self.active_screen = ActiveScreen::SettingsSubmenu;
                self.status_message = None;
            }
        }
    }

    fn count_results_items(&self) -> usize {
        if let Some(ref results) = self.autotune_results {
            let mut n = 10;
            for pr in &results.preset_results {
                n += 1;
                n += pr.domain_checks.len();
                if !pr.strategy_results.is_empty() {
                    n += 1;
                    for sr in &pr.strategy_results {
                        n += 1;
                        n += sr.domain_checks.len();
                    }
                    let working_count = pr.strategy_results.iter().filter(|s| s.works).count();
                    if working_count > 0 {
                        n += 1;
                        n += working_count;
                    }
                }
                n += 1;
            }
            if !results.common_strategies.is_empty() {
                n += 1;
                n += results.common_strategies.len();
                n += 1;
            }
            n += 1;
            n
        } else if let Some(cached) = crate::autotune::load_results_file() {
            cached.lines().count() + 1
        } else {
            2
        }
    }

    pub fn service_matrix_label(&self) -> String {
        self.strategies
            .get(self.selected_strategy)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| rust_i18n::t!("autotune_matrix_current").into_owned())
    }

    pub fn service_matrix_profiles(&self) -> Vec<String> {
        let live = self.service_matrix_label();
        let selected: Vec<String> = if self.engine.uses_presets() {
            self.z2_selected_preset_names()
        } else {
            self.autotune_config
                .strategy_indices
                .iter()
                .filter_map(|&index| self.strategies.get(index).cloned())
                .collect()
        };

        let mut profiles: Vec<String> = Vec::new();
        for name in selected {
            if name != live && !profiles.contains(&name) {
                profiles.push(name);
            }
        }
        profiles
    }

    pub fn count_service_matrix_items(&self) -> usize {
        if self.service_matrix_rows.is_empty() {
            return 2;
        }

        let mut total = crate::autotune::render_matrix(&self.service_matrix_rows).len();
        total += 2;
        if !self.service_matrix_reports.is_empty() {
            total += 1 + self.service_matrix_reports.len();
        }
        total += 2;
        total
    }

    pub fn service_matrix_summary(&self) -> String {
        if self.service_matrix_rows.is_empty() {
            return rust_i18n::t!("menu_autotune_matrix_none").into_owned();
        }

        let usable = self
            .service_matrix_rows
            .iter()
            .filter(|row| row.unblocks_everything())
            .count();

        format!("{} / {}", usable, self.service_matrix_rows.len())
    }

    pub fn is_ttl_autopick_selected(&self) -> bool {
        self.active_screen == ActiveScreen::Main && self.main_menu == MainMenuState::TtlAutopick
    }

    pub fn change_ttl(&mut self, forward: bool) {
        let len = crate::ttl::TTL_MAX as i32 + 1;
        let current = self.dpi_desync_ttl.map_or(0, |v| v as i32);
        let new = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        self.dpi_desync_ttl = if new == 0 { None } else { Some(new as u8) };
        let _ = crate::config::save_ttl(self.dpi_desync_ttl);
    }

    pub fn cycle_current(&mut self, forward: bool) {
        match self.active_screen {
            ActiveScreen::Main => match self.main_menu {
                MainMenuState::Engine => {
                    self.switch_engine();
                }
                MainMenuState::Interface => {
                    if !self.interfaces.is_empty() {
                        let len = self.interfaces.len();
                        if forward {
                            self.selected_interface = (self.selected_interface + 1) % len;
                        } else {
                            self.selected_interface = (self.selected_interface + len - 1) % len;
                        }
                        self.save_current_config();
                    }
                }
                #[cfg(target_os = "linux")]
                MainMenuState::BackendSettings => {
                    let backends = LinuxBackend::variants();
                    if !backends.is_empty() {
                        let current_idx = backends.iter().position(|b| *b == self.selected_backend).unwrap_or(0);
                        let len = backends.len();
                        let new_idx = if forward {
                            (current_idx + 1) % len
                        } else {
                            (current_idx + len - 1) % len
                        };
                        self.selected_backend = backends[new_idx];
                        self.save_current_config();
                    }
                }
                MainMenuState::IpsetMode => {
                    if !self.available_ipset_modes.is_empty() {
                        let len = self.available_ipset_modes.len();
                        let old_mode = self.available_ipset_modes[self.selected_ipset_mode];
                        if forward {
                            self.selected_ipset_mode = (self.selected_ipset_mode + 1) % len;
                        } else {
                            self.selected_ipset_mode = (self.selected_ipset_mode + len - 1) % len;
                        }
                        let new_mode = self.available_ipset_modes[self.selected_ipset_mode];
                        crate::ipset::apply_ipset_mode(old_mode, new_mode);
                        self.available_ipset_modes = crate::ipset::get_available_modes_for(&self.engine);
                        self.selected_ipset_mode = self
                            .available_ipset_modes
                            .iter()
                            .position(|m| m == &new_mode)
                            .unwrap_or(0);
                    }
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::DownloadZapretSubmenu => match self.download_zapret_menu {
                DownloadSubmenuState::Version => {
                    self.nfqws_target = self.nfqws_target.cycle(forward);
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::DownloadStrategiesSubmenu => match self.download_strategies_menu {
                DownloadSubmenuState::Version => {
                    self.strat_target = self.strat_target.cycle(forward);
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::GamefilterSubmenu => match self.gamefilter_menu {
                GamefilterMenuState::Tcp => {
                    self.tcp_gamefilter = !self.tcp_gamefilter;
                    self.save_current_config();
                }
                GamefilterMenuState::Udp => {
                    self.udp_gamefilter = !self.udp_gamefilter;
                    self.save_current_config();
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::FakesSubmenu => match self.fakes_menu {
                FakesMenuState::DiscordUdp | FakesMenuState::GameUdp => {
                    if forward {
                        self.toggle_current();
                    }
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::FakesSelectSubmenu => {
                self.toggle_current();
            }
            ActiveScreen::AutotuneSubmenu => match self.autotune_menu {
                AutotuneMenuState::PresetSelection => {
                    self.active_screen = ActiveScreen::AutotunePresetSelectionSubmenu;
                    let count = crate::autotune::PRESETS.len();
                    if self.autotune_preset_index >= count {
                        self.autotune_preset_index = count - 1;
                    }
                    self.status_message = None;
                }
                AutotuneMenuState::NumRequests => {
                    if forward {
                        self.autotune_request_buf = self.autotune_config.num_requests.to_string();
                        self.autotune_request_editing = true;
                    }
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::AutotuneProtocolsSubmenu => match self.autotune_protocols_menu {
                AutotuneProtocolsState::Http => {
                    self.autotune_config.check_http = !self.autotune_config.check_http;
                }
                AutotuneProtocolsState::Tls12 => {
                    self.autotune_config.check_tls12 = !self.autotune_config.check_tls12;
                }
                AutotuneProtocolsState::Tls13 => {
                    self.autotune_config.check_tls13 = !self.autotune_config.check_tls13;
                }
                AutotuneProtocolsState::Quic => {
                    self.autotune_config.check_quic = !self.autotune_config.check_quic;
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::AutotuneBlockChecksSubmenu => match self.autotune_block_checks_menu {
                AutotuneBlockChecksState::Back => {
                    self.active_screen = ActiveScreen::AutotuneSubmenu;
                    self.status_message = None;
                }
                _ => {
                    self.toggle_current();
                }
            },
            ActiveScreen::AutotunePresetSelectionSubmenu => {
                self.toggle_current();
            }
            ActiveScreen::AutotuneStrategiesSubmenu => {
                self.toggle_current();
            }
            ActiveScreen::AutotuneZ2PresetsSubmenu => {
                self.toggle_current();
            }
            ActiveScreen::AutotuneZ2TargetsSubmenu => {
                self.toggle_current();
            }
            ActiveScreen::AutotuneResultsSubmenu => {
                self.active_screen = ActiveScreen::AutotuneSubmenu;
                self.status_message = None;
            }
            ActiveScreen::SettingsSubmenu => match self.settings_menu {
                SettingsMenuState::BackupLists => {
                    self.toggle_backup_lists();
                }
                _ => {
                    if forward {
                        self.toggle_current();
                    }
                }
            },
            ActiveScreen::SettingsEditorSubmenu => {
                if forward {
                    self.toggle_current();
                }
            }
            ActiveScreen::LogViewer => {
                self.active_screen = ActiveScreen::SettingsSubmenu;
                self.status_message = None;
            }
            _ => {
                if forward {
                    self.toggle_current();
                }
            }
        }
    }

    pub fn screen_title(&self) -> String {
        match self.active_screen {
            ActiveScreen::Main => rust_i18n::t!("tui_title_main").into_owned(),
            #[cfg(target_os = "windows")]
            ActiveScreen::DefenderSubmenu => rust_i18n::t!("tui_title_defender").into_owned(),
            ActiveScreen::StrategySubmenu => rust_i18n::t!("tui_title_strategy").into_owned(),
            ActiveScreen::DownloadDepsSubmenu => rust_i18n::t!("tui_title_download_cat").into_owned(),
            ActiveScreen::DownloadZapretSubmenu => rust_i18n::t!("tui_title_download_zapret").into_owned(),
            ActiveScreen::DownloadStrategiesSubmenu => rust_i18n::t!("tui_title_download_strat").into_owned(),
            ActiveScreen::GamefilterSubmenu => rust_i18n::t!("tui_title_gamefilter").into_owned(),
            ActiveScreen::FakesSubmenu => rust_i18n::t!("tui_title_fakes").into_owned(),
            ActiveScreen::FakesSelectSubmenu => rust_i18n::t!("menu_fakes_select_title").into_owned(),
            ActiveScreen::ZapretTagSelect => rust_i18n::t!("tui_title_tag_zapret").into_owned(),
            ActiveScreen::StrategyTagSelect => rust_i18n::t!("tui_title_tag_strat").into_owned(),
            ActiveScreen::ServiceSubmenu => rust_i18n::t!("tui_title_service").into_owned(),
            ActiveScreen::ServiceConflictSubmenu => rust_i18n::t!("tui_title_service_conflict").into_owned(),
            ActiveScreen::ListsEditorSubmenu => rust_i18n::t!("tui_title_lists").into_owned(),
            ActiveScreen::StrategyEditorSubmenu => rust_i18n::t!("tui_title_strategy_editor").into_owned(),
            ActiveScreen::AutotuneSubmenu => rust_i18n::t!("tui_title_autotune").into_owned(),
            ActiveScreen::AutotuneEditDomainsSubmenu => rust_i18n::t!("tui_title_autotune_edit_domains").into_owned(),
            ActiveScreen::AutotuneProtocolsSubmenu => rust_i18n::t!("tui_title_autotune_proto").into_owned(),
            ActiveScreen::AutotuneBlockChecksSubmenu => rust_i18n::t!("tui_title_autotune_bc").into_owned(),
            ActiveScreen::AutotunePresetSelectionSubmenu => rust_i18n::t!("tui_title_autotune_presets").into_owned(),
            ActiveScreen::AutotuneStrategiesSubmenu => rust_i18n::t!("tui_title_autotune_strat").into_owned(),
            ActiveScreen::AutotuneZ2PresetsSubmenu => rust_i18n::t!("tui_title_autotune_z2_presets").into_owned(),
            ActiveScreen::AutotuneZ2TargetsSubmenu => rust_i18n::t!("tui_title_autotune_z2_targets").into_owned(),
            ActiveScreen::AutotuneResultsSubmenu => rust_i18n::t!("tui_title_autotune_results").into_owned(),
            ActiveScreen::AutotuneServiceMatrixSubmenu => rust_i18n::t!("tui_title_autotune_matrix").into_owned(),
            ActiveScreen::SettingsSubmenu => rust_i18n::t!("tui_title_settings").into_owned(),
            ActiveScreen::SettingsEditorSubmenu => rust_i18n::t!("tui_title_settings_editor").into_owned(),
            ActiveScreen::LogViewer => rust_i18n::t!("tui_title_logs").into_owned(),
        }
    }

    pub fn breadcrumb(&self) -> Vec<String> {
        let root = rust_i18n::t!("breadcrumb_root").into_owned();
        let downloader = rust_i18n::t!("menu_main_downloader").into_owned();
        let autotune = rust_i18n::t!("menu_main_autotune").into_owned();
        let settings = rust_i18n::t!("menu_main_settings").into_owned();

        match self.active_screen {
            ActiveScreen::Main => vec![root],
            #[cfg(target_os = "windows")]
            ActiveScreen::DefenderSubmenu => vec![root, rust_i18n::t!("menu_main_defender").into_owned()],
            ActiveScreen::StrategySubmenu => vec![root, rust_i18n::t!("menu_main_strategy").into_owned()],
            ActiveScreen::DownloadDepsSubmenu => vec![root, downloader],
            ActiveScreen::DownloadZapretSubmenu => {
                vec![root, downloader, rust_i18n::t!("menu_dl_zapret").into_owned()]
            }
            ActiveScreen::DownloadStrategiesSubmenu => {
                vec![root, downloader, rust_i18n::t!("menu_dl_strat").into_owned()]
            }
            ActiveScreen::ZapretTagSelect => vec![
                root,
                downloader,
                rust_i18n::t!("menu_dl_zapret").into_owned(),
                rust_i18n::t!("menu_subdl_tag").into_owned(),
            ],
            ActiveScreen::StrategyTagSelect => vec![
                root,
                downloader,
                rust_i18n::t!("menu_dl_strat").into_owned(),
                rust_i18n::t!("menu_subdl_tag").into_owned(),
            ],
            ActiveScreen::GamefilterSubmenu => vec![root, rust_i18n::t!("menu_main_gamefilter").into_owned()],
            ActiveScreen::FakesSubmenu => vec![root, rust_i18n::t!("menu_main_fakes").into_owned()],
            ActiveScreen::FakesSelectSubmenu => vec![
                root,
                rust_i18n::t!("menu_main_fakes").into_owned(),
                rust_i18n::t!("menu_fakes_select_title").into_owned(),
            ],
            ActiveScreen::ServiceSubmenu => vec![root, rust_i18n::t!("menu_main_service").into_owned()],
            ActiveScreen::ServiceConflictSubmenu => vec![
                root,
                rust_i18n::t!("menu_main_service").into_owned(),
                rust_i18n::t!("srv_conflict_crumb").into_owned(),
            ],
            ActiveScreen::ListsEditorSubmenu => vec![root, rust_i18n::t!("menu_main_lists").into_owned()],
            ActiveScreen::StrategyEditorSubmenu => {
                vec![root, rust_i18n::t!("menu_main_strategy_editor").into_owned()]
            }
            ActiveScreen::AutotuneSubmenu => vec![root, autotune],
            ActiveScreen::AutotuneEditDomainsSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_edit_domains").into_owned()]
            }
            ActiveScreen::AutotuneProtocolsSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_protocols").into_owned()]
            }
            ActiveScreen::AutotuneBlockChecksSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_blockchecks").into_owned()]
            }
            ActiveScreen::AutotunePresetSelectionSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_domains").into_owned()]
            }
            ActiveScreen::AutotuneStrategiesSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_strategies").into_owned()]
            }
            ActiveScreen::AutotuneZ2PresetsSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_z2_presets").into_owned()]
            }
            ActiveScreen::AutotuneZ2TargetsSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_z2_targets").into_owned()]
            }
            ActiveScreen::AutotuneResultsSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_results").into_owned()]
            }
            ActiveScreen::AutotuneServiceMatrixSubmenu => {
                vec![root, autotune, rust_i18n::t!("menu_autotune_matrix").into_owned()]
            }
            ActiveScreen::SettingsSubmenu => vec![root, settings],
            ActiveScreen::SettingsEditorSubmenu => {
                vec![root, settings, rust_i18n::t!("settings_editor").into_owned()]
            }
            ActiveScreen::LogViewer => vec![root, settings, rust_i18n::t!("settings_logs").into_owned()],
        }
    }

    pub fn help_text(&self) -> String {
        if let Some(ref msg) = self.status_message {
            return msg.clone();
        }

        match self.active_screen {
            ActiveScreen::Main => match self.main_menu {
                #[cfg(target_os = "windows")]
                MainMenuState::DefenderSettings => rust_i18n::t!("help_def").into_owned(),
                MainMenuState::DownloadDeps => rust_i18n::t!("help_dl").into_owned(),
                MainMenuState::Engine => rust_i18n::t!("help_engine").into_owned(),
                MainMenuState::Interface => rust_i18n::t!("help_iface").into_owned(),
                MainMenuState::IpsetMode => rust_i18n::t!("help_ipset").into_owned(),
                MainMenuState::Strategy => rust_i18n::t!("help_strat").into_owned(),
                MainMenuState::GamefilterSettings => rust_i18n::t!("help_gf").into_owned(),
                #[cfg(target_os = "linux")]
                MainMenuState::BackendSettings => rust_i18n::t!("help_backend").into_owned(),
                MainMenuState::ServiceSettings => rust_i18n::t!("help_srv").into_owned(),
                MainMenuState::ListsEditor => rust_i18n::t!("help_lists").into_owned(),
                MainMenuState::StrategyEditor => rust_i18n::t!("help_strat_editor").into_owned(),
                MainMenuState::Autotune => rust_i18n::t!("help_autotune").into_owned(),
                MainMenuState::TtlAutopick => rust_i18n::t!("help_ttl").into_owned(),
                MainMenuState::FakesSettings => rust_i18n::t!("help_fakes").into_owned(),
                MainMenuState::Settings => rust_i18n::t!("help_settings").into_owned(),
                MainMenuState::Run => rust_i18n::t!("help_run").into_owned(),
                MainMenuState::Quit => rust_i18n::t!("help_quit").into_owned(),
            },
            ActiveScreen::DownloadDepsSubmenu => match self.download_deps_menu {
                DownloadDepsMenuState::ZapretDownloader => rust_i18n::t!("help_dl_zap").into_owned(),
                DownloadDepsMenuState::Zapret2Downloader => rust_i18n::t!("help_dl_zapret2").into_owned(),
                DownloadDepsMenuState::StrategiesDownloader => rust_i18n::t!("help_dl_str").into_owned(),
                DownloadDepsMenuState::Zapret2Strategies => rust_i18n::t!("help_dl_zapret2_strat").into_owned(),
                DownloadDepsMenuState::DownloadDefaults => rust_i18n::t!("help_dl_def").into_owned(),
                DownloadDepsMenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::DownloadZapretSubmenu => match self.download_zapret_menu {
                DownloadSubmenuState::Version => rust_i18n::t!("help_dl_ver").into_owned(),
                DownloadSubmenuState::SelectTag => rust_i18n::t!("help_dl_tag").into_owned(),
                DownloadSubmenuState::Start => rust_i18n::t!("help_dl_start").into_owned(),
                DownloadSubmenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::DownloadStrategiesSubmenu => match self.download_strategies_menu {
                DownloadSubmenuState::Version => rust_i18n::t!("help_dl_ver").into_owned(),
                DownloadSubmenuState::SelectTag => rust_i18n::t!("help_dl_tag").into_owned(),
                DownloadSubmenuState::Start => rust_i18n::t!("help_dl_start").into_owned(),
                DownloadSubmenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::GamefilterSubmenu => match self.gamefilter_menu {
                GamefilterMenuState::Tcp => rust_i18n::t!("help_gf_tcp").into_owned(),
                GamefilterMenuState::Udp => rust_i18n::t!("help_gf_udp").into_owned(),
                GamefilterMenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::FakesSubmenu => match self.fakes_menu {
                FakesMenuState::DiscordUdp | FakesMenuState::GameUdp => rust_i18n::t!("help_fakes_sel").into_owned(),
                FakesMenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::FakesSelectSubmenu => rust_i18n::t!("help_fakes_select").into_owned(),
            #[cfg(target_os = "windows")]
            ActiveScreen::DefenderSubmenu => rust_i18n::t!("help_def_sel").into_owned(),
            ActiveScreen::StrategySubmenu => rust_i18n::t!("help_strat_sel").into_owned(),
            ActiveScreen::ZapretTagSelect => rust_i18n::t!("help_tag_sel").into_owned(),
            ActiveScreen::StrategyTagSelect => rust_i18n::t!("help_tag_sel").into_owned(),
            ActiveScreen::ServiceSubmenu => rust_i18n::t!("help_srv_sel").into_owned(),
            ActiveScreen::ServiceConflictSubmenu => rust_i18n::t!("help_srv_conflict").into_owned(),
            ActiveScreen::ListsEditorSubmenu => rust_i18n::t!("help_lists_edit").into_owned(),
            ActiveScreen::StrategyEditorSubmenu => {
                if self.strategy_editor_delete_confirm {
                    rust_i18n::t!("help_strat_editor_delete_confirm").into_owned()
                } else if self.strategy_editor_new_name_editing {
                    rust_i18n::t!("help_strat_editor_new_name").into_owned()
                } else {
                    rust_i18n::t!("help_strat_editor_actions").into_owned()
                }
            }
            ActiveScreen::AutotuneSubmenu => match self.autotune_menu {
                AutotuneMenuState::PresetSelection => rust_i18n::t!("help_autotune_domains").into_owned(),
                AutotuneMenuState::Z2Bundle => rust_i18n::t!("help_autotune_z2_bundle").into_owned(),
                AutotuneMenuState::Z2Presets => rust_i18n::t!("help_autotune_z2_presets").into_owned(),
                AutotuneMenuState::Z2Targets => rust_i18n::t!("help_autotune_z2_targets").into_owned(),
                AutotuneMenuState::NumRequests => rust_i18n::t!("help_autotune_req").into_owned(),
                AutotuneMenuState::Strategies => rust_i18n::t!("help_autotune_strat_sel").into_owned(),
                AutotuneMenuState::Protocols => rust_i18n::t!("help_autotune_proto").into_owned(),
                AutotuneMenuState::BlockChecks => rust_i18n::t!("help_autotune_blockchecks").into_owned(),
                AutotuneMenuState::EditDomains => rust_i18n::t!("help_autotune_edit_domains").into_owned(),
                AutotuneMenuState::ServiceMatrix => rust_i18n::t!("help_autotune_matrix").into_owned(),
                AutotuneMenuState::Results => rust_i18n::t!("help_autotune_results_sel").into_owned(),
                AutotuneMenuState::Run => rust_i18n::t!("help_autotune_run").into_owned(),
                AutotuneMenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::AutotuneProtocolsSubmenu => match self.autotune_protocols_menu {
                AutotuneProtocolsState::Back => rust_i18n::t!("help_back").into_owned(),
                _ => rust_i18n::t!("help_autotune_toggle").into_owned(),
            },
            ActiveScreen::AutotuneBlockChecksSubmenu => match self.autotune_block_checks_menu {
                AutotuneBlockChecksState::Back => rust_i18n::t!("help_back").into_owned(),
                _ => rust_i18n::t!("help_autotune_toggle").into_owned(),
            },
            ActiveScreen::AutotuneEditDomainsSubmenu => rust_i18n::t!("help_autotune_edit_domains").into_owned(),
            ActiveScreen::AutotunePresetSelectionSubmenu => rust_i18n::t!("help_autotune_presets").into_owned(),
            ActiveScreen::AutotuneStrategiesSubmenu => rust_i18n::t!("help_autotune_strat").into_owned(),
            ActiveScreen::AutotuneZ2PresetsSubmenu => rust_i18n::t!("help_autotune_z2_presets").into_owned(),
            ActiveScreen::AutotuneZ2TargetsSubmenu => rust_i18n::t!("help_autotune_z2_targets").into_owned(),
            ActiveScreen::AutotuneResultsSubmenu => rust_i18n::t!("help_autotune_results").into_owned(),
            ActiveScreen::AutotuneServiceMatrixSubmenu => rust_i18n::t!("help_autotune_matrix_view").into_owned(),
            ActiveScreen::SettingsSubmenu => match self.settings_menu {
                SettingsMenuState::Editor => rust_i18n::t!("help_settings_editor").into_owned(),
                SettingsMenuState::BackupLists => rust_i18n::t!("help_settings_backup").into_owned(),
                SettingsMenuState::ViewLogs => rust_i18n::t!("help_settings_logs").into_owned(),
                SettingsMenuState::Back => rust_i18n::t!("help_back").into_owned(),
            },
            ActiveScreen::SettingsEditorSubmenu => {
                if self.editor_custom_editing {
                    rust_i18n::t!("help_settings_editor_input").into_owned()
                } else {
                    rust_i18n::t!("help_settings_editor_sel").into_owned()
                }
            }
            ActiveScreen::LogViewer => rust_i18n::t!("help_settings_logs_view").into_owned(),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod nav_tests {
    use super::MainMenuState;
    use crate::config::ZapretEngine;

    const VISUAL_ORDER: &[MainMenuState] = &[
        MainMenuState::DownloadDeps,
        MainMenuState::Engine,
        MainMenuState::Interface,
        MainMenuState::Strategy,
        MainMenuState::GamefilterSettings,
        MainMenuState::BackendSettings,
        MainMenuState::IpsetMode,
        MainMenuState::TtlAutopick,
        MainMenuState::ListsEditor,
        MainMenuState::StrategyEditor,
        MainMenuState::Autotune,
        MainMenuState::FakesSettings,
        MainMenuState::Settings,
        MainMenuState::ServiceSettings,
        MainMenuState::Run,
        MainMenuState::Quit,
    ];

    #[test]
    fn navigation_follows_render_order() {
        for pair in VISUAL_ORDER.windows(2) {
            assert_eq!(pair[0].next(), pair[1]);
            assert_eq!(pair[1].prev(), pair[0]);
        }
        assert_eq!(MainMenuState::Quit.next(), MainMenuState::DownloadDeps);
        assert_eq!(MainMenuState::DownloadDeps.prev(), MainMenuState::Quit);
    }

    #[test]
    fn ttl_autopick_follows_ipset_mode() {
        assert_eq!(MainMenuState::IpsetMode.next(), MainMenuState::TtlAutopick);
        assert_eq!(MainMenuState::TtlAutopick.next(), MainMenuState::ListsEditor);
        assert_eq!(MainMenuState::Autotune.next(), MainMenuState::FakesSettings);
        assert_eq!(MainMenuState::FakesSettings.prev(), MainMenuState::Autotune);
        assert_eq!(MainMenuState::ListsEditor.prev(), MainMenuState::TtlAutopick);
        assert_eq!(MainMenuState::TtlAutopick.prev(), MainMenuState::IpsetMode);
    }

    #[test]
    fn game_filter_is_skipped_for_zapret2_only() {
        let z1 = ZapretEngine::Zapret1;
        let z2 = ZapretEngine::Zapret2;
        assert_eq!(MainMenuState::Strategy.next_visible(&z1), MainMenuState::GamefilterSettings);
        assert_eq!(MainMenuState::Strategy.next_visible(&z2), MainMenuState::BackendSettings);
        assert_eq!(MainMenuState::BackendSettings.prev_visible(&z1), MainMenuState::GamefilterSettings);
        assert_eq!(MainMenuState::BackendSettings.prev_visible(&z2), MainMenuState::Strategy);
    }
}
