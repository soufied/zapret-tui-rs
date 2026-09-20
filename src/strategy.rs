pub mod discovery;
pub mod embedded;
pub mod parser;
pub mod refs;

pub use discovery::{get_strategies, get_strategies_for, zapret2_presets};
pub use embedded::{ensure_custom_strategies, repo_dir};
#[allow(unused_imports)]
pub use parser::{ParsedPreset, ParsedStrategy};
pub use parser::{parse_bat_file, parse_preset_content, parse_zapret2_preset, GameFilterPorts};
pub use refs::{collect_list_refs, list_file_present, missing_list_refs, placeholder_content, resolve_within};
