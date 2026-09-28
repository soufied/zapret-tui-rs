use std::fs;
use std::path::{Path, PathBuf};

pub struct EditorCandidate {
    pub command: &'static str,
    pub label: &'static str,
    pub args: &'static [&'static str],
}

pub const EDITOR_CANDIDATES: &[EditorCandidate] = &[
    EditorCandidate {
        command: "nano",
        label: "nano",
        args: &[],
    },
    EditorCandidate {
        command: "micro",
        label: "micro",
        args: &[],
    },
    EditorCandidate {
        command: "vim",
        label: "vim",
        args: &[],
    },
    EditorCandidate {
        command: "nvim",
        label: "neovim",
        args: &[],
    },
    EditorCandidate {
        command: "hx",
        label: "helix",
        args: &[],
    },
    EditorCandidate {
        command: "emacs",
        label: "emacs",
        args: &["-nw"],
    },
    EditorCandidate {
        command: "vi",
        label: "vi",
        args: &[],
    },
    EditorCandidate {
        command: "subl",
        label: "sublime text",
        args: &["--wait"],
    },
    EditorCandidate {
        command: "code",
        label: "vs code",
        args: &["--wait"],
    },
    EditorCandidate {
        command: "gedit",
        label: "gedit",
        args: &["--wait"],
    },
    EditorCandidate {
        command: "kate",
        label: "kate",
        args: &["--block"],
    },
    EditorCandidate {
        command: "notepad",
        label: "notepad",
        args: &[],
    },
];

struct WaitRule {
    long: &'static str,
    short: char,
}

fn wait_rule(command: &str) -> Option<WaitRule> {
    let file = Path::new(command).file_name()?.to_string_lossy().to_lowercase();
    let name = file.trim_end_matches(".exe");

    if name.starts_with("subl") {
        return Some(WaitRule {
            long: "--wait",
            short: 'w',
        });
    }

    match name {
        "code" | "code-insiders" | "codium" | "vscodium" | "code-oss" | "gedit" => Some(WaitRule {
            long: "--wait",
            short: 'w',
        }),
        "kate" => Some(WaitRule {
            long: "--block",
            short: 'b',
        }),
        _ => None,
    }
}

fn has_short_flag(arg: &str, flag: char) -> bool {
    arg.len() > 1 && arg.starts_with('-') && !arg.starts_with("--") && arg[1..].contains(flag)
}

pub fn ensure_wait_flag(command: &str, mut args: Vec<String>) -> Vec<String> {
    let Some(rule) = wait_rule(command) else {
        return args;
    };

    let present = args
        .iter()
        .any(|a| a.as_str() == rule.long || has_short_flag(a, rule.short));
    if !present {
        args.insert(0, rule.long.to_string());
    }
    args
}

pub fn candidate_args(command: &str) -> Vec<String> {
    EDITOR_CANDIDATES
        .iter()
        .find(|c| c.command == command)
        .map(|c| c.args.iter().map(|a| a.to_string()).collect())
        .unwrap_or_default()
}

fn path_separator() -> char {
    if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    }
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn windows_path_extensions() -> Vec<String> {
    let mut exts = vec![String::new()];
    if let Ok(pathext) = std::env::var("PATHEXT") {
        for ext in pathext.split(';') {
            if !ext.trim().is_empty() {
                exts.push(ext.trim().to_lowercase());
            }
        }
    } else {
        exts.push(".exe".to_string());
    }
    exts
}

fn search_path_env(command: &str) -> Option<PathBuf> {
    let raw_path = std::env::var_os("PATH")?;
    let raw_path = raw_path.to_string_lossy().into_owned();

    let extensions: Vec<String> = if cfg!(target_os = "windows") {
        windows_path_extensions()
    } else {
        vec![String::new()]
    };

    for dir in raw_path.split(path_separator()) {
        if dir.trim().is_empty() {
            continue;
        }
        for ext in &extensions {
            let candidate = Path::new(dir).join(format!("{}{}", command, ext));
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn well_known_install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMW6432"] {
        if let Some(value) = std::env::var_os(var) {
            let path = PathBuf::from(value);
            if !path.as_os_str().is_empty() {
                roots.push(path);
            }
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("C:\\Program Files"));
        roots.push(PathBuf::from("C:\\Program Files (x86)"));
    }
    roots
}

#[cfg(target_os = "windows")]
fn well_known_candidates(stem: &str) -> Vec<PathBuf> {
    let roots = well_known_install_roots();
    let mut candidates = Vec::new();

    let relative: &[&str] = match stem {
        "subl" | "sublime_text" => &[
            "Sublime Text\\subl.exe",
            "Sublime Text\\sublime_text.exe",
            "Sublime Text 3\\subl.exe",
            "Sublime Text 3\\sublime_text.exe",
        ],
        "code" => &[
            "Microsoft VS Code\\bin\\code.cmd",
            "Microsoft VS Code\\Code.exe",
            "Programs\\Microsoft VS Code\\bin\\code.cmd",
            "Programs\\Microsoft VS Code\\Code.exe",
        ],
        "code-insiders" => &[
            "Microsoft VS Code Insiders\\bin\\code-insiders.cmd",
            "Microsoft VS Code Insiders\\Code - Insiders.exe",
            "Programs\\Microsoft VS Code Insiders\\bin\\code-insiders.cmd",
            "Programs\\Microsoft VS Code Insiders\\Code - Insiders.exe",
        ],
        "codium" => &[
            "VSCodium\\bin\\codium.cmd",
            "VSCodium\\VSCodium.exe",
            "Programs\\VSCodium\\bin\\codium.cmd",
            "Programs\\VSCodium\\VSCodium.exe",
        ],
        "notepad++" => &["Notepad++\\notepad++.exe"],
        "geany" => &["Geany\\bin\\geany.exe"],
        _ => &[],
    };

    for root in &roots {
        for rel in relative {
            candidates.push(root.join(rel));
        }
    }

    candidates
}

#[cfg(not(target_os = "windows"))]
fn well_known_candidates(stem: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut roots: Vec<PathBuf> = vec![
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/snap/bin"),
        PathBuf::from("/var/lib/flatpak/exports/bin"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/bin"));
        roots.push(home.join(".local/share/flatpak/exports/bin"));
    }
    if let Some(sudo_user) = std::env::var_os("SUDO_USER") {
        if let Some(name) = sudo_user.to_str() {
            if !name.is_empty() && name != "root" {
                let user_home = PathBuf::from("/home").join(name);
                roots.push(user_home.join(".local/bin"));
                roots.push(user_home.join(".local/share/flatpak/exports/bin"));
            }
        }
    }

    let relative: &[&str] = match stem {
        "subl" | "sublime_text" => &["subl", "sublime_text"],
        "code" => &["code", "com.visualstudio.code"],
        "code-insiders" => &["code-insiders"],
        "codium" | "vscodium" => &["codium", "vscodium", "com.vscodium.codium"],
        "gedit" => &["gedit", "org.gnome.gedit", "org.gnome.TextEditor"],
        "kate" => &["kate", "org.kde.kate"],
        _ => &[],
    };

    for root in &roots {
        for rel in relative {
            candidates.push(root.join(rel));
        }
    }

    candidates
}

fn search_well_known_locations(command: &str) -> Option<PathBuf> {
    let stem = crate::platform::launcher::command_stem(command);
    for candidate in well_known_candidates(&stem) {
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub fn find_in_path(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }

    let direct = Path::new(command);
    if direct.components().count() > 1 || direct.is_absolute() {
        return is_executable(direct).then(|| direct.to_path_buf());
    }

    if let Some(found) = search_path_env(command) {
        return Some(found);
    }

    search_well_known_locations(command)
}

pub fn editor_is_installed(command: &str) -> bool {
    find_in_path(command).is_some()
}

#[derive(Debug, Clone)]
pub struct ResolvedEditor {
    pub command: String,
    pub args: Vec<String>,
    pub source: EditorSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorSource {
    Configured,
    Environment,
    Detected,
}

#[derive(Debug, Clone)]
pub struct EditorResolution {
    pub editor: Option<ResolvedEditor>,
    pub skipped_configured: Option<String>,
}

fn split_command(raw: &str) -> Option<(String, Vec<String>)> {
    let mut parts = raw.split_whitespace();
    let program = parts.next()?.to_string();
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();
    Some((program, args))
}

pub fn resolve_editor_detailed() -> EditorResolution {
    let configured = crate::config::load_editor();
    let mut skipped_configured: Option<String> = None;

    if !configured.trim().is_empty() {
        if let Some((program, mut args)) = split_command(&configured) {
            if editor_is_installed(&program) {
                if args.is_empty() {
                    args = candidate_args(&program);
                }
                let args = ensure_wait_flag(&program, args);
                return EditorResolution {
                    editor: Some(ResolvedEditor {
                        command: program,
                        args,
                        source: EditorSource::Configured,
                    }),
                    skipped_configured: None,
                };
            }
            skipped_configured = Some(program);
        } else {
            skipped_configured = Some(configured.trim().to_string());
        }
    }

    for var in ["VISUAL", "EDITOR"] {
        let Ok(raw) = std::env::var(var) else {
            continue;
        };
        let Some((program, mut args)) = split_command(&raw) else {
            continue;
        };
        if editor_is_installed(&program) {
            if args.is_empty() {
                args = candidate_args(&program);
            }
            let args = ensure_wait_flag(&program, args);
            return EditorResolution {
                editor: Some(ResolvedEditor {
                    command: program,
                    args,
                    source: EditorSource::Environment,
                }),
                skipped_configured,
            };
        }
    }

    for candidate in EDITOR_CANDIDATES {
        if editor_is_installed(candidate.command) {
            let args = ensure_wait_flag(
                candidate.command,
                candidate.args.iter().map(|a| a.to_string()).collect(),
            );
            return EditorResolution {
                editor: Some(ResolvedEditor {
                    command: candidate.command.to_string(),
                    args,
                    source: EditorSource::Detected,
                }),
                skipped_configured,
            };
        }
    }

    EditorResolution {
        editor: None,
        skipped_configured,
    }
}

pub fn resolve_editor() -> Option<ResolvedEditor> {
    resolve_editor_detailed().editor
}

pub fn active_editor_label() -> String {
    let configured = crate::config::load_editor();
    if configured.trim().is_empty() {
        return match resolve_editor() {
            Some(ed) => format!("{} ({})", ed.command, rust_i18n::t!("settings_editor_auto")),
            None => rust_i18n::t!("settings_editor_none").into_owned(),
        };
    }
    configured
}

fn backup_dir() -> PathBuf {
    crate::config::get_cache_dir().join("backups")
}

const MAX_BACKUPS_PER_FILE: usize = 10;

fn prune_backups(stem: &str) {
    let dir = backup_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };

    let prefix = format!("{}.", stem);
    let mut matching: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if !name.starts_with(&prefix) || !name.ends_with(".bak") {
                return None;
            }
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e.path()))
        })
        .collect();

    if matching.len() <= MAX_BACKUPS_PER_FILE {
        return;
    }

    matching.sort_by_key(|(t, _)| *t);
    let excess = matching.len() - MAX_BACKUPS_PER_FILE;
    for (_, path) in matching.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}

pub fn backup_file(file_path: &str) -> Option<PathBuf> {
    let source = Path::new(file_path);
    if !source.is_file() {
        return None;
    }

    let stem = source.file_name()?.to_string_lossy().into_owned();
    let dir = backup_dir();
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let target = dir.join(format!("{}.{}.bak", stem, stamp));
    if fs::copy(source, &target).is_err() {
        return None;
    }

    prune_backups(&stem);
    Some(target)
}

pub fn get_lists_files() -> Vec<String> {
    let mut files = Vec::new();

    let exe_dir = crate::config::get_app_dir();

    let engine = crate::runner::active_engine();
    let base_dir = {
        let workspace = engine.workspace_dir();
        if workspace.exists() {
            workspace
        } else {
            exe_dir.join(engine.workspace_folder())
        }
    };
    let lists_dir = base_dir.join("lists");

    if lists_dir.exists() && lists_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(lists_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    files.push(path.to_string_lossy().into_owned());
                }
            }
        }
    } else if base_dir.exists() && base_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "txt") {
                    files.push(path.to_string_lossy().into_owned());
                }
            }
        }
    } else {
        let local_base = std::path::PathBuf::from(engine.workspace_folder());
        let local_base = local_base.as_path();
        let local_lists = local_base.join("lists");

        if local_lists.exists() && local_lists.is_dir() {
            if let Ok(entries) = fs::read_dir(local_lists) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        files.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        } else if local_base.exists() && local_base.is_dir() {
            if let Ok(entries) = fs::read_dir(local_base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|e| e == "txt") {
                        files.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    files.sort();
    files
}

use crate::platform::launcher::{classify_editor, launch_editor_process, EditorKind, LaunchOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSession {
    Closed(i32),
    Detached(String),
}

#[derive(Debug, Clone)]
pub struct EditorLaunch {
    pub session: EditorSession,
    pub kind: EditorKind,
    pub command: String,
}

fn launch_editor(command: &str, args: &[String], file_path: &str) -> Result<EditorSession, String> {
    match launch_editor_process(command, args, file_path) {
        Ok(LaunchOutcome::Completed(code)) => Ok(EditorSession::Closed(code)),
        Ok(LaunchOutcome::Detached) => Ok(EditorSession::Detached(command.to_string())),
        Err(error) => Err(error.to_string()),
    }
}

pub fn open_editor_detailed(file_path: &str) -> std::io::Result<EditorLaunch> {
    if crate::config::load_backup_lists() {
        let _ = backup_file(file_path);
    }

    let mut failures: Vec<String> = Vec::new();
    let mut tried: Vec<String> = Vec::new();

    let resolution = resolve_editor_detailed();

    if let Some(skipped) = &resolution.skipped_configured {
        let message = format!(
            "Configured editor '{}' is not available on PATH for this process; falling back to auto-detection",
            skipped
        );
        crate::logger::log_error(&message);
    }

    let configured_kind = resolution
        .skipped_configured
        .as_deref()
        .map(classify_editor)
        .or_else(|| resolution.editor.as_ref().map(|editor| classify_editor(&editor.command)))
        .unwrap_or(EditorKind::Terminal);

    if let Some(editor) = resolution.editor {
        crate::logger::log_info(&format!(
            "Launching editor '{}' resolved from {:?}",
            editor.command, editor.source
        ));
        let kind = classify_editor(&editor.command);
        match launch_editor(&editor.command, &editor.args, file_path) {
            Ok(session) => {
                return Ok(EditorLaunch {
                    session,
                    kind,
                    command: editor.command,
                })
            }
            Err(reason) => {
                crate::logger::log_error(&format!("editor '{}' failed to launch: {}", editor.command, reason));
                failures.push(format!("{}: {}", editor.command, reason));
            }
        }
        tried.push(editor.command);
    }

    for candidate in EDITOR_CANDIDATES {
        if tried.iter().any(|t| t.as_str() == candidate.command) || !editor_is_installed(candidate.command) {
            continue;
        }

        if configured_kind == EditorKind::Graphical && classify_editor(candidate.command) == EditorKind::Terminal {
            continue;
        }

        let args = ensure_wait_flag(
            candidate.command,
            candidate.args.iter().map(|a| a.to_string()).collect(),
        );

        match launch_editor(candidate.command, &args, file_path) {
            Ok(session) => {
                return Ok(EditorLaunch {
                    session,
                    kind: classify_editor(candidate.command),
                    command: candidate.command.to_string(),
                })
            }
            Err(reason) => {
                crate::logger::log_error(&format!("editor '{}' failed to launch: {}", candidate.command, reason));
                failures.push(format!("{}: {}", candidate.command, reason));
            }
        }
        tried.push(candidate.command.to_string());
    }

    let detail = if failures.is_empty() {
        "No suitable editor found on PATH".to_string()
    } else {
        format!("No editor could be launched: {}", failures.join("; "))
    };

    crate::logger::log_error(&detail);
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, detail))
}

pub fn open_editor(file_path: &str) -> std::io::Result<EditorSession> {
    open_editor_detailed(file_path).map(|launch| launch.session)
}

pub fn read_log_tail(path: &Path, max_lines: usize) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].iter().map(|l| l.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sublime_gets_wait_flag() {
        assert_eq!(ensure_wait_flag("subl", Vec::new()), owned(&["--wait"]));
        assert_eq!(ensure_wait_flag("/usr/bin/subl", owned(&["-n"])), owned(&["--wait", "-n"]));
        assert_eq!(ensure_wait_flag("sublime_text", Vec::new()), owned(&["--wait"]));
        assert_eq!(ensure_wait_flag("subl.exe", Vec::new()), owned(&["--wait"]));
    }

    #[test]
    fn existing_wait_flag_is_not_duplicated() {
        assert_eq!(ensure_wait_flag("subl", owned(&["-w"])), owned(&["-w"]));
        assert_eq!(ensure_wait_flag("subl", owned(&["--wait"])), owned(&["--wait"]));
        assert_eq!(ensure_wait_flag("subl", owned(&["-nw"])), owned(&["-nw"]));
    }

    #[test]
    fn long_options_containing_w_do_not_count_as_wait() {
        assert_eq!(
            ensure_wait_flag("subl", owned(&["--new-window"])),
            owned(&["--wait", "--new-window"])
        );
    }

    #[test]
    fn other_gui_editors_get_their_blocking_flag() {
        assert_eq!(ensure_wait_flag("code", Vec::new()), owned(&["--wait"]));
        assert_eq!(ensure_wait_flag("gedit", Vec::new()), owned(&["--wait"]));
        assert_eq!(ensure_wait_flag("kate", Vec::new()), owned(&["--block"]));
        assert_eq!(ensure_wait_flag("kate", owned(&["-b"])), owned(&["-b"]));
    }

    #[test]
    fn terminal_editors_are_untouched() {
        assert_eq!(ensure_wait_flag("nano", Vec::new()), Vec::<String>::new());
        assert_eq!(ensure_wait_flag("vim", owned(&["-p"])), owned(&["-p"]));
    }
}
