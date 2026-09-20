#[cfg(target_os = "linux")]
pub mod dinit;
#[cfg(target_os = "linux")]
pub mod init;
#[cfg(target_os = "linux")]
pub mod openrc;
#[cfg(target_os = "linux")]
pub mod runit;
#[cfg(target_os = "linux")]
pub mod s6;
#[cfg(target_os = "linux")]
pub mod systemd;

#[cfg(target_os = "linux")]
pub use dinit::DinitManager;
#[cfg(target_os = "linux")]
pub use init::InitManager;
#[cfg(target_os = "linux")]
pub use openrc::OpenRcManager;
#[cfg(target_os = "linux")]
pub use runit::RunitManager;
#[cfg(target_os = "linux")]
pub use s6::S6Manager;
#[cfg(target_os = "linux")]
pub use systemd::SystemdManager;

#[cfg(target_os = "windows")]
pub mod winservice;

#[cfg(target_os = "windows")]
pub use winservice::WindowsServiceManager;

use crate::config::ZapretEngine;
use std::path::Path;

pub const LEGACY_SERVICE_NAME: &str = "zapret-rust";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceConflict {
    pub active: bool,
    pub enabled: bool,
}

pub fn service_description(name: &str) -> &'static str {
    if name == ZapretEngine::Zapret2.service_name() {
        "Zapret2 Discord Youtube Service"
    } else {
        "Zapret Discord Youtube Service"
    }
}

pub trait ServiceManager: Send + Sync {
    fn is_installed(&self) -> bool;
    fn is_active(&self) -> bool;
    fn is_enabled(&self) -> bool {
        self.is_installed()
    }
    fn install(&self, exe_path: &Path, config_path: &Path, cache_dir: &Path) -> Result<(), String>;
    fn uninstall(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
    fn restart(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitType {
    #[cfg(target_os = "linux")]
    Systemd,
    #[cfg(target_os = "linux")]
    OpenRc,
    #[cfg(target_os = "linux")]
    Runit,
    #[cfg(target_os = "linux")]
    Dinit,
    #[cfg(target_os = "linux")]
    S6,
    #[cfg(target_os = "linux")]
    Init,
    #[cfg(target_os = "windows")]
    Windows,
}

impl InitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            Self::Systemd => "systemd",
            #[cfg(target_os = "linux")]
            Self::OpenRc => "openrc",
            #[cfg(target_os = "linux")]
            Self::Runit => "runit",
            #[cfg(target_os = "linux")]
            Self::Dinit => "dinit",
            #[cfg(target_os = "linux")]
            Self::S6 => "s6",
            #[cfg(target_os = "linux")]
            Self::Init => "init",
            #[cfg(target_os = "windows")]
            Self::Windows => "windows",
        }
    }
}

pub fn detect_init_system() -> Option<InitType> {
    #[cfg(target_os = "windows")]
    {
        Some(InitType::Windows)
    }

    #[cfg(target_os = "linux")]
    {
        if Path::new("/run/systemd/system").exists() {
            return Some(InitType::Systemd);
        }

        if Path::new("/run/openrc").exists() || Path::new("/sbin/openrc-run").exists() {
            return Some(InitType::OpenRc);
        }

        if Path::new("/etc/dinit.d").exists() {
            if std::process::Command::new("which")
                .arg("dinitctl")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(InitType::Dinit);
            }
        }

        if Path::new("/etc/runit").exists() || Path::new("/var/service").exists() {
            if std::process::Command::new("which")
                .arg("sv")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(InitType::Runit);
            }
        }

        if Path::new("/etc/s6").exists() {
            if std::process::Command::new("which")
                .arg("s6-svstat")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Some(InitType::S6);
            }
        }

        if Path::new("/etc/init.d").exists() {
            return Some(InitType::Init);
        }

        None
    }
}

pub fn get_manager_named(init_type: InitType, name: &'static str) -> Box<dyn ServiceManager> {
    match init_type {
        #[cfg(target_os = "linux")]
        InitType::Systemd => Box::new(SystemdManager::new(name)),
        #[cfg(target_os = "linux")]
        InitType::OpenRc => Box::new(OpenRcManager::new(name)),
        #[cfg(target_os = "linux")]
        InitType::Runit => Box::new(RunitManager::new(name)),
        #[cfg(target_os = "linux")]
        InitType::Dinit => Box::new(DinitManager::new(name)),
        #[cfg(target_os = "linux")]
        InitType::S6 => Box::new(S6Manager::new(name)),
        #[cfg(target_os = "linux")]
        InitType::Init => Box::new(InitManager::new(name)),

        #[cfg(target_os = "windows")]
        InitType::Windows => {
            let _ = name;
            Box::new(WindowsServiceManager)
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_detected_manager_for(engine: &ZapretEngine) -> Option<Box<dyn ServiceManager>> {
    detect_init_system().map(|init_type| get_manager_named(init_type, engine.service_name()))
}

#[cfg(target_os = "linux")]
pub fn legacy_service_conflict(engine: &ZapretEngine) -> Option<ServiceConflict> {
    if *engine != ZapretEngine::Zapret2 {
        return None;
    }

    let init_type = detect_init_system()?;
    let legacy = get_manager_named(init_type, LEGACY_SERVICE_NAME);
    if !legacy.is_installed() {
        return None;
    }

    let active = legacy.is_active();
    let enabled = legacy.is_enabled();
    if active || enabled {
        Some(ServiceConflict { active, enabled })
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn legacy_service_conflict(_engine: &ZapretEngine) -> Option<ServiceConflict> {
    None
}

#[cfg(target_os = "linux")]
pub fn remove_legacy_service() -> Result<(), String> {
    let init_type = detect_init_system().ok_or_else(|| rust_i18n::t!("msg_err_init").into_owned())?;
    get_manager_named(init_type, LEGACY_SERVICE_NAME).uninstall()
}

#[cfg(not(target_os = "linux"))]
pub fn remove_legacy_service() -> Result<(), String> {
    Ok(())
}
