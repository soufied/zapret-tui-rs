use std::fs;
use std::path::PathBuf;

pub const IPSET_ALL_FILE: &str = "ipset-all.txt";

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum IpsetMode {
    None,
    Any,
    Loaded,
    Custom,
}

impl std::fmt::Display for IpsetMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IpsetMode::None => rust_i18n::t!("ipset_none").into_owned(),
            IpsetMode::Any => rust_i18n::t!("ipset_any").into_owned(),
            IpsetMode::Loaded => rust_i18n::t!("ipset_loaded").into_owned(),
            IpsetMode::Custom => rust_i18n::t!("ipset_custom").into_owned(),
        };
        write!(f, "{}", s)
    }
}

pub fn get_ipset_dir() -> PathBuf {
    let engine = crate::runner::active_engine();
    let exe_dir = crate::config::get_app_dir();

    let workspace = engine.workspace_dir();
    let base_dir = if workspace.exists() {
        workspace
    } else {
        exe_dir.join(engine.workspace_folder())
    };
    let lists_dir = base_dir.join("lists");

    if lists_dir.exists() && lists_dir.is_dir() {
        lists_dir
    } else if base_dir.exists() && base_dir.is_dir() {
        base_dir
    } else {
        let local_base = PathBuf::from(engine.workspace_folder());
        let local_lists = local_base.join("lists");
        if local_lists.exists() && local_lists.is_dir() {
            local_lists
        } else {
            local_base
        }
    }
}

pub fn get_ipset_all_path() -> PathBuf {
    get_ipset_dir().join(IPSET_ALL_FILE)
}

pub fn get_ipset_backup_path() -> PathBuf {
    get_ipset_dir().join("ipset-all.txt.backup")
}

pub fn get_ipset_custom_path() -> PathBuf {
    get_ipset_dir().join("ipset-all.txt.custom")
}

pub fn determine_current_mode() -> IpsetMode {
    let path = get_ipset_all_path();
    if !path.exists() {
        return IpsetMode::Any;
    }

    let content = fs::read_to_string(&path).unwrap_or_default().trim().to_string();

    if content == "203.0.113.113/32" {
        return IpsetMode::None;
    }

    if content.is_empty() {
        return IpsetMode::Any;
    }

    let backup_path = get_ipset_backup_path();
    if backup_path.exists() {
        let backup_content = fs::read_to_string(&backup_path).unwrap_or_default().trim().to_string();
        if content == backup_content {
            return IpsetMode::Loaded;
        }
    }

    IpsetMode::Custom
}

pub fn engine_uses_global_ipset_mode(engine: &crate::config::ZapretEngine) -> bool {
    matches!(engine, crate::config::ZapretEngine::Zapret1)
}

pub fn legacy_mode_notice() -> String {
    rust_i18n::t!("ipset_z2_per_profile").into_owned()
}

pub fn get_available_modes_for(engine: &crate::config::ZapretEngine) -> Vec<IpsetMode> {
    if !engine_uses_global_ipset_mode(engine) {
        return Vec::new();
    }

    get_available_modes()
}

pub fn get_available_modes() -> Vec<IpsetMode> {
    if !crate::download::check_strategies_installed() {
        return vec![IpsetMode::None];
    }

    let mut modes = vec![IpsetMode::None, IpsetMode::Any, IpsetMode::Loaded];
    let custom_path = get_ipset_custom_path();

    if custom_path.exists() || determine_current_mode() == IpsetMode::Custom {
        modes.push(IpsetMode::Custom);
    }

    modes
}

pub fn apply_ipset_mode(old_mode: IpsetMode, new_mode: IpsetMode) {
    let path = get_ipset_all_path();
    let dir = get_ipset_dir();

    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }

    if old_mode == IpsetMode::Custom && new_mode != IpsetMode::Custom && path.exists() {
        let custom_path = get_ipset_custom_path();
        let _ = fs::copy(&path, &custom_path);
    }

    match new_mode {
        IpsetMode::None => {
            let _ = fs::write(&path, "203.0.113.113/32\n");
        }
        IpsetMode::Any => {
            let _ = fs::write(&path, "");
        }
        IpsetMode::Loaded => {
            let backup_path = get_ipset_backup_path();
            if backup_path.exists() {
                let _ = fs::copy(&backup_path, &path);
            } else {
                let _ = fs::write(&path, "");
            }
        }
        IpsetMode::Custom => {
            let custom_path = get_ipset_custom_path();
            if custom_path.exists() {
                let _ = fs::copy(&custom_path, &path);
            }
        }
    }
}
