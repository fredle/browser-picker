//! Small persisted app preferences, separate from `rules.json` since it's a
//! different shape (one object, not a list).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// Resolve known email "safe link" wrappers (Mimecast, SafeLinks, ...) to
    /// their real destination before showing the picker. Off by default:
    /// resolving sends a request to the wrapper's server, which registers as
    /// a click against the original link.
    #[serde(default)]
    pub unwrap_wrapped_links: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            unwrap_wrapped_links: false,
        }
    }
}

fn settings_path() -> std::path::PathBuf {
    crate::rules::rules_dir().join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) {
    let _ = std::fs::create_dir_all(crate::rules::rules_dir());
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(), json);
    }
}
