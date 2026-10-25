
use crate::common::utils::find_in_path;
use std::fs;
use std::path::Path;
use std::process::Command;

pub const DEFAULT_CACHE_DIR: &str = "/usr/share/zapret-rust";

const OPENWRT_RELEASE: &str = "/etc/openwrt_release";
const OS_RELEASE: &str = "/etc/os-release";
const FIREWALL_CONFIG: &str = "/etc/config/firewall";
const NFT_QUEUE_MODULE: &str = "nft_queue";
const NFT_QUEUE_PACKAGE: &str = "kmod-nft-queue";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apk,
    Opkg,
    Unknown,
}

pub fn is_openwrt() -> bool {
    if Path::new(OPENWRT_RELEASE).exists() {
        return true;
    }
    fs::read_to_string(OS_RELEASE)
        .map(|content| content.lines().any(|line| line.starts_with("ID=openwrt") || line.starts_with("NAME=\"OpenWrt")))
        .unwrap_or(false)
}

pub fn package_manager() -> PackageManager {
    if find_in_path("apk").is_some() {
        PackageManager::Apk
    } else if find_in_path("opkg").is_some() {
        PackageManager::Opkg
    } else {
        PackageManager::Unknown
    }
}

pub fn install_command(packages: &str) -> Option<String> {
    match package_manager() {
        PackageManager::Apk => Some(format!("apk update && apk add {}", packages)),
        PackageManager::Opkg => Some(format!("opkg update && opkg install {}", packages)),
        PackageManager::Unknown => None,
    }
}

pub fn package_installed(name: &str) -> bool {
    match package_manager() {
        PackageManager::Apk => Command::new("apk")
            .args(["info", "-e", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
        PackageManager::Opkg => Command::new("opkg")
            .args(["list-installed", name])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|line| line.split(' ').next() == Some(name))
            })
            .unwrap_or(false),
        PackageManager::Unknown => false,
    }
}

fn package_for_tool(tool: &str) -> &str {
    match tool {
        "pgrep" => "procps-ng-pgrep",
        other => other,
    }
}

fn kernel_module_loaded(module: &str) -> bool {
    if Path::new("/sys/module").join(module).exists() {
        return true;
    }
    fs::read_to_string("/proc/modules")
        .map(|list| list.lines().any(|line| line.split(' ').next() == Some(module)))
        .unwrap_or(false)
}

pub fn flow_offloading_enabled() -> bool {
    let Ok(content) = fs::read_to_string(FIREWALL_CONFIG) else {
        return false;
    };
    content.lines().map(str::trim).any(|line| {
        let is_flag = line.starts_with("option flow_offloading ") || line.starts_with("option flow_offloading_hw ");
        is_flag && line.trim_end_matches(|c: char| c == '\'' || c == '"').ends_with('1')
    })
}

pub fn preflight_warnings() -> Vec<String> {
    let mut warnings = Vec::new();

    let has_nft = find_in_path("nft").is_some();
    if !has_nft && find_in_path("iptables").is_none() {
        warnings.push(rust_i18n::t!("router_warn_no_firewall").into_owned());
        if let Some(cmd) = install_command("nftables") {
            warnings.push(rust_i18n::t!("router_hint_install", command = cmd).into_owned());
        }
    }

    if has_nft && !kernel_module_loaded(NFT_QUEUE_MODULE) && !package_installed(NFT_QUEUE_PACKAGE) {
        warnings.push(rust_i18n::t!("router_warn_nft_queue").into_owned());
        if let Some(cmd) = install_command(NFT_QUEUE_PACKAGE) {
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
