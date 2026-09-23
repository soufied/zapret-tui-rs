use std::env;
use std::path::PathBuf;
use std::process::Command;

#[link(name = "kernel32")]
extern "system" {
    fn SetConsoleCP(wCodePageID: u32) -> i32;
    fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
}

pub fn setup_console() {
    unsafe {
        SetConsoleCP(65001);
        SetConsoleOutputCP(65001);
    }
    let _ = crossterm::ansi_support::supports_ansi();
}

pub fn ensure_admin() {
    let elevated = is_elevated();

    if elevated && in_windows_terminal() {
        return;
    }

    let exe_path = env::current_exe().unwrap();
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if wt_available() {
        let verb = if elevated { "" } else { " -Verb RunAs" };
        let ps = format!(
            "Start-Process -FilePath 'wt'{} -ArgumentList @('-d', '\"{}\"', '\"{}\"')",
            verb,
            cwd.display(),
            exe_path.display()
        );
        if run_powershell(&ps) {
            std::process::exit(0);
        }
        if elevated {
            return;
        }
        eprintln!("Failed to elevate privileges. Please run as Administrator.");
        std::process::exit(1);
    }

    let ps = format!("Start-Process -FilePath \"{}\" -Verb RunAs", exe_path.display());
    if run_powershell(&ps) {
        std::process::exit(0);
    }

    eprintln!("Failed to elevate privileges. Please run as Administrator.");
    std::process::exit(1);
}

fn in_windows_terminal() -> bool {
    env::var("WT_SESSION").is_ok()
}

fn wt_available() -> bool {
    let on_path = Command::new("where.exe")
        .arg("wt")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    on_path
        || env::var_os("LOCALAPPDATA").map_or(false, |p| {
            PathBuf::from(p)
                .join("Microsoft")
                .join("WindowsApps")
                .join("wt.exe")
                .exists()
        })
}

fn run_powershell(script: &str) -> bool {
    Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn is_elevated_process() -> bool {
    is_elevated()
}

fn is_elevated() -> bool {
    Command::new("fsutil")
        .arg("dirty")
        .arg("query")
        .arg(env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string()))
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub fn is_nfqws_running() -> bool {
    ["winws.exe", "winws2.exe"].iter().any(|name| {
        let filter = format!("IMAGENAME eq {}", name);
        let out = Command::new("tasklist")
            .args(["/FI", &filter, "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        out.contains(name)
    })
}
