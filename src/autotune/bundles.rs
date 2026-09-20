pub const CORE_LISTS: &[&str] = &[
    "discord.txt",
    "discord-updates.txt",
    "russia-discord.txt",
    "youtube.txt",
    "youtube_v2.txt",
    "youtubeQ.txt",
    "youtubeGV.txt",
    "googlevideo.txt",
    "russia-youtube.txt",
    "russia-youtubeQ.txt",
    "list-google.txt",
];

pub const SOCIAL_LISTS: &[&str] = &[
    "facebook.txt",
    "instagram.txt",
    "twitter.txt",
    "telegram.txt",
    "whatsapp.txt",
    "twitch.txt",
    "soundcloud.txt",
    "reddit.txt",
    "tiktok.txt",
    "vk.txt",
];

pub const CUSTOM_LISTS: &[&str] = &[
    "list-other.txt",
    "other.txt",
    "list-general.txt",
    "list-general-user.txt",
    "myhostlist.txt",
    "mycdnlist.txt",
    "custom.txt",
];

const EXCLUDED_SUBSTRINGS: &[&str] = &[
    "ipset",
    "exclude",
    "blacklist",
    "allzone",
    "netrogat",
    "cloudflare",
    "zapretkvn",
];

const EXCLUDED_NAMES: &[&str] = &["russia-blacklist.txt"];

pub fn is_domain_list(name: &str) -> bool {
    let lower = name.to_lowercase();
    if !lower.ends_with(".txt") || lower.starts_with('_') {
        return false;
    }
    if EXCLUDED_NAMES.contains(&lower.as_str()) {
        return false;
    }
    !EXCLUDED_SUBSTRINGS.iter().any(|frag| lower.contains(frag))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetBundle {
    CoreMedia,
    WithSocial,
    WithCustom,
    Manual,
}

impl Default for TargetBundle {
    fn default() -> Self {
        Self::CoreMedia
    }
}

impl TargetBundle {
    pub fn all() -> &'static [Self] {
        &[Self::CoreMedia, Self::WithSocial, Self::WithCustom, Self::Manual]
    }

    pub fn index(self) -> usize {
        match self {
            Self::CoreMedia => 0,
            Self::WithSocial => 1,
            Self::WithCustom => 2,
            Self::Manual => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::WithSocial,
            2 => Self::WithCustom,
            3 => Self::Manual,
            _ => Self::CoreMedia,
        }
    }

    pub fn cycle(self, forward: bool) -> Self {
        let len = Self::all().len();
        let current = self.index();
        let next = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        Self::from_index(next)
    }

    pub fn is_manual(self) -> bool {
        matches!(self, Self::Manual)
    }

    pub fn label(self) -> String {
        match self {
            Self::CoreMedia => rust_i18n::t!("autotune_bundle_core").into_owned(),
            Self::WithSocial => rust_i18n::t!("autotune_bundle_social").into_owned(),
            Self::WithCustom => rust_i18n::t!("autotune_bundle_custom").into_owned(),
            Self::Manual => rust_i18n::t!("autotune_bundle_manual").into_owned(),
        }
    }

    pub fn candidates(self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        match self {
            Self::CoreMedia => out.extend_from_slice(CORE_LISTS),
            Self::WithSocial => {
                out.extend_from_slice(CORE_LISTS);
                out.extend_from_slice(SOCIAL_LISTS);
            }
            Self::WithCustom => {
                out.extend_from_slice(CORE_LISTS);
                out.extend_from_slice(SOCIAL_LISTS);
                out.extend_from_slice(CUSTOM_LISTS);
            }
            Self::Manual => {}
        }
        out
    }

    pub fn resolve(self, available: &[String]) -> Vec<String> {
        if self.is_manual() {
            return Vec::new();
        }

        let mut out: Vec<String> = Vec::new();
        for candidate in self.candidates() {
            let found = available
                .iter()
                .find(|name| name.eq_ignore_ascii_case(candidate))
                .cloned();
            if let Some(name) = found {
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipset_and_exclusion_lists_are_not_probe_targets() {
        assert!(is_domain_list("discord.txt"));
        assert!(is_domain_list("youtube.txt"));
        assert!(!is_domain_list("ipset-discord.txt"));
        assert!(!is_domain_list("russia-discord-ipset.txt"));
        assert!(!is_domain_list("list-exclude.txt"));
        assert!(!is_domain_list("russia-blacklist.txt"));
        assert!(!is_domain_list("netrogat.txt"));
        assert!(!is_domain_list("cloudflare-ipset.txt"));
        assert!(!is_domain_list("_internal.txt"));
        assert!(!is_domain_list("notes.md"));
    }

    #[test]
    fn bundles_accumulate_in_order() {
        assert!(TargetBundle::CoreMedia.candidates().contains(&"discord.txt"));
        assert!(!TargetBundle::CoreMedia.candidates().contains(&"twitter.txt"));
        assert!(TargetBundle::WithSocial.candidates().contains(&"twitter.txt"));
        assert!(!TargetBundle::WithSocial.candidates().contains(&"other.txt"));
        assert!(TargetBundle::WithCustom.candidates().contains(&"other.txt"));
        assert!(TargetBundle::Manual.candidates().is_empty());
    }

    #[test]
    fn resolve_keeps_only_present_files() {
        let available = vec![
            "discord.txt".to_string(),
            "youtube.txt".to_string(),
            "zzz-unknown.txt".to_string(),
        ];
        let resolved = TargetBundle::CoreMedia.resolve(&available);
        assert_eq!(resolved, vec!["discord.txt".to_string(), "youtube.txt".to_string()]);
        assert!(TargetBundle::Manual.resolve(&available).is_empty());
    }

    #[test]
    fn cycling_wraps_both_directions() {
        assert_eq!(TargetBundle::CoreMedia.cycle(false), TargetBundle::Manual);
        assert_eq!(TargetBundle::Manual.cycle(true), TargetBundle::CoreMedia);
    }
}
