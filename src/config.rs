use std::env;
use std::fmt;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum ZapretEngine {
    Zapret1,
    Zapret2,
}

impl Default for ZapretEngine {
    fn default() -> Self {
        Self::Zapret1
    }
}

impl fmt::Display for ZapretEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zapret1 => write!(f, "zapret"),
            Self::Zapret2 => write!(f, "zapret2"),
        }
    }
}

impl ZapretEngine {
    pub fn workspace_folder(&self) -> String {
        match self {
            Self::Zapret1 => "zapret-discord-youtube-linux".to_string(),
            Self::Zapret2 => "zapret2-discord-youtube-linux".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zapret2" => Self::Zapret2,
            _ => Self::Zapret1,
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Zapret1 => {
                if cfg!(target_os = "windows") {
                    "winws.exe"
                } else {
                    "nfqws"
                }
            }
            Self::Zapret2 => {
                if cfg!(target_os = "windows") {
                    "winws2.exe"
                } else {
                    "nfqws2"
                }
            }
        }
    }

    pub fn workspace_dir(&self) -> std::path::PathBuf {
        get_cache_dir().join(self.workspace_folder())
    }

    pub fn presets_dir(&self) -> std::path::PathBuf {
        self.workspace_dir().join("presets")
    }

    pub fn profiles_dir(&self) -> std::path::PathBuf {
        self.workspace_dir().join("profiles")
    }

    pub fn binary_path(&self) -> std::path::PathBuf {
        get_cache_dir().join("bin").join(self.binary_name())
    }

    pub fn uses_presets(&self) -> bool {
        matches!(self, Self::Zapret2)
    }

    pub fn supports_game_filter(&self) -> bool {
        matches!(self, Self::Zapret1)
    }

    pub fn service_name(&self) -> &'static str {
        match self {
            Self::Zapret1 => "zapret-rust",
            Self::Zapret2 => "zapret2-rust",
        }
    }
}

pub fn load_engine() -> ZapretEngine {
    load_config(&config_path().to_string_lossy())
        .map(|cfg| cfg.engine)
        .unwrap_or_default()
}

pub fn save_strategy(strategy: &str) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.strategy = strategy.to_string();
    save_config(&cfg)
}

#[derive(Debug)]
pub struct RunConfig {
    pub engine: ZapretEngine,
    pub interface: String,
    pub strategy: String,
    pub gamefilter_tcp: bool,
    pub gamefilter_udp: bool,
    pub backend: String,
    pub active_discord_fake: String,
    pub active_gamefilter_fake: String,
    pub dpi_desync_ttl: Option<u8>,
    pub editor: String,
    pub backup_lists: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            engine: ZapretEngine::Zapret1,
            interface: "any".to_string(),
            strategy: String::new(),
            gamefilter_tcp: false,
            gamefilter_udp: false,
            backend: "nftables".to_string(),
            active_discord_fake: "quic_initial_steamcommunity_com.bin".to_string(),
            active_gamefilter_fake: "quic_initial_4pda_to.bin".to_string(),
            dpi_desync_ttl: None,
            editor: String::new(),
            backup_lists: true,
        }
    }
}

pub fn load_config(file: &str) -> Result<RunConfig, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("Cannot read config '{}': {}", file, e))?;

    let mut cfg = RunConfig::default();

    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("engine=") {
            cfg.engine = ZapretEngine::from_str(val.trim());
        } else if let Some(val) = line.strip_prefix("interface=") {
            cfg.interface = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("strategy=") {
            cfg.strategy = val.trim().to_string();
        } else if line == "gamefiltertcp=true" {
            cfg.gamefilter_tcp = true;
        } else if line == "gamefilterudp=true" {
            cfg.gamefilter_udp = true;
        } else if let Some(val) = line.strip_prefix("backend=") {
            cfg.backend = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("active_discord_fake=") {
            cfg.active_discord_fake = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("active_gamefilter_fake=") {
            cfg.active_gamefilter_fake = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("dpi_desync_ttl=") {
            cfg.dpi_desync_ttl = val.trim().parse::<u8>().ok();
        } else if let Some(val) = line.strip_prefix("editor=") {
            cfg.editor = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("backup_lists=") {
            cfg.backup_lists = !matches!(val.trim().to_lowercase().as_str(), "false" | "0" | "no" | "off");
        }
    }

    Ok(cfg)
}

pub fn get_interfaces() -> Vec<String> {
    #[allow(unused_mut)]
    let mut interfaces = vec!["any".to_string()];

    #[cfg(target_os = "linux")]
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                interfaces.push(name);
            }
        }
    }

    let _ = env::consts::OS;
    interfaces
}

pub fn get_cache_dir() -> std::path::PathBuf {
    if let Ok(val) = env::var("ZAPRET_CACHE_DIR") {
        std::path::PathBuf::from(val)
    } else if let Ok(exe_path) = env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            parent.to_path_buf()
        } else {
            std::path::PathBuf::from(".")
        }
    } else {
        std::path::PathBuf::from(".")
    }
}

const CONFIG_FILENAME: &str = "conf.env";

const DEFAULT_CONFIG_LINES: &[&str] = &[
    "engine=zapret",
    "interface=any",
    "strategy=",
    "gamefiltertcp=false",
    "gamefilterudp=false",
    "backend=nftables",
    "active_discord_fake=quic_initial_steamcommunity_com.bin",
    "active_gamefilter_fake=quic_initial_4pda_to.bin",
    "dpi_desync_ttl=",
    "editor=",
    "backup_lists=true",
];

pub fn config_path() -> std::path::PathBuf {
    get_cache_dir().join(CONFIG_FILENAME)
}

pub fn save_config(cfg: &RunConfig) -> Result<(), String> {
    let path = config_path();
    let ttl = cfg.dpi_desync_ttl.map(|v| v.to_string()).unwrap_or_default();
    let content = format!(
        "engine={}\ninterface={}\nstrategy={}\ngamefiltertcp={}\ngamefilterudp={}\nbackend={}\nactive_discord_fake={}\nactive_gamefilter_fake={}\ndpi_desync_ttl={}\neditor={}\nbackup_lists={}\n",
        cfg.engine, cfg.interface, cfg.strategy, cfg.gamefilter_tcp, cfg.gamefilter_udp, cfg.backend,
        cfg.active_discord_fake, cfg.active_gamefilter_fake, ttl, cfg.editor, cfg.backup_lists,
    );
    fs::write(&path, &content).map_err(|e| format!("Cannot write config '{}': {}", path.display(), e))?;
    Ok(())
}

pub fn load_ttl() -> Option<u8> {
    load_config(&config_path().to_string_lossy())
        .ok()
        .and_then(|cfg| cfg.dpi_desync_ttl)
}

pub fn save_ttl(ttl: Option<u8>) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.dpi_desync_ttl = ttl;
    save_config(&cfg)
}

pub fn save_tui_state(
    engine: &ZapretEngine,
    interface: &str,
    strategy: &str,
    tcp: bool,
    udp: bool,
    backend: &str,
) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.engine = engine.clone();
    cfg.interface = interface.to_string();
    cfg.strategy = strategy.to_string();
    cfg.gamefilter_tcp = tcp;
    cfg.gamefilter_udp = udp;
    cfg.backend = backend.to_string();
    save_config(&cfg)
}

pub fn ensure_default_config() -> Result<(), String> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Cannot create config directory: {}", e))?;
        }
        save_config(&RunConfig::default())?;
    }
    validate_config()?;
    Ok(())
}

fn validate_config() -> Result<(), String> {
    let path = config_path();
    let mut content =
        fs::read_to_string(&path).map_err(|e| format!("Cannot read config '{}': {}", path.display(), e))?;

    let existing_keys: Vec<&str> = content
        .lines()
        .filter_map(|line| line.trim().split_once('=').map(|(k, _)| k.trim()))
        .collect();

    let mut missing = Vec::new();
    for default_line in DEFAULT_CONFIG_LINES {
        if let Some(key) = default_line.split_once('=').map(|(k, _)| k.trim()) {
            if !existing_keys.contains(&key) {
                missing.push(*default_line);
            }
        }
    }

    let mut updated = false;
    if !missing.is_empty() {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        for line in &missing {
            content.push_str(line);
            content.push('\n');
        }
        updated = true;
    }

    let defaults = [
        ("active_discord_fake=", "quic_initial_steamcommunity_com.bin"),
        ("active_gamefilter_fake=", "quic_initial_4pda_to.bin"),
    ];

    for (key, default_val) in &defaults {
        let needs_fix = content
            .lines()
            .any(|line| line.trim().strip_prefix(*key).is_some_and(|val| val.trim().is_empty()));
        if needs_fix {
            content = content.replace(*key, &format!("{}{}", key, default_val));
            updated = true;
        }
    }

    let legacy_gamefilter_fake = "active_gamefilter_fake=quic_initial_4pda.to.bin";
    if content.contains(legacy_gamefilter_fake) {
        content = content.replace(
            legacy_gamefilter_fake,
            "active_gamefilter_fake=quic_initial_4pda_to.bin",
        );
        updated = true;
    }

    if updated {
        fs::write(&path, &content).map_err(|e| format!("Cannot write config '{}': {}", path.display(), e))?;
    }

    Ok(())
}

pub fn load_active_fakes() -> (String, String) {
    let cfg = load_config(&config_path().to_string_lossy()).unwrap_or_default();
    (cfg.active_discord_fake, cfg.active_gamefilter_fake)
}

pub fn save_active_fakes(discord: &str, game: &str) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.active_discord_fake = discord.to_string();
    cfg.active_gamefilter_fake = game.to_string();
    save_config(&cfg)
}
pub fn load_editor() -> String {
    load_config(&config_path().to_string_lossy())
        .map(|cfg| cfg.editor)
        .unwrap_or_default()
}

pub fn save_editor(editor: &str) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.editor = editor.trim().to_string();
    save_config(&cfg)
}

pub fn load_backup_lists() -> bool {
    load_config(&config_path().to_string_lossy())
        .map(|cfg| cfg.backup_lists)
        .unwrap_or(true)
}

pub fn save_backup_lists(enabled: bool) -> Result<(), String> {
    let path = config_path();
    let mut cfg = load_config(&path.to_string_lossy()).unwrap_or_default();
    cfg.backup_lists = enabled;
    save_config(&cfg)
}
