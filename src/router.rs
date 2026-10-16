//! Router mode: headless operation on OpenWrt and similar routers.
//!
//! In router mode the TUI is skipped, settings come from a config file
//! (by default in `DEFAULT_CACHE_DIR`), and the process is expected to run
//! under procd via the init script installed by `inits::procd`.

use crate::utils::find_in_path;
use std::fs;
use std::path::Path;

/// Cache and config directory used in router mode unless `-d` or
/// `ZAPRET_CACHE_DIR` override it. Lives on persistent storage so that
/// downloads survive reboots.
pub const DEFAULT_CACHE_DIR: &str = "/usr/share/zapret-rust";

const OPENWRT_RELEASE: &str = "/etc/openwrt_release";
const OS_RELEASE: &str = "/etc/os-release";
const FIREWALL_CONFIG: &str = "/etc/config/firewall";

/// Package manager available on the router. OpenWrt 25.x and newer use `apk`;
/// 24.10 and older use `opkg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apk,
    Opkg,
    Unknown,
}

/// True when running on OpenWrt. Checks the release file first, then falls
/// back to `os-release` for images that omit `openwrt_release`.
pub fn is_openwrt() -> bool {
    if Path::new(OPENWRT_RELEASE).exists() {
        return true;
    }
    fs::read_to_string(OS_RELEASE)
        .map(|content| content.lines().any(|line| line.starts_with("ID=openwrt") || line.starts_with("NAME=\"OpenWrt")))
        .unwrap_or(false)
}

/// Detects the package manager. `apk` is checked first because 25.x images
/// have no `opkg`.
pub fn package_manager() -> PackageManager {
    if find_in_path("apk").is_some() {
        PackageManager::Apk
    } else if find_in_path("opkg").is_some() {
        PackageManager::Opkg
    } else {
        PackageManager::Unknown
    }
}

/// Shell command that installs `packages` with the router's package manager,
/// or `None` when no supported package manager is found.
pub fn install_command(packages: &str) -> Option<String> {
    match package_manager() {
        PackageManager::Apk => Some(format!("apk add {}", packages)),
        PackageManager::Opkg => Some(format!("opkg install {}", packages)),
        PackageManager::Unknown => None,
    }
}

/// OpenWrt package that provides a given command.
fn package_for_tool(tool: &str) -> &str {
    match tool {
        "pgrep" => "procps-ng-pgrep",
        other => other,
    }
}

/// True when OpenWrt's firewall enables flow offloading. Offloaded flows skip
/// the netfilter hooks zapret relies on, so DPI desync may never see them.
pub fn flow_offloading_enabled() -> bool {
    let Ok(content) = fs::read_to_string(FIREWALL_CONFIG) else {
        return false;
    };
    content.lines().map(str::trim).any(|line| {
        let is_flag = line.starts_with("option flow_offloading ") || line.starts_with("option flow_offloading_hw ");
        is_flag && line.trim_end_matches(|c: char| c == '\'' || c == '"').ends_with('1')
    })
}

/// Collects warnings about the environment that would make router mode fail
/// or silently do nothing. Never blocks startup on its own.
pub fn preflight_warnings() -> Vec<String> {
    let mut warnings = Vec::new();

    if find_in_path("nft").is_none() && find_in_path("iptables").is_none() {
        warnings.push(rust_i18n::t!("router_warn_no_firewall").into_owned());
        if let Some(cmd) = install_command("nftables") {
            warnings.push(rust_i18n::t!("router_hint_install", command = cmd).into_owned());
        }
    }

    for tool in ["curl", "pgrep"] {
        if find_in_path(tool).is_none() {
            warnings.push(rust_i18n::t!("router_warn_missing_tool", tool = tool).into_owned());
            if let Some(cmd) = install_command(package_for_tool(tool)) {
                warnings.push(rust_i18n::t!("router_hint_install", command = cmd).into_owned());
            }
        }
    }

    if flow_offloading_enabled() {
        warnings.push(rust_i18n::t!("router_warn_flow_offloading").into_owned());
    }

    warnings
}

pub fn print_preflight() {
    for warning in preflight_warnings() {
        eprintln!("{}", warning);
    }
}
