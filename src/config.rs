//! java: ConfigSchema — the Gson-pretty config file, `config/enchdesc.json`,
//! camelCase fields, load-or-create and always save, same defaults.

use std::path::{Path, PathBuf};

/// java: Constants.MOD_ID.
pub const MOD_ID: &str = "enchdesc";

/// java: ConfigSchema's fields, same defaults, the original's camelCase names
/// in the file.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "enableMod")]
    pub enable_mod: bool,
    #[serde(rename = "onlyDisplayOnBooks")]
    pub only_display_on_books: bool,
    #[serde(rename = "onlyDisplayInEnchantingTable")]
    pub only_display_in_enchanting_table: bool,
    #[serde(rename = "requireKeybindPress")]
    pub require_keybind_press: bool,
    #[serde(rename = "indentSize")]
    pub indent_size: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_mod: true,
            only_display_on_books: false,
            only_display_in_enchanting_table: false,
            require_keybind_press: false,
            indent_size: 0,
        }
    }
}

/// java: the game dir the host sets (`XI_GAME_DIR`), the config under
/// `config/` next to the original's file name.
pub fn game_dir() -> PathBuf {
    std::env::var("XI_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    game_dir().join("config").join(format!("{MOD_ID}.json"))
}

/// java: ConfigSchema.load — an existing file parses, a broken one falls back
/// to the defaults (upstream logs the failure and keeps the defaults), and a
/// missing one is the defaults until the save writes it.
pub fn load(path: &PathBuf) -> Config {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// java: ConfigSchema's save — Gson pretty printing writes the file every
/// run, creating the config directory first.
pub fn save(path: &PathBuf, config: &Config) -> bool {
    serde_json::to_string_pretty(config).ok().is_some_and(|text| {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent).is_ok()
            && std::fs::write(path, text).is_ok()
    })
}
