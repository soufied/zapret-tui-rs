use std::path::Path;
use std::process::{Command, Stdio};

pub const TERMINAL_EDITORS: &[&str] = &[
    "nano", "vim", "nvim", "vi", "micro", "hx", "helix", "emacs", "ne", "joe", "mcedit",
];

pub const GRAPHICAL_EDITORS: &[&str] = &[
    "subl",
    "sublime_text",
    "code",
    "code-insiders",
    "codium",
    "gedit",
    "kate",
    "kwrite",
    "gnome-text-editor",
    "mousepad",
    "notepad",
    "notepad++",
    "geany",
    "featherpad",
    "pluma",
];

pub const GRAPHICAL_ENV_KEYS: &[&str] = &[
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "QT_QPA_PLATFORM",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    Terminal,
    Graphical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchOutcome {
    Completed(i32),
    Detached,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchError {
    pub program: String,
    pub reason: String,
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to launch {}: {}", self.program, self.reason)
    }
}

pub fn command_stem(command: &str) -> String {
    let trimmed = command.trim().trim_matches('"');
    let name = Path::new(trimmed)
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| trimmed.to_string());
    name.trim_end_matches(".exe").to_lowercase()
}

pub fn classify_editor(command: &str) -> EditorKind {
    let stem = command_stem(command);

    if TERMINAL_EDITORS.iter().any(|entry| *entry == stem) {
        return EditorKind::Terminal;
    }

    if GRAPHICAL_EDITORS.iter().any(|entry| *entry == stem) {
        return EditorKind::Graphical;
    }

    EditorKind::Terminal
}

#[cfg(target_os = "linux")]
pub fn original_user() -> Option<String> {
    for key in ["SUDO_USER", "PKEXEC_USER", "DOAS_USER"] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if !value.is_empty() && value != "root" {
                return Some(value);
            }
        }
    }

    if let Ok(uid) = std::env::var("PKEXEC_UID") {
        let uid = uid.trim();
        if !uid.is_empty() {
            if let Some(name) = username_for_uid(uid) {
                return Some(name);
            }
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn username_for_uid(uid: &str) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;

    for line in passwd.lines() {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let _password = fields.next()?;
        let entry_uid = fields.next()?;
        if entry_uid == uid && name != "root" {
            return Some(name.to_string());
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn preserved_environment() -> Vec<(String, String)> {
    let mut preserved = Vec::new();

    for key in GRAPHICAL_ENV_KEYS {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                preserved.push(((*key).to_string(), value));
            }
        }
    }

    preserved
}

#[cfg(target_os = "linux")]
pub fn build_deescalated_invocation(
    user: &str,
    environment: &[(String, String)],
    program: &str,
    args: &[String],
    file_path: &str,
) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();

    if crate::utils::find_in_path("runuser").is_some() {
        parts.push("runuser".to_string());
        parts.push("-u".to_string());
        parts.push(user.to_string());
        parts.push("--".to_string());
    } else {
        parts.push("sudo".to_string());
        parts.push("-u".to_string());
        parts.push(user.to_string());
        parts.push("--".to_string());
    }

    parts.push("env".to_string());
    for (key, value) in environment {
        parts.push(format!("{}={}", key, value));
    }

    parts.push(program.to_string());
    for arg in args {
        parts.push(arg.clone());
    }
    parts.push(file_path.to_string());

    parts
}

#[cfg(target_os = "linux")]
fn spawn_graphical(program: &str, args: &[String], file_path: &str) -> Result<LaunchOutcome, LaunchError> {
    let environment = preserved_environment();

    if environment.is_empty() {
        return Err(LaunchError {
            program: program.to_string(),
            reason: "no graphical session variables are available in this environment".to_string(),
        });
    }

    let invocation = match original_user() {
        Some(user) => build_deescalated_invocation(&user, &environment, program, args, file_path),
        None => {
            let mut direct: Vec<String> = vec![program.to_string()];
            direct.extend(args.iter().cloned());
            direct.push(file_path.to_string());
            direct
        }
    };

    let (head, tail) = invocation.split_first().ok_or_else(|| LaunchError {
        program: program.to_string(),
        reason: "empty invocation".to_string(),
    })?;

    let mut command = Command::new(head);
    command.args(tail);

    if original_user().is_none() {
        for (key, value) in &environment {
            command.env(key, value);
        }
    }

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command
        .spawn()
        .map(|_| LaunchOutcome::Detached)
        .map_err(|error| LaunchError {
            program: program.to_string(),
            reason: error.to_string(),
        })
}

#[cfg(target_os = "windows")]
fn quote_argument(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(target_os = "windows")]
pub fn build_shell_dispatch_script(program: &str, args: &[String], file_path: &str) -> String {
    let mut arguments: Vec<String> = args.to_vec();
    arguments.push(file_path.to_string());

    let joined = arguments
        .iter()
        .map(|value| format!("\"{}\"", value.replace('"', "\\\"")))
        .collect::<Vec<String>>()
        .join(" ");

    format!(
        "$shell = New-Object -ComObject Shell.Application; $shell.ShellExecute({}, {}, '', 'open', 1)",
        quote_argument(program),
        quote_argument(&joined)
    )
}

#[cfg(target_os = "windows")]
fn spawn_graphical(program: &str, args: &[String], file_path: &str) -> Result<LaunchOutcome, LaunchError> {
    if !crate::platform::windows::is_elevated_process() {
        let mut command = Command::new(program);
        command.args(args).arg(file_path);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        return command
            .spawn()
            .map(|_| LaunchOutcome::Detached)
            .map_err(|error| LaunchError {
                program: program.to_string(),
                reason: error.to_string(),
            });
    }

    let script = build_shell_dispatch_script(program, args, file_path);

    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command"])
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match command.status() {
        Ok(status) if status.success() => Ok(LaunchOutcome::Detached),
        Ok(status) => Err(LaunchError {
            program: program.to_string(),
            reason: format!(
                "the de-elevating shell dispatch exited with {}",
                status.code().unwrap_or(-1)
            ),
        }),
        Err(error) => Err(LaunchError {
            program: program.to_string(),
            reason: error.to_string(),
        }),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn spawn_graphical(program: &str, args: &[String], file_path: &str) -> Result<LaunchOutcome, LaunchError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .arg(file_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command
        .spawn()
        .map(|_| LaunchOutcome::Detached)
        .map_err(|error| LaunchError {
            program: program.to_string(),
            reason: error.to_string(),
        })
}

fn spawn_terminal(program: &str, args: &[String], file_path: &str) -> Result<LaunchOutcome, LaunchError> {
    let status = Command::new(program)
        .args(args)
        .arg(file_path)
        .status()
        .map_err(|error| LaunchError {
            program: program.to_string(),
            reason: error.to_string(),
        })?;

    Ok(LaunchOutcome::Completed(status.code().unwrap_or(0)))
}

pub fn launch_editor_process(
    program: &str,
    args: &[String],
    file_path: &str,
) -> Result<LaunchOutcome, LaunchError> {
    match classify_editor(program) {
        EditorKind::Terminal => spawn_terminal(program, args, file_path),
        EditorKind::Graphical => spawn_graphical(program, args, file_path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_editors_are_classified_as_terminal() {
        assert_eq!(classify_editor("nano"), EditorKind::Terminal);
        assert_eq!(classify_editor("/usr/bin/vim"), EditorKind::Terminal);
        assert_eq!(classify_editor("nvim"), EditorKind::Terminal);
    }

    #[test]
    fn graphical_editors_are_classified_as_graphical() {
        assert_eq!(classify_editor("subl"), EditorKind::Graphical);
        assert_eq!(classify_editor("/opt/sublime_text/sublime_text"), EditorKind::Graphical);
        assert_eq!(classify_editor("code"), EditorKind::Graphical);
        assert_eq!(classify_editor("C:\\Windows\\notepad.exe"), EditorKind::Graphical);
    }

    #[test]
    fn unknown_editors_default_to_terminal() {
        assert_eq!(classify_editor("some-unknown-editor"), EditorKind::Terminal);
    }

    #[test]
    fn the_stem_drops_directories_and_the_exe_suffix() {
        assert_eq!(command_stem("C:\\Program Files\\Sublime Text\\subl.exe"), "subl");
        assert_eq!(command_stem("/usr/bin/kate"), "kate");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_deescalated_invocation_forwards_the_session_variables() {
        let environment = vec![
            ("DISPLAY".to_string(), ":0".to_string()),
            ("XAUTHORITY".to_string(), "/home/user/.Xauthority".to_string()),
        ];

        let parts = build_deescalated_invocation(
            "user",
            &environment,
            "subl",
            &["--wait".to_string()],
            "/etc/zapret/list.txt",
        );

        assert!(parts.contains(&"user".to_string()));
        assert!(parts.contains(&"env".to_string()));
        assert!(parts.contains(&"DISPLAY=:0".to_string()));
        assert!(parts.contains(&"XAUTHORITY=/home/user/.Xauthority".to_string()));
        assert_eq!(parts.last().unwrap(), "/etc/zapret/list.txt");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn the_shell_dispatch_script_quotes_the_program_and_arguments() {
        let script = build_shell_dispatch_script("subl", &["--wait".to_string()], "C:\\lists\\a.txt");
        assert!(script.contains("Shell.Application"));
        assert!(script.contains("'subl'"));
        assert!(script.contains("--wait"));
    }
}
