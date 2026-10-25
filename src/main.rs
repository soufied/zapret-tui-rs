mod common;
mod diagnostics;
mod engine;
mod firewalls;
pub mod inits;
mod platform;
mod strategy;
mod tui;

rust_i18n::i18n!("assets/locales", fallback = "en");

use clap::Parser;
#[cfg(not(target_os = "windows"))]
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, Signal};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

static RUNNING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
use firewalls::LinuxBackend;

#[cfg(target_os = "windows")]
use firewalls::windivert::WinDivertBackend;

#[derive(Parser, Debug)]
#[command(
    name = "run",
    about = "Run zapret in foreground (useful for testing).",
    disable_help_flag = true
)]
struct Cli {
    #[arg(short = 'c', long = "config", help = "Load configuration from file")]
    config: Option<String>,

    #[arg(short = 's', long = "strategy", help = "Use specific strategy")]
    strategy: Option<String>,

    #[arg(
        short = 'i',
        long = "interface",
        default_value = "any",
        help = "Network interface (default: any)"
    )]
    interface: String,

    #[arg(long = "gamefiltertcp", short = 't', help = "Enable gamefiltertcp")]
    gamefiltertcp: bool,

    #[arg(long = "gamefilterudp", short = 'u', help = "Enable gamefilterudp")]
    gamefilterudp: bool,

    #[arg(
        long = "cache-dir",
        short = 'd',
        help = "Cache directory for downloaded dependencies and strategies"
    )]
    cache_dir: Option<String>,

    #[arg(
        short = 'R',
        long = "router",
        help = "Headless router mode (OpenWrt): no TUI, config from the router cache dir by default"
    )]
    router: bool,

    #[arg(long = "blockcheck", help = "Rank strategies against the blockcheck domains, then exit")]
    blockcheck: bool,

    #[arg(long = "bc-engine", help = "Blockcheck engine: zapret or zapret2 (default: zapret)")]
    bc_engine: Option<String>,

    #[arg(long = "bc-strategies", help = "Blockcheck: comma-separated strategy or preset names (default: all)")]
    bc_strategies: Option<String>,

    #[arg(long = "bc-limit", help = "Blockcheck: test only the first N strategies")]
    bc_limit: Option<usize>,

    #[arg(long = "bc-per-list", default_value_t = 5, help = "Blockcheck (zapret2): random hosts per hostlist, 1-20")]
    bc_per_list: usize,

    #[arg(long = "bc-services", help = "Blockcheck (zapret only): comma-separated service keys, e.g. yt,ds")]
    bc_services: Option<String>,

    #[arg(short = 'h', long = "help", help = "Show this help")]
    help: bool,
}

fn show_help() {
    println!("{}", rust_i18n::t!("cli_usage"));
    println!("{}", rust_i18n::t!("cli_desc"));
    println!("{}", rust_i18n::t!("cli_opts"));
    println!("{}", rust_i18n::t!("cli_opt_c"));
    println!("{}", rust_i18n::t!("cli_opt_s"));
    println!("{}", rust_i18n::t!("cli_opt_i"));
    println!("{}", rust_i18n::t!("cli_opt_t"));
    println!("{}", rust_i18n::t!("cli_opt_u"));
    println!("{}", rust_i18n::t!("cli_opt_d"));
    println!("{}", rust_i18n::t!("cli_opt_router"));
    println!("{}", rust_i18n::t!("cli_opt_h"));
    println!("{}", rust_i18n::t!("cli_modes"));
    println!("{}", rust_i18n::t!("cli_mode1"));
    println!("{}", rust_i18n::t!("cli_mode2"));
    println!("{}", rust_i18n::t!("cli_mode3"));
    println!("  --blockcheck           Rank strategies against the blockcheck domains, then exit");
    println!("    --bc-engine <name>     zapret (default) or zapret2");
    println!("    --bc-strategies <a,b>  test only these strategies or presets");
    println!("    --bc-limit <N>         test only the first N strategies");
    println!("    --bc-per-list <N>      zapret2: random hosts per hostlist (default 5)");
    println!("    --bc-services <a,b>    zapret only: service keys, e.g. yt,ds");
}

fn main() {
    let loc = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    if loc.to_lowercase().starts_with("ru") {
        rust_i18n::set_locale("ru");
    } else {
        rust_i18n::set_locale("en");
    }

    #[cfg(target_os = "windows")]
    {
        if std::env::args().any(|arg| arg == "--service") {
            if let Err(e) = inits::winservice::run_service() {
                eprintln!("{}{}", rust_i18n::t!("err_srv"), e);
                std::process::exit(1);
            }
            return;
        }
    }

    platform::setup_console();
    platform::ensure_admin();

    let args = Cli::parse();

    if let Some(ref d) = args.cache_dir {
        std::env::set_var("ZAPRET_CACHE_DIR", d);
    } else if args.router && std::env::var_os("ZAPRET_CACHE_DIR").is_none() {
        std::env::set_var("ZAPRET_CACHE_DIR", engine::router::DEFAULT_CACHE_DIR);
    }

    if let Err(e) = common::config::ensure_default_config() {
        println!("{}{}", rust_i18n::t!("msg_err"), e);
    }

    if let Err(e) = strategy::ensure_custom_strategies() {
        println!("{}{}", rust_i18n::t!("err_custom_strategies"), e);
    }

    if args.help {
        show_help();
        return;
    }

    if args.blockcheck {
        let per_list = args.bc_per_list;
        if !(1..=20).contains(&per_list) {
            eprintln!("--bc-per-list must be from 1 to 20");
            exit(1);
        }
        let engine = crate::common::config::ZapretEngine::from_str(args.bc_engine.as_deref().unwrap_or("zapret"));
        let opts = diagnostics::blockcheck::Options {
            engine,
            interface: args.interface.clone(),
            names: args.bc_strategies.as_deref().map(diagnostics::blockcheck::split_csv).unwrap_or_default(),
            limit: args.bc_limit.unwrap_or(0),
            per_list,
            services: args.bc_services.as_deref().map(diagnostics::blockcheck::split_csv).unwrap_or_default(),
            domains: Vec::new(),
        };
        exit(diagnostics::blockcheck::run_cli(opts));
    }

    let mut use_interface = args.interface.clone();
    let mut use_strategy = args.strategy.clone();
    let mut use_gamefilter_tcp = args.gamefiltertcp;
    let mut use_gamefilter_udp = args.gamefilterudp;
    #[cfg(target_os = "linux")]
    let mut use_backend: LinuxBackend = LinuxBackend::Nftables;
    let mut is_interactive = true;

    let config_file = match (&args.config, args.router) {
        (Some(path), _) => Some(path.clone()),
        (None, true) => Some(common::config::config_path().to_string_lossy().into_owned()),
        (None, false) => None,
    };

    if let Some(config_file) = &config_file {
        println!("{}{}", rust_i18n::t!("msg_load_cfg"), config_file);
        match common::config::load_config(config_file) {
            Ok(cfg) => {
                engine::runner::set_engine(cfg.engine.clone());
                std::env::set_var("REPO_DIR", cfg.engine.workspace_dir());
                use_interface = cfg.interface;
                use_strategy = Some(cfg.strategy);
                use_gamefilter_tcp = cfg.gamefilter_tcp;
                use_gamefilter_udp = cfg.gamefilter_udp;
                #[cfg(target_os = "linux")]
                {
                    use_backend = LinuxBackend::from_config(&cfg.backend);
                }
                is_interactive = false;
            }
            Err(e) => {
                println!("{}{}", rust_i18n::t!("err_load_cfg"), e);
                exit(1);
            }
        }
    } else if use_strategy.is_some() {
        is_interactive = false;
    }

    if args.router {
        is_interactive = false;
        if use_strategy.as_deref().map_or(true, str::is_empty) {
            eprintln!("{}", rust_i18n::t!("router_err_no_strategy"));
            exit(1);
        }
        engine::router::print_preflight();
    }

    #[cfg(not(target_os = "windows"))]
    {
        extern "C" fn handle_signal(_: i32) {
            RUNNING.store(false, Ordering::SeqCst);
        }
        let handler = SigHandler::Handler(handle_signal);
        let sig_action = SigAction::new(handler, SaFlags::empty(), signal::SigSet::empty());
        unsafe {
            let _ = signal::sigaction(Signal::SIGTERM, &sig_action);
            let _ = signal::sigaction(Signal::SIGINT, &sig_action);
        }
    }
    #[cfg(target_os = "windows")]
    {
        ctrlc::set_handler(move || {
            RUNNING.store(false, Ordering::SeqCst);
        })
        .unwrap_or_else(|e| eprintln!("{}{}", rust_i18n::t!("err_ctrl_c"), e));
    }

    let reader = tui::spawn_event_reader();

    loop {
        if is_interactive {
            let interfaces = common::config::get_interfaces();
            let strategies = strategy::get_strategies();
            let mut app = tui::AppState::new(interfaces, strategies);

            let res = tui::run_tui(&mut app, &reader);
            if let Err(e) = res {
                println!("{}{}", rust_i18n::t!("err_tui"), e);
                exit(1);
            }

            use_interface = app
                .interfaces
                .get(app.selected_interface)
                .unwrap_or(&"any".to_string())
                .to_string();
            use_strategy = app.strategies.get(app.selected_strategy).cloned();
            engine::runner::set_engine(app.engine.clone());
            use_gamefilter_tcp = app.tcp_gamefilter;
            use_gamefilter_udp = app.udp_gamefilter;
            #[cfg(target_os = "linux")]
            {
                use_backend = app.selected_backend;
            }

            if app.should_quit {
                println!("{}", rust_i18n::t!("msg_exited"));
                return;
            }
        }

        let strategy_file = match use_strategy {
            Some(ref s) => s.clone(),
            None => {
                println!("{}", rust_i18n::t!("msg_no_strat"));
                if is_interactive {
                    continue;
                } else {
                    exit(1);
                }
            }
        };

        let nfqws_ok = common::download::check_nfqws_installed();
        let strat_ok = common::download::check_strategies_installed();
        if !nfqws_ok || !strat_ok {
            if !nfqws_ok && !strat_ok {
                eprintln!("{}", rust_i18n::t!("msg_err_both_missing"));
            } else if !nfqws_ok {
                eprintln!("{}", rust_i18n::t!("msg_err_nfqws_missing"));
            } else {
                eprintln!("{}", rust_i18n::t!("msg_err_strat_missing"));
            }
            if is_interactive {
                thread::sleep(Duration::from_secs(2));
                continue;
            } else {
                exit(1);
            }
        }

        if !engine::runner::active_engine().supports_game_filter() {
            use_gamefilter_tcp = false;
            use_gamefilter_udp = false;
        }

        #[cfg(target_os = "linux")]
        let backend = use_backend;

        #[cfg(target_os = "windows")]
        let backend = WinDivertBackend;

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let backend = firewalls::UnsupportedBackend;

        #[cfg(target_os = "linux")]
        let backend_info = format!(", backend={}", use_backend.to_config());
        #[cfg(not(target_os = "linux"))]
        let backend_info = String::new();

        println!(
            "{}{}, interface={}, gamefiltertcp={}, gamefilterudp={}{}",
            rust_i18n::t!("msg_run_params"),
            strategy_file,
            use_interface,
            use_gamefilter_tcp,
            use_gamefilter_udp,
            backend_info
        );

        if let Err(e) = engine::runner::run_zapret(
            &strategy_file,
            &use_interface,
            use_gamefilter_tcp,
            use_gamefilter_udp,
            &backend,
        ) {
            eprintln!("{}", e);
            engine::runner::stop_zapret_quiet(&backend);
            if is_interactive {
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            exit(1);
        }

        println!("{}", rust_i18n::t!("msg_zapret_started"));

        RUNNING.store(true, Ordering::SeqCst);

        let mut daemon_lost = false;
        while RUNNING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
            if !engine::runner::nfqws_process_running() {
                daemon_lost = true;
                break;
            }
        }

        engine::runner::stop_zapret(&backend);

        if daemon_lost {
            eprintln!("{}{}", rust_i18n::t!("err_start_nfqws"), rust_i18n::t!("err_nfqws_vanished"));
            if is_interactive {
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            exit(1);
        }

        if !is_interactive {
            break;
        }
    }
}
