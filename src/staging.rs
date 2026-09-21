use std::fs;
use std::path::{Path, PathBuf};

pub const STAGED_DIRS: &[&str] = &["lists", "custom", "lua", "bin", "presets"];

#[cfg(any(target_os = "linux", test))]
const RUN_BASE: &str = "/run/zapret";

#[cfg(any(target_os = "linux", test))]
const TMP_BASE: &str = "/tmp/zapret-runtime";

const DIR_MODE: u32 = 0o755;
const SHARED_FILE_MODE: u32 = 0o666;

#[cfg(any(target_os = "linux", test))]
const FILE_MODE: u32 = 0o644;

#[cfg(any(target_os = "linux", test))]
const TEXT_EXTENSION: &str = "txt";

const LIST_PREFIXES: &[&str] = &[
    "--hostlist=",
    "--hostlist-exclude=",
    "--hostlist-auto=",
    "--ipset=",
    "--ipset-exclude=",
];

const AUTO_LIST_PREFIX: &str = "--hostlist-auto=";

pub fn sanitize(value: &str) -> String {
    value.replace(&['\r', '\n'][..], "").trim().to_string()
}

pub fn sanitize_all(values: &[String]) -> Vec<String> {
    values.iter().map(|value| sanitize(value)).collect()
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|e| format!("cannot set mode on {}: {}", path.display(), e))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn remove_any(path: &Path) -> Result<(), String> {
    let meta = match path.symlink_metadata() {
        Ok(meta) => meta,
        Err(_) => return Ok(()),
    };

    let result = if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };

    result.map_err(|e| format!("cannot remove {}: {}", path.display(), e))
}

#[cfg(any(target_os = "linux", test))]
fn reset_dir(path: &Path) -> Result<(), String> {
    remove_any(path)?;
    fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {}", path.display(), e))?;
    set_mode(path, DIR_MODE)
}

#[cfg(any(target_os = "linux", test))]
fn select_base() -> Result<PathBuf, String> {
    let mut last_error = String::from("no writable runtime directory");

    for candidate in [RUN_BASE, TMP_BASE] {
        let path = PathBuf::from(candidate);
        let parent_ready = path.parent().map(|p| p.is_dir()).unwrap_or(false);
        if !parent_ready {
            last_error = format!("{} is not available", candidate);
            continue;
        }
        match reset_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) => last_error = e,
        }
    }

    Err(last_error)
}

#[cfg(any(target_os = "linux", test))]
fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(TEXT_EXTENSION))
        .unwrap_or(false)
}

#[cfg(any(target_os = "linux", test))]
fn copy_file(src: &Path, dst: &Path) -> Result<(), String> {
    if is_text_file(src) {
        if let Ok(text) = fs::read_to_string(src) {
            fs::write(dst, text.replace("\r\n", "\n"))
                .map_err(|e| format!("cannot write {}: {}", dst.display(), e))?;
            return set_mode(dst, FILE_MODE);
        }
    }

    fs::copy(src, dst).map_err(|e| format!("cannot copy {}: {}", src.display(), e))?;
    set_mode(dst, FILE_MODE)
}

#[cfg(any(target_os = "linux", test))]
fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("cannot create {}: {}", dst.display(), e))?;
    set_mode(dst, DIR_MODE)?;

    let entries = fs::read_dir(src).map_err(|e| format!("cannot read {}: {}", src.display(), e))?;

    for entry in entries.flatten() {
        let source = entry.path();
        let target = dst.join(entry.file_name());
        let Ok(meta) = fs::metadata(&source) else {
            continue;
        };

        if meta.is_dir() {
            copy_tree(&source, &target)?;
        } else if meta.is_file() {
            copy_file(&source, &target)?;
        }
    }

    Ok(())
}

fn strip_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn looks_like_path(value: &str) -> bool {
    let candidate = value.trim_start_matches('!');
    let candidate = candidate.strip_prefix("./").unwrap_or(candidate);

    if candidate.starts_with('/') {
        return true;
    }

    STAGED_DIRS.iter().any(|dir| {
        candidate
            .strip_prefix(*dir)
            .map(|rest| rest.starts_with('/'))
            .unwrap_or(false)
    })
}

#[derive(Debug)]
pub struct Staging {
    root: PathBuf,
    workspace: PathBuf,
    active: bool,
    writeback: Vec<PathBuf>,
}

impl Staging {
    #[cfg(target_os = "linux")]
    pub fn create(workspace: &Path) -> Result<Self, String> {
        let workspace = fs::canonicalize(workspace)
            .map_err(|e| format!("cannot resolve workspace {}: {}", workspace.display(), e))?;

        let root = select_base()?;
        let mut staged = 0usize;

        for name in STAGED_DIRS {
            let source = workspace.join(name);
            if !source.is_dir() {
                continue;
            }
            copy_tree(&source, &root.join(name))?;
            staged += 1;
        }

        if staged == 0 {
            let _ = fs::remove_dir_all(&root);
            return Ok(Self::passthrough(workspace));
        }

        Ok(Self {
            root,
            workspace,
            active: true,
            writeback: Vec::new(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn create(workspace: &Path) -> Result<Self, String> {
        let resolved = fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
        Ok(Self::passthrough(resolved))
    }

    fn passthrough(workspace: PathBuf) -> Self {
        Self {
            root: workspace.clone(),
            workspace,
            active: false,
            writeback: Vec::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn stage_args(&mut self, args: &[String]) -> Vec<String> {
        args.iter().map(|arg| self.stage_arg(arg)).collect()
    }

    fn stage_arg(&mut self, arg: &str) -> String {
        let arg = sanitize(arg);
        let Some(split) = arg.find('=') else {
            return arg;
        };

        let (head, tail) = arg.split_at(split + 1);
        let flag = head.to_string();
        let value = tail.to_string();
        let shared = flag == AUTO_LIST_PREFIX;

        if value.contains('@') {
            return format!("{}{}", flag, self.map_at_refs(&value));
        }

        if !LIST_PREFIXES.contains(&flag.as_str()) && !looks_like_path(strip_quotes(&value)) {
            return arg;
        }

        match self.map_value(&value, shared) {
            Some(mapped) => format!("{}{}", flag, mapped),
            None => arg,
        }
    }

    fn map_value(&mut self, value: &str, shared: bool) -> Option<String> {
        let stripped = strip_quotes(value);
        let (negation, raw) = match stripped.strip_prefix('!') {
            Some(rest) => ("!", rest),
            None => ("", stripped),
        };

        let mapped = self.map_path(raw, shared)?;
        Some(format!("{}{}", negation, mapped))
    }

    fn map_at_refs(&mut self, value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let mut rest = value;

        while let Some(position) = rest.find('@') {
            out.push_str(&rest[..position]);
            out.push('@');

            let after = &rest[position + 1..];
            let end = after
                .find(&[':', ',', ' ', '"', '\''][..])
                .unwrap_or(after.len());
            let (raw, tail) = after.split_at(end);

            match self.map_path(raw, false) {
                Some(mapped) => out.push_str(&mapped),
                None => out.push_str(raw),
            }

            rest = tail;
        }

        out.push_str(rest);
        out
    }

    fn map_path(&mut self, raw: &str, shared: bool) -> Option<String> {
        if !self.active || raw.is_empty() {
            return None;
        }

        let resolved = crate::strategy::resolve_within(&self.workspace, raw)?;
        let relative = resolved.strip_prefix(&self.workspace).ok()?;
        let head = relative.components().next()?.as_os_str().to_str()?;
        if !STAGED_DIRS.contains(&head) {
            return None;
        }

        let staged = self.root.join(relative);
        if !staged.exists() {
            if !shared {
                return None;
            }
            if let Some(parent) = staged.parent() {
                fs::create_dir_all(parent).ok()?;
                set_mode(parent, DIR_MODE).ok()?;
            }
            fs::write(&staged, "").ok()?;
        }

        if shared {
            set_mode(&staged, SHARED_FILE_MODE).ok()?;
            let owned = relative.to_path_buf();
            if !self.writeback.contains(&owned) {
                self.writeback.push(owned);
            }
        }

        Some(staged.to_string_lossy().into_owned())
    }

    fn restore_writeback(&self) {
        for relative in &self.writeback {
            let from = self.root.join(relative);
            let to = self.workspace.join(relative);
            if !from.is_file() {
                continue;
            }
            if let Some(parent) = to.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(&from, &to);
        }
    }

    pub fn dispose(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        self.restore_writeback();
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zapret_staging_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("lists")).unwrap();
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::create_dir_all(dir.join("lua")).unwrap();
        dir
    }

    fn staging_for(workspace: &Path) -> Staging {
        let root = workspace.join(".staged");
        fs::create_dir_all(&root).unwrap();
        for name in ["lists", "bin", "lua"] {
            let source = workspace.join(name);
            if source.is_dir() {
                copy_tree(&source, &root.join(name)).unwrap();
            }
        }
        Staging {
            root,
            workspace: fs::canonicalize(workspace).unwrap(),
            active: true,
            writeback: Vec::new(),
        }
    }

    fn owned(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn strips_carriage_returns_and_padding() {
        assert_eq!(sanitize("  --ipset=lists/a.txt\r\n"), "--ipset=lists/a.txt");
        assert_eq!(sanitize("--name=youtube.com (interface)\r"), "--name=youtube.com (interface)");
    }

    #[test]
    fn rewrites_list_arguments_into_the_staging_root() {
        let ws = workspace("lists");
        fs::write(ws.join("lists/ipset-ru.txt"), "1.1.1.1\n").unwrap();
        let mut staging = staging_for(&ws);

        let args = staging.stage_args(&owned(&["--ipset=lists/ipset-ru.txt\r"]));
        let expected = format!("--ipset={}", staging.root().join("lists/ipset-ru.txt").display());
        assert_eq!(args[0], expected);

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn rewrites_absolute_workspace_paths() {
        let ws = workspace("absolute");
        fs::write(ws.join("lists/list-general.txt"), "example.com\n").unwrap();
        let mut staging = staging_for(&ws);

        let canonical = fs::canonicalize(ws.join("lists/list-general.txt")).unwrap();
        let args = staging.stage_args(&owned(&[&format!("--hostlist={}", canonical.display())]));
        let expected = format!("--hostlist={}", staging.root().join("lists/list-general.txt").display());
        assert_eq!(args[0], expected);

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn rewrites_at_prefixed_blob_references() {
        let ws = workspace("blobs");
        fs::write(ws.join("bin/quic.bin"), b"x").unwrap();
        fs::write(ws.join("lua/zapret-lib.lua"), "return {}\n").unwrap();
        let mut staging = staging_for(&ws);

        let args = staging.stage_args(&owned(&["--blob=quic:@bin/quic.bin", "--lua-init=@lua/zapret-lib.lua"]));
        assert_eq!(
            args[0],
            format!("--blob=quic:@{}", staging.root().join("bin/quic.bin").display())
        );
        assert_eq!(
            args[1],
            format!("--lua-init=@{}", staging.root().join("lua/zapret-lib.lua").display())
        );

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn leaves_non_path_values_untouched() {
        let ws = workspace("plain");
        let mut staging = staging_for(&ws);

        let args = staging.stage_args(&owned(&[
            "--filter-tcp=80,443",
            "--hostlist-domains=a.com,b.com",
            "--name=youtube.com (interface)",
            "--new",
        ]));
        assert_eq!(
            args,
            owned(&[
                "--filter-tcp=80,443",
                "--hostlist-domains=a.com,b.com",
                "--name=youtube.com (interface)",
                "--new",
            ])
        );

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn never_maps_paths_outside_the_workspace() {
        let ws = workspace("escape");
        let mut staging = staging_for(&ws);

        let args = staging.stage_args(&owned(&["--ipset=/etc/hosts", "--hostlist=../outside.txt"]));
        assert_eq!(args, owned(&["--ipset=/etc/hosts", "--hostlist=../outside.txt"]));

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn auto_hostlists_are_created_and_written_back() {
        let ws = workspace("auto");
        let mut staging = staging_for(&ws);

        let args = staging.stage_args(&owned(&["--hostlist-auto=lists/auto.txt"]));
        let staged = staging.root().join("lists/auto.txt");
        assert_eq!(args[0], format!("--hostlist-auto={}", staged.display()));
        assert!(staged.is_file());

        fs::write(&staged, "learned.example\n").unwrap();
        staging.restore_writeback();
        assert_eq!(
            fs::read_to_string(ws.join("lists/auto.txt")).unwrap(),
            "learned.example\n"
        );

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn passthrough_staging_rewrites_nothing() {
        let ws = workspace("passthrough");
        let mut staging = Staging::passthrough(fs::canonicalize(&ws).unwrap());

        let args = staging.stage_args(&owned(&["--ipset=lists/a.txt"]));
        assert_eq!(args, owned(&["--ipset=lists/a.txt"]));
        assert!(!staging.is_active());

        let _ = fs::remove_dir_all(&ws);
    }

    #[test]
    fn copying_normalizes_windows_line_endings() {
        let ws = workspace("crlf");
        fs::write(ws.join("lists/crlf.txt"), "a.com\r\nb.com\r\n").unwrap();
        let staging = staging_for(&ws);

        assert_eq!(
            fs::read_to_string(staging.root().join("lists/crlf.txt")).unwrap(),
            "a.com\nb.com\n"
        );

        let _ = fs::remove_dir_all(&ws);
    }
}
