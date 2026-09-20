use crate::config::ZapretEngine;
use crate::firewalls::FirewallBackend;
use crate::strategy::{self, GameFilterPorts};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static NFQWS_PROCESSES: Mutex<Vec<Child>> = Mutex::new(Vec::new());
static ENGINE_OVERRIDE: Mutex<Option<ZapretEngine>> = Mutex::new(None);

const BIND_SETTLE_MS: u64 = 700;

pub struct FirewallGuard<'a> {
    backend: &'a dyn FirewallBackend,
    armed: bool,
}

impl<'a> FirewallGuard<'a> {
    pub fn new(backend: &'a dyn FirewallBackend) -> Self {
        Self { backend, armed: true }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for FirewallGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.backend.clear();
        }
    }
}

pub fn set_engine(engine: ZapretEngine) {
    if let Ok(mut guard) = ENGINE_OVERRIDE.lock() {
        *guard = Some(engine);
    }
}

pub fn active_engine() -> ZapretEngine {
    if let Ok(guard) = ENGINE_OVERRIDE.lock() {
        if let Some(ref engine) = *guard {
            return engine.clone();
        }
    }
    crate::config::load_engine()
}

pub fn nfqws_process_running() -> bool {
    let Ok(mut procs) = NFQWS_PROCESSES.lock() else {
        return false;
    };
    procs
        .iter_mut()
        .any(|p| p.try_wait().map(|s| s.is_none()).unwrap_or(false))
}

#[cfg(target_os = "linux")]
fn kill_stale_zapret() {
    for name in ["nfqws", "nfqws2"] {
        let _ = Command::new("pkill")
            .arg("-9")
            .arg("-x")
            .arg(name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[cfg(target_os = "windows")]
fn kill_stale_zapret() {
    for name in ["winws.exe", "winws2.exe"] {
        let _ = Command::new("taskkill").args(["/F", "/IM", name]).output();
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn kill_stale_zapret() {}

fn workspace_path(engine: &ZapretEngine) -> PathBuf {
    match engine {
        ZapretEngine::Zapret1 => crate::strategy::repo_dir(),
        ZapretEngine::Zapret2 => engine.workspace_dir(),
    }
}

fn bin_path(engine: &ZapretEngine) -> PathBuf {
    engine.binary_path()
}

fn strategy_file_path(engine: &ZapretEngine, workspace: &Path, strategy_file: &str) -> PathBuf {
    let direct = PathBuf::from(strategy_file);
    if direct.is_absolute() && direct.exists() {
        return direct;
    }

    match engine {
        ZapretEngine::Zapret2 => {
            let preset = engine.presets_dir().join(strategy_file);
            if preset.exists() {
                preset
            } else {
                direct
            }
        }
        ZapretEngine::Zapret1 => {
            let cache_custom = crate::config::get_cache_dir()
                .join("custom-strategies")
                .join(strategy_file);
            if cache_custom.exists() {
                return cache_custom;
            }
            let repo_custom = workspace.join("custom-strategies").join(strategy_file);
            if repo_custom.exists() {
                repo_custom
            } else {
                workspace.join(strategy_file)
            }
        }
    }
}

fn game_filter(use_tcp: bool, use_udp: bool) -> Option<GameFilterPorts> {
    if use_tcp || use_udp {
        Some(GameFilterPorts {
            ports: "1024-65535".to_string(),
            tcp_ports: "1024-65535".to_string(),
            udp_ports: "1024-65535".to_string(),
        })
    } else {
        None
    }
}

fn ensure_user_lists(workspace: &Path) {
    let lists_dir = workspace.join("lists");
    if !lists_dir.exists() {
        return;
    }
    for name in &[
        "list-general-user.txt",
        "list-exclude-user.txt",
        "ipset-exclude-user.txt",
    ] {
        let path = lists_dir.join(name);
        if !path.exists() {
            let _ = fs::write(&path, "");
        }
    }
}

fn ensure_referenced_lists(launch: &Launch) -> Vec<PathBuf> {
    if launch.engine != ZapretEngine::Zapret2 {
        return Vec::new();
    }

    let Ok(root) = fs::canonicalize(&launch.workspace) else {
        return Vec::new();
    };

    let mut created: Vec<PathBuf> = Vec::new();
    for reference in strategy::collect_list_refs(&launch.args) {
        let Some(path) = strategy::resolve_within(&root, &reference.path) else {
            continue;
        };
        if strategy::list_file_present(&path) {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = strategy::placeholder_content(reference.kind, &file_name);
        if fs::write(&path, content).is_ok() {
            created.push(path);
        }
    }
    created
}

fn build_zapret1_args(parsed: &strategy::ParsedStrategy, ttl: Option<u8>) -> Vec<String> {
    #[cfg(target_os = "linux")]
    let mut args = vec!["--dpi-desync-fwmark=0x40000000".to_string(), "--qnum=200".to_string()];

    #[cfg(not(target_os = "linux"))]
    let mut args = vec![
        format!("--wf-tcp={}", parsed.tcp_ports),
        format!("--wf-udp={}", parsed.udp_ports),
    ];

    for param in &parsed.nfqws_params {
        for p in param.split_whitespace() {
            let p = p.replace('"', "");
            if p.is_empty() || p == "^" {
                continue;
            }
            if ttl.is_some() && (p.starts_with("--dpi-desync-ttl") || p.starts_with("--dpi-desync-autottl")) {
                continue;
            }
            args.push(p.to_string());
        }
        if let Some(ttl) = ttl {
            args.push(format!("--dpi-desync-ttl={}", ttl));
            args.push(format!("--dpi-desync-ttl6={}", ttl));
            args.push("--dpi-desync-autottl=-".to_string());
            args.push("--dpi-desync-autottl6=-".to_string());
        }
    }
    args
}

fn first_error_line(raw: &str) -> String {
    raw.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .next_back()
        .unwrap_or("no diagnostic output")
        .to_string()
}

fn validate_launch(launch: &Launch) -> Result<(), String> {
    if launch.engine != ZapretEngine::Zapret2 {
        return Ok(());
    }

    let output = Command::new(&launch.bin)
        .arg("--dry-run")
        .args(&launch.args)
        .current_dir(&launch.workspace)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run {} --dry-run: {}", launch.engine.binary_name(), e))?;

    if output.status.success() {
        return Ok(());
    }

    let mut diagnostic = String::from_utf8_lossy(&output.stderr).into_owned();
    if diagnostic.trim().is_empty() {
        diagnostic = String::from_utf8_lossy(&output.stdout).into_owned();
    }

    Err(format!(
        "{} rejected the preset arguments: {}",
        launch.engine.binary_name(),
        first_error_line(&diagnostic)
    ))
}

fn confirm_bound(child: &mut Child, engine: &ZapretEngine) -> Result<(), String> {
    thread::sleep(Duration::from_millis(BIND_SETTLE_MS));

    match child.try_wait() {
        Ok(Some(status)) => Err(format!(
            "{} exited before binding NFQUEUE {} ({})",
            engine.binary_name(),
            crate::firewalls::NFQUEUE_NUM,
            status
        )),
        Ok(None) => Ok(()),
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!("cannot determine {} state: {}", engine.binary_name(), e))
        }
    }
}

fn set_cap(bin_path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("setcap")
            .args(["cap_net_admin+ep", &bin_path.to_string_lossy()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = bin_path;
        true
    }
}

pub struct Launch {
    pub engine: ZapretEngine,
    pub bin: PathBuf,
    pub workspace: PathBuf,
    pub tcp_ports: String,
    pub udp_ports: String,
    pub args: Vec<String>,
    pub log_params: Vec<String>,
}

pub fn prepare(
    engine: &ZapretEngine,
    strategy_file: &str,
    use_tcp: bool,
    use_udp: bool,
    ttl: Option<u8>,
) -> Result<Launch, String> {
    let (use_tcp, use_udp) = if engine.supports_game_filter() {
        (use_tcp, use_udp)
    } else {
        (false, false)
    };
    let workspace = workspace_path(engine);
    let path = strategy_file_path(engine, &workspace, strategy_file);
    let path_str = path.to_str().ok_or("invalid strategy path")?;

    let (tcp_ports, udp_ports, args, log_params) = match engine {
        ZapretEngine::Zapret1 => {
            let parsed = strategy::parse_bat_file(path_str, game_filter(use_tcp, use_udp).as_ref())
                .map_err(|e| format!("parse error: {}", e))?;
            let args = build_zapret1_args(&parsed, ttl);
            (
                parsed.tcp_ports.clone(),
                parsed.udp_ports.clone(),
                args,
                parsed.nfqws_params.clone(),
            )
        }
        ZapretEngine::Zapret2 => {
            let parsed = strategy::parse_zapret2_preset(path_str, ttl).map_err(|e| format!("parse error: {}", e))?;
            let log_params = parsed.args.clone();
            (parsed.tcp_ports, parsed.udp_ports, parsed.args, log_params)
        }
    };

    Ok(Launch {
        engine: engine.clone(),
        bin: bin_path(engine),
        workspace,
        tcp_ports,
        udp_ports,
        args,
        log_params,
    })
}

fn log_file_path() -> PathBuf {
    crate::config::get_cache_dir().join("logs").join("zapret.log")
}

pub fn run_zapret(strategy_file: &str, interface: &str, use_tcp: bool, use_udp: bool, backend: &dyn FirewallBackend) {
    let engine = active_engine();
    let mut term: Vec<String> = Vec::new();
    let ttl = crate::config::load_ttl();

    let launch = match prepare(&engine, strategy_file, use_tcp, use_udp, ttl) {
        Ok(l) => l,
        Err(e) => {
            println!("{}{}", rust_i18n::t!("err_parse_strat"), e);
            return;
        }
    };

    if !launch.bin.exists() {
        let msg = rust_i18n::t!("err_bin_miss").replace("{:?}", &format!("{:?}", launch.bin));
        println!("{}", msg);
        return;
    }

    if !set_cap(&launch.bin) {
        let msg = rust_i18n::t!("err_setcap").to_string();
        term.push(msg.clone());
        println!("{}", msg);
    }

    ensure_user_lists(&launch.workspace);

    for path in ensure_referenced_lists(&launch) {
        let msg = format!("{}{}", rust_i18n::t!("msg_placeholder_created"), path.display());
        term.push(msg.clone());
        println!("{}", msg);
    }

    if let Err(e) = validate_launch(&launch) {
        println!("{}{}", rust_i18n::t!("err_start_nfqws"), e);
        return;
    }

    if let Err(e) = backend.setup(&launch.tcp_ports, &launch.udp_ports, interface) {
        println!("{}{}", rust_i18n::t!("msg_err_firewall"), e);
        let _ = backend.clear();
        return;
    }

    let mut guard = FirewallGuard::new(backend);

    kill_stale_zapret();

    let msg = rust_i18n::t!("msg_start_nfqws").to_string();
    term.push(msg.clone());
    println!("{}", msg);

    let cmd_msg = format!("{}{:?} {:?}", rust_i18n::t!("msg_cmd"), launch.bin, launch.args);
    term.push(cmd_msg.clone());
    println!("{}", cmd_msg);

    crate::logger::log_nfqws_launch(&launch.bin.to_string_lossy(), &launch.log_params, &term);

    let log_path = log_file_path();
    let _ = fs::create_dir_all(log_path.parent().unwrap());
    let offset = fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);

    let output_file = match fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => f,
        Err(e) => {
            println!("failed to open log file: {}", e);
            return;
        }
    };
    let out_dup = match output_file.try_clone() {
        Ok(f) => f,
        Err(e) => {
            println!("failed to clone log file handle: {}", e);
            return;
        }
    };

    let mut child = match Command::new(&launch.bin)
        .args(&launch.args)
        .current_dir(&launch.workspace)
        .stdin(Stdio::null())
        .stdout(output_file)
        .stderr(out_dup)
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            println!("{}{}", rust_i18n::t!("err_start_nfqws"), e);
            return;
        }
    };

    if let Err(e) = confirm_bound(&mut child, &launch.engine) {
        println!("{}{}", rust_i18n::t!("err_start_nfqws"), e);
        let content = fs::read_to_string(&log_path).unwrap_or_default();
        let tail = content.get(offset as usize..).unwrap_or("").to_string();
        if !tail.trim().is_empty() {
            print!("{}", tail);
        }
        return;
    }

    if let Ok(mut procs) = NFQWS_PROCESSES.lock() {
        procs.push(child);
    }

    guard.disarm();

    println!("{}", rust_i18n::t!("msg_nfqws_run"));

    let content = fs::read_to_string(&log_path).unwrap_or_default();
    let tail = content.get(offset as usize..).unwrap_or("").to_string();
    if !tail.trim().is_empty() {
        print!("{}", tail);
    }
}

pub fn run_zapret_silent(
    strategy_file: &str,
    interface: &str,
    use_tcp: bool,
    use_udp: bool,
    backend: &dyn FirewallBackend,
) -> Result<(), String> {
    let ttl = crate::config::load_ttl();
    run_zapret_silent_impl(strategy_file, interface, use_tcp, use_udp, backend, ttl)
}

pub fn run_zapret_silent_ttl(
    strategy_file: &str,
    interface: &str,
    use_tcp: bool,
    use_udp: bool,
    backend: &dyn FirewallBackend,
    ttl: u8,
) -> Result<(), String> {
    run_zapret_silent_impl(strategy_file, interface, use_tcp, use_udp, backend, Some(ttl))
}

fn run_zapret_silent_impl(
    strategy_file: &str,
    interface: &str,
    use_tcp: bool,
    use_udp: bool,
    backend: &dyn FirewallBackend,
    ttl: Option<u8>,
) -> Result<(), String> {
    let engine = active_engine();
    let launch = prepare(&engine, strategy_file, use_tcp, use_udp, ttl)?;

    if !launch.bin.exists() {
        return Err(format!("binary not found: {:?}", launch.bin));
    }

    let _ = set_cap(&launch.bin);
    ensure_user_lists(&launch.workspace);
    let _ = ensure_referenced_lists(&launch);
    validate_launch(&launch)?;

    kill_stale_zapret();

    backend
        .setup(&launch.tcp_ports, &launch.udp_ports, interface)
        .map_err(|e| format!("firewall setup error: {}", e))?;

    let mut guard = FirewallGuard::new(backend);

    crate::logger::log_nfqws_launch(&launch.bin.to_string_lossy(), &launch.log_params, &[]);

    let mut child = Command::new(&launch.bin)
        .args(&launch.args)
        .current_dir(&launch.workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start {}: {}", launch.engine.binary_name(), e))?;

    confirm_bound(&mut child, &launch.engine)?;

    if let Ok(mut procs) = NFQWS_PROCESSES.lock() {
        procs.push(child);
    }

    guard.disarm();
    Ok(())
}

pub fn stop_zapret(backend: &dyn FirewallBackend) {
    let mut term: Vec<String> = Vec::new();

    let msg = rust_i18n::t!("msg_zapret_stop").to_string();
    term.push(msg.clone());
    println!("{}", msg);

    if let Ok(mut procs) = NFQWS_PROCESSES.lock() {
        for child in procs.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        procs.clear();
    }

    let _ = backend.clear();

    let msg = rust_i18n::t!("msg_zapret_clear").to_string();
    term.push(msg.clone());
    println!("{}", msg);

    crate::logger::log_stop(&term);
}

pub fn stop_zapret_quiet(backend: &dyn FirewallBackend) {
    if let Ok(mut procs) = NFQWS_PROCESSES.lock() {
        for child in procs.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        procs.clear();
    }
    kill_stale_zapret();
    let _ = backend.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch_with(engine: ZapretEngine, workspace: PathBuf, args: &[&str]) -> Launch {
        Launch {
            engine,
            bin: PathBuf::from("nfqws2"),
            workspace,
            tcp_ports: String::new(),
            udp_ports: String::new(),
            args: args.iter().map(|s| s.to_string()).collect(),
            log_params: Vec::new(),
        }
    }

    fn temp_workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zapret_runner_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("lists")).unwrap();
        dir
    }

    #[test]
    fn creates_missing_lists_with_safe_content() {
        let ws = temp_workspace("create");
        let launch = launch_with(
            ZapretEngine::Zapret2,
            ws.clone(),
            &[
                "--hostlist=lists/whatsapp.txt",
                "--hostlist-exclude=lists/list-exclude.txt",
                "--ipset-exclude=lists/ipset-exclude.txt",
                "--ipset=lists/ipset-all.txt",
                "--ipset=lists/other-ips.txt",
            ],
        );

        let created = ensure_referenced_lists(&launch);
        assert_eq!(created.len(), 5);

        let host_placeholder = fs::read_to_string(ws.join("lists/whatsapp.txt")).unwrap();
        assert_eq!(host_placeholder, format!("{}\n", strategy::refs::PLACEHOLDER_HOST));
        assert!(fs::read_to_string(ws.join("lists/list-exclude.txt")).unwrap().is_empty());
        assert!(fs::read_to_string(ws.join("lists/ipset-exclude.txt")).unwrap().is_empty());
        assert!(fs::read_to_string(ws.join("lists/ipset-all.txt")).unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(ws.join("lists/other-ips.txt")).unwrap(),
            format!("{}\n", strategy::refs::PLACEHOLDER_IP)
        );

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn leaves_existing_lists_untouched() {
        let ws = temp_workspace("existing");
        fs::write(ws.join("lists/list-general.txt"), "example.com\n").unwrap();
        let launch = launch_with(ZapretEngine::Zapret2, ws.clone(), &["--hostlist=lists/list-general.txt"]);

        assert!(ensure_referenced_lists(&launch).is_empty());
        assert_eq!(
            fs::read_to_string(ws.join("lists/list-general.txt")).unwrap(),
            "example.com\n"
        );

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn never_writes_outside_the_workspace() {
        let ws = temp_workspace("escape");
        let outside_name = format!("outside_ref_{}.txt", std::process::id());
        let outside = ws.parent().unwrap().join(&outside_name);
        let escape_arg = format!("--hostlist=../{}", outside_name);
        let launch = launch_with(
            ZapretEngine::Zapret2,
            ws.clone(),
            &[escape_arg.as_str(), "--hostlist=/etc/zapret_rust_should_not_exist.txt"],
        );

        assert!(ensure_referenced_lists(&launch).is_empty());
        assert!(!outside.exists());
        assert!(!Path::new("/etc/zapret_rust_should_not_exist.txt").exists());

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn zapret1_launches_are_left_alone() {
        let ws = temp_workspace("z1");
        let launch = launch_with(ZapretEngine::Zapret1, ws.clone(), &["--hostlist=lists/whatsapp.txt"]);

        assert!(ensure_referenced_lists(&launch).is_empty());
        assert!(!ws.join("lists/whatsapp.txt").exists());

        let _ = fs::remove_dir_all(&ws);
    }
}
