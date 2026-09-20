#![cfg(target_os = "linux")]

use std::os::unix::process::CommandExt;

pub fn ensure_admin() {
    let not_root = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() != "0")
        .unwrap_or(true);

    if not_root {
        println!("{}", rust_i18n::t!("root_req"));

        let _err1 = std::process::Command::new("pkexec")
            .arg(std::env::current_exe().unwrap_or_default())
            .args(std::env::args().skip(1))
            .exec();

        let err2 = std::process::Command::new("sudo")
            .arg(std::env::current_exe().unwrap_or_default())
            .args(std::env::args().skip(1))
            .exec();

        eprintln!("{} ({})", rust_i18n::t!("root_err_sudo"), err2);
        std::process::exit(1);
    }
}

pub fn is_nfqws_running() -> bool {
    ["nfqws", "nfqws2"].iter().any(|name| {
        std::process::Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}
