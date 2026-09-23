use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const COMMUNITY_ARCHIVE_URL: &str =
    "https://github.com/klondike0x/zapret2-youtube-discord/archive/refs/heads/main.zip";

pub const INGESTED_SUBDIRECTORIES: &[&str] = &["bin", "lists", "lua", "profiles"];

const DOWNLOAD_TIMEOUT_SECS: u64 = 120;
const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngestSummary {
    pub added: Vec<String>,
    pub preserved: Vec<String>,
    pub skipped: Vec<String>,
}

impl IngestSummary {
    pub fn added_count(&self) -> usize {
        self.added.len()
    }

    pub fn preserved_count(&self) -> usize {
        self.preserved.len()
    }

    pub fn render(&self) -> Vec<String> {
        let mut lines = vec![
            format!("New files added      : {}", self.added_count()),
            format!("Existing files kept  : {}", self.preserved_count()),
        ];

        if !self.skipped.is_empty() {
            lines.push(format!("Entries skipped      : {}", self.skipped.len()));
        }

        for entry in &self.added {
            lines.push(format!("  + {}", entry));
        }

        for entry in &self.preserved {
            lines.push(format!("  = {} (kept)", entry));
        }

        for entry in &self.skipped {
            lines.push(format!("  ! {} (skipped)", entry));
        }

        lines
    }
}

pub fn default_target_dir() -> PathBuf {
    let sibling = crate::config::get_app_dir().join(crate::config::ZapretEngine::Zapret2.workspace_folder());
    if sibling.is_dir() {
        return sibling;
    }

    let workspace = crate::config::ZapretEngine::Zapret2.workspace_dir();
    if workspace.is_dir() {
        return workspace;
    }

    crate::config::get_cache_dir()
}

fn is_safe_relative(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }

    path.components().all(|component| {
        matches!(
            component,
            Component::Normal(_) | Component::CurDir
        )
    })
}

pub fn strip_repository_root(raw: &str) -> Option<PathBuf> {
    let normalized = raw.replace('\\', "/");
    let path = Path::new(&normalized);

    let mut components = path.components();
    let _root = components.next()?;
    let remainder: PathBuf = components.collect();

    if remainder.as_os_str().is_empty() {
        return None;
    }

    if !is_safe_relative(&remainder) {
        return None;
    }

    Some(remainder)
}

pub fn is_ingested_path(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };

    let Component::Normal(name) = first else {
        return false;
    };

    let name = name.to_string_lossy().to_lowercase();
    INGESTED_SUBDIRECTORIES.iter().any(|entry| *entry == name)
}

fn download_archive(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .call()
        .map_err(|error| format!("could not fetch {}: {}", url, error))?;

    let mut buffer = Vec::new();
    response
        .into_reader()
        .take(MAX_ARCHIVE_BYTES)
        .read_to_end(&mut buffer)
        .map_err(|error| format!("could not read the archive body: {}", error))?;

    if buffer.is_empty() {
        return Err("the downloaded archive is empty".to_string());
    }

    Ok(buffer)
}

fn set_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn is_binary_payload(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .map(|component| match component {
            Component::Normal(name) => name.to_string_lossy().to_lowercase() == "bin",
            _ => false,
        })
        .unwrap_or(false)
}

pub fn merge_archive(bytes: &[u8], target: &Path) -> Result<IngestSummary, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("could not open the downloaded archive: {}", error))?;

    let mut summary = IngestSummary::default();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("could not read archive entry {}: {}", index, error))?;

        let raw_name = entry.name().to_string();

        let Some(relative) = strip_repository_root(&raw_name) else {
            continue;
        };

        if !is_ingested_path(&relative) {
            continue;
        }

        let display = relative.to_string_lossy().into_owned();
        let destination = target.join(&relative);

        if entry.is_dir() {
            if let Err(error) = std::fs::create_dir_all(&destination) {
                summary.skipped.push(format!("{}: {}", display, error));
            }
            continue;
        }

        if destination.exists() {
            summary.preserved.push(display);
            continue;
        }

        if let Some(parent) = destination.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                summary.skipped.push(format!("{}: {}", display, error));
                continue;
            }
        }

        let mut contents = Vec::new();
        if let Err(error) = entry.read_to_end(&mut contents) {
            summary.skipped.push(format!("{}: {}", display, error));
            continue;
        }

        if let Err(error) = std::fs::write(&destination, &contents) {
            summary.skipped.push(format!("{}: {}", display, error));
            continue;
        }

        if is_binary_payload(&relative) {
            set_executable(&destination);
        }

        summary.added.push(display);
    }

    Ok(summary)
}

pub fn sync_community_strategies(target: Option<&Path>) -> Result<IngestSummary, String> {
    let destination = match target {
        Some(path) => path.to_path_buf(),
        None => default_target_dir(),
    };

    std::fs::create_dir_all(&destination)
        .map_err(|error| format!("could not create {}: {}", destination.display(), error))?;

    let bytes = download_archive(COMMUNITY_ARCHIVE_URL)?;
    let summary = merge_archive(&bytes, &destination)?;

    crate::logger::log_error(&format!(
        "community strategy sync into {}: {} added, {} preserved, {} skipped",
        destination.display(),
        summary.added_count(),
        summary.preserved_count(),
        summary.skipped.len()
    ));

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repository_root_is_stripped() {
        let stripped = strip_repository_root("zapret2-youtube-discord-main/lists/youtube.txt");
        assert_eq!(stripped, Some(PathBuf::from("lists/youtube.txt")));
    }

    #[test]
    fn a_bare_root_entry_is_ignored() {
        assert_eq!(strip_repository_root("zapret2-youtube-discord-main/"), None);
    }

    #[test]
    fn traversal_entries_are_rejected() {
        assert_eq!(strip_repository_root("root/../../etc/passwd"), None);
        assert_eq!(strip_repository_root("root/../outside.txt"), None);
    }

    #[test]
    fn only_the_four_known_subdirectories_are_ingested() {
        assert!(is_ingested_path(Path::new("bin/nfqws2")));
        assert!(is_ingested_path(Path::new("lists/youtube.txt")));
        assert!(is_ingested_path(Path::new("lua/mutate.lua")));
        assert!(is_ingested_path(Path::new("profiles/general.txt")));
        assert!(!is_ingested_path(Path::new("README.md")));
        assert!(!is_ingested_path(Path::new("docs/manual.md")));
    }

    #[test]
    fn binaries_are_recognised_for_the_executable_bit() {
        assert!(is_binary_payload(Path::new("bin/nfqws2")));
        assert!(!is_binary_payload(Path::new("lists/youtube.txt")));
    }

    #[test]
    fn the_summary_counts_added_and_preserved_entries() {
        let summary = IngestSummary {
            added: vec!["lists/new.txt".to_string()],
            preserved: vec!["lists/kept.txt".to_string(), "profiles/mine.txt".to_string()],
            skipped: Vec::new(),
        };

        assert_eq!(summary.added_count(), 1);
        assert_eq!(summary.preserved_count(), 2);

        let rendered = summary.render();
        assert!(rendered[0].contains('1'));
        assert!(rendered[1].contains('2'));
        assert!(rendered.iter().any(|line| line.contains("+ lists/new.txt")));
        assert!(rendered.iter().any(|line| line.contains("= lists/kept.txt (kept)")));
    }
}
