use crate::firewalls::FirewallBackend;
use std::io::Write;
use std::time::Duration;

pub const TTL_MIN: u8 = 1;
pub const TTL_MAX: u8 = 20;

const TEST_DOMAINS: &[&str] = &[
    "discord.com",
    "youtube.com",
    "cdn.discordapp.com",
    "googlevideo.com",
    "discord.media",
];
const EXTRA_DOMAINS_FILE: &str = "ttl_domains.txt";

pub fn ttl_domains_file_path() -> std::path::PathBuf {
    crate::config::get_cache_dir().join(EXTRA_DOMAINS_FILE)
}

pub fn ensure_ttl_file() -> Result<(), String> {
    let path = ttl_domains_file_path();
    if path.exists() && !crate::autotune::load_domain_file(&path).is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory '{}': {}", parent.display(), e))?;
    }
    let mut content = format!("# {}\n", rust_i18n::t!("domain_file_header_ttl"));
    for d in TEST_DOMAINS {
        content.push_str(d);
        content.push('\n');
    }
    std::fs::write(&path, content).map_err(|e| format!("Cannot write '{}': {}", path.display(), e))
}

fn get_test_domains() -> Vec<String> {
    let from_file = crate::autotune::load_domain_file(&ttl_domains_file_path());
    if !from_file.is_empty() {
        return from_file;
    }
    TEST_DOMAINS.iter().map(|s| s.to_string()).collect()
}

fn null_device() -> &'static str {
    if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    }
}

fn curl_tls_ok(domain: &str) -> bool {
    std::process::Command::new("curl")
        .arg("-s")
        .arg("-k")
        .args([
            "--tlsv1.3",
            "--connect-timeout",
            "3",
            "--max-time",
            "3",
            "-o",
            null_device(),
        ])
        .arg(format!("https://{}", domain))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn wait_for_nfqws(timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    let mut running = false;
    while std::time::Instant::now() < deadline {
        if crate::runner::nfqws_process_running() || crate::platform::is_nfqws_running() {
            running = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if running {
        std::thread::sleep(Duration::from_millis(500));
    }
    running
}

pub fn autopick_ttl(strategy_file: &str, interface: &str, backend: &dyn FirewallBackend) -> Result<u8, String> {
    if crate::platform::is_nfqws_running() {
        return Err(rust_i18n::t!("ttl_err_running").into_owned());
    }

    for ttl in TTL_MIN..=TTL_MAX {
        println!("{} {}", rust_i18n::t!("ttl_testing"), ttl);
        let _ = std::io::stdout().flush();

        if let Err(e) = crate::runner::run_zapret_silent_ttl(strategy_file, interface, false, false, backend, ttl) {
            println!("  ❌ {}", e);
            continue;
        }

        if !wait_for_nfqws(Duration::from_secs(3)) {
            println!("  {}", rust_i18n::t!("ttl_nfqws_failed"));
            crate::runner::stop_zapret(backend);
            continue;
        }

        let mut all_ok = true;
        let domains = get_test_domains();
        for domain in &domains {
            let ok = curl_tls_ok(domain);
            println!("  {} {}", domain, if ok { "✅" } else { "❌" });
            let _ = std::io::stdout().flush();
            if !ok {
                all_ok = false;
            }
        }

        crate::runner::stop_zapret(backend);

        if all_ok {
            return Ok(ttl);
        }
    }

    Err(rust_i18n::t!("ttl_err_none").into_owned())
}
