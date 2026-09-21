#![cfg(target_os = "linux")]

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const CACHE_DIR_FLAGS: &[&str] = &["-d", "--cache-dir"];

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

    let _err1 = Command::new("pkexec")
        .arg(&exe)
        .args(&args)
        .current_dir(&work_dir)
        .exec();

    let err2 = Command::new("sudo")
        .arg(&exe)
        .args(&args)
        .current_dir(&work_dir)
        .exec();

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
