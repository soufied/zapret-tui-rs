use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const CACHE_DIR_FLAGS: &[&str] = &["-d", "--cache-dir"];

const PRESERVED_SESSION_KEYS: &[&str] = &[
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "QT_QPA_PLATFORM",
    "HOME",
    "SUDO_USER",
];

fn preserved_path_entry() -> Option<(String, String)> {
    let current = std::env::var("PATH").unwrap_or_default();
    let mut extra_dirs: Vec<String> = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        extra_dirs.push(home.join(".local/bin").to_string_lossy().into_owned());
        extra_dirs.push(
            home.join(".local/share/flatpak/exports/bin")
                .to_string_lossy()
                .into_owned(),
        );
    }
    extra_dirs.push("/snap/bin".to_string());
    extra_dirs.push("/var/lib/flatpak/exports/bin".to_string());

    let mut combined = current;
    for dir in extra_dirs {
        if !combined.split(':').any(|part| part == dir) {
            if !combined.is_empty() {
                combined.push(':');
            }
            combined.push_str(&dir);
        }
    }

    if combined.trim().is_empty() {
        None
    } else {
        Some(("PATH".to_string(), combined))
    }
}

fn preserved_session_environment() -> Vec<(String, String)> {
    let mut preserved = Vec::new();
    for key in PRESERVED_SESSION_KEYS {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                preserved.push(((*key).to_string(), value));
            }
        }
    }
    if let Some(path_entry) = preserved_path_entry() {
        preserved.push(path_entry);
    }
    preserved
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false)
}

fn current_exe_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("zapret-rust"));
    std::fs::canonicalize(&exe).unwrap_or(exe)
}

fn has_cache_dir_flag(args: &[String]) -> bool {
    args.iter().any(|arg| {
        CACHE_DIR_FLAGS.contains(&arg.as_str()) || arg.starts_with("--cache-dir=") || arg.starts_with("-d=")
    })
}

fn elevated_args() -> Vec<String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    if !has_cache_dir_flag(&args) {
        args.push("--cache-dir".to_string());
        args.push(crate::config::get_cache_dir().to_string_lossy().into_owned());
    }

    args
}

pub fn ensure_admin() {
    if is_root() {
        return;
    }

    println!("{}", rust_i18n::t!("root_req"));

    let exe = current_exe_path();
    let work_dir = exe
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/"));
    let args = elevated_args();
    let session_env = preserved_session_environment();

    if crate::utils::find_in_path("pkexec").is_some() {
        let mut pkexec_args: Vec<String> = Vec::new();
        if crate::utils::find_in_path("env").is_some() && !session_env.is_empty() {
            pkexec_args.push("env".to_string());
            for (key, value) in &session_env {
                pkexec_args.push(format!("{}={}", key, value));
            }
        }
        pkexec_args.push(exe.to_string_lossy().into_owned());
        pkexec_args.extend(args.iter().cloned());

        let _err1 = Command::new("pkexec")
            .args(&pkexec_args)
            .current_dir(&work_dir)
            .exec();
    }

    let mut sudo_command = Command::new("sudo");
    sudo_command.current_dir(&work_dir);
    if !session_env.is_empty() {
        sudo_command.arg("env");
        for (key, value) in &session_env {
            sudo_command.arg(format!("{}={}", key, value));
        }
    }
    sudo_command.arg(&exe).args(&args);

    let err2 = sudo_command.exec();

    eprintln!("{} ({})", rust_i18n::t!("root_err_sudo"), err2);
    std::process::exit(1);
}

pub fn is_nfqws_running() -> bool {
    ["nfqws", "nfqws2"].iter().any(|name| {
        Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}
