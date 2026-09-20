use std::path::{Component, Path, PathBuf};

pub const PLACEHOLDER_HOST: &str = "placeholder.zapret-rust.invalid";
pub const PLACEHOLDER_IP: &str = "203.0.113.113/32";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRefKind {
    HostInclude,
    HostExclude,
    HostAuto,
    IpInclude,
    IpExclude,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRef {
    pub kind: ListRefKind,
    pub path: String,
}

const REF_PREFIXES: &[(&str, ListRefKind)] = &[
    ("--hostlist=", ListRefKind::HostInclude),
    ("--hostlist-exclude=", ListRefKind::HostExclude),
    ("--hostlist-auto=", ListRefKind::HostAuto),
    ("--ipset=", ListRefKind::IpInclude),
    ("--ipset-exclude=", ListRefKind::IpExclude),
];

pub fn collect_list_refs(args: &[String]) -> Vec<ListRef> {
    let mut refs: Vec<ListRef> = Vec::new();

    for arg in args {
        for (prefix, kind) in REF_PREFIXES {
            let Some(value) = arg.strip_prefix(*prefix) else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                break;
            }
            let reference = ListRef {
                kind: *kind,
                path: value.to_string(),
            };
            if !refs.contains(&reference) {
                refs.push(reference);
            }
            break;
        }
    }

    refs
}

pub fn resolve_within(root: &Path, raw: &str) -> Option<PathBuf> {
    let candidate = Path::new(raw);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };

    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }

    if out.starts_with(root) {
        Some(out)
    } else {
        None
    }
}

fn gz_sibling(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".gz");
    PathBuf::from(name)
}

pub fn list_file_present(path: &Path) -> bool {
    path.exists() || gz_sibling(path).exists()
}

pub fn placeholder_content(kind: ListRefKind, file_name: &str) -> String {
    match kind {
        ListRefKind::HostInclude => format!("{}\n", PLACEHOLDER_HOST),
        ListRefKind::IpInclude if file_name != crate::ipset::IPSET_ALL_FILE => format!("{}\n", PLACEHOLDER_IP),
        _ => String::new(),
    }
}

pub fn missing_list_refs(workspace: &Path, args: &[String]) -> Vec<ListRef> {
    let Ok(root) = std::fs::canonicalize(workspace) else {
        return Vec::new();
    };

    collect_list_refs(args)
        .into_iter()
        .filter(|reference| match resolve_within(&root, &reference.path) {
            Some(path) => !list_file_present(&path),
            None => false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn collects_every_list_flavour() {
        let refs = collect_list_refs(&args(&[
            "--filter-tcp=443",
            "--hostlist=lists/whatsapp.txt",
            "--hostlist-exclude=lists/list-exclude.txt",
            "--hostlist-auto=lists/auto.txt",
            "--ipset=lists/ipset-all.txt",
            "--ipset-exclude=lists/ipset-exclude.txt",
        ]));
        assert_eq!(refs.len(), 5);
        assert_eq!(refs[0].kind, ListRefKind::HostInclude);
        assert_eq!(refs[1].kind, ListRefKind::HostExclude);
        assert_eq!(refs[2].kind, ListRefKind::HostAuto);
        assert_eq!(refs[3].kind, ListRefKind::IpInclude);
        assert_eq!(refs[4].kind, ListRefKind::IpExclude);
    }

    #[test]
    fn ignores_inline_domain_and_ip_options() {
        let refs = collect_list_refs(&args(&[
            "--hostlist-domains=a.com,b.com",
            "--hostlist-exclude-domains=c.com",
            "--ipset-ip=1.1.1.1",
            "--ipset-exclude-ip=2.2.2.2",
            "--lua-init=@lua/zapret-lib.lua",
            "--blob=quic:@bin/quic.bin",
        ]));
        assert!(refs.is_empty());
    }

    #[test]
    fn deduplicates_and_skips_empty_values() {
        let refs = collect_list_refs(&args(&[
            "--hostlist=lists/a.txt",
            "--hostlist=lists/a.txt",
            "--hostlist=",
        ]));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "lists/a.txt");
    }

    #[test]
    fn keeps_same_path_with_different_kinds() {
        let refs = collect_list_refs(&args(&["--hostlist=lists/a.txt", "--hostlist-exclude=lists/a.txt"]));
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn resolves_relative_paths_inside_root() {
        let root = Path::new("/ws");
        assert_eq!(
            resolve_within(root, "lists/whatsapp.txt"),
            Some(PathBuf::from("/ws/lists/whatsapp.txt"))
        );
        assert_eq!(
            resolve_within(root, "./lists/../lists/x.txt"),
            Some(PathBuf::from("/ws/lists/x.txt"))
        );
    }

    #[test]
    fn rejects_paths_escaping_root() {
        let root = Path::new("/ws");
        assert_eq!(resolve_within(root, "../etc/passwd"), None);
        assert_eq!(resolve_within(root, "lists/../../x.txt"), None);
        assert_eq!(resolve_within(root, "/etc/hosts"), None);
    }

    #[test]
    fn include_lists_get_a_never_matching_entry() {
        assert_eq!(
            placeholder_content(ListRefKind::HostInclude, "whatsapp.txt"),
            format!("{}\n", PLACEHOLDER_HOST)
        );
        assert_eq!(
            placeholder_content(ListRefKind::IpInclude, "whatsapp-ips.txt"),
            format!("{}\n", PLACEHOLDER_IP)
        );
    }

    #[test]
    fn exclude_auto_and_managed_ipset_stay_empty() {
        assert!(placeholder_content(ListRefKind::HostExclude, "x.txt").is_empty());
        assert!(placeholder_content(ListRefKind::IpExclude, "x.txt").is_empty());
        assert!(placeholder_content(ListRefKind::HostAuto, "x.txt").is_empty());
        assert!(placeholder_content(ListRefKind::IpInclude, crate::ipset::IPSET_ALL_FILE).is_empty());
    }

    #[test]
    fn gz_sibling_counts_as_present() {
        let dir = std::env::temp_dir().join(format!("zapret_refs_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let plain = dir.join("only_gz.txt");
        assert!(!list_file_present(&plain));
        std::fs::write(dir.join("only_gz.txt.gz"), b"x").unwrap();
        assert!(list_file_present(&plain));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
