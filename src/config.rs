use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::AppResult;

const APP_DIR: &str = "hugdocker";
const LEGACY_APP_DIR: &str = "dockerctl";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub docker: DockerConfig,
    #[serde(default)]
    pub groups: GroupConfig,
    #[serde(default)]
    pub profiles: ProfileConfig,
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            docker: DockerConfig::default(),
            groups: GroupConfig::default(),
            profiles: ProfileConfig::default(),
            tui: TuiConfig::default(),
            safety: SafetyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockerConfig {
    pub context: Option<String>,
    pub host: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupConfig {
    #[serde(default)]
    pub exact: BTreeMap<String, String>,
    #[serde(default)]
    pub prefix: Vec<(String, String)>,
    #[serde(default)]
    pub image_prefix: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub groups: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    pub refresh_ms: u64,
    pub log_tail: usize,
    pub default_filter: String,
    pub theme: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            refresh_ms: 2_000,
            log_tail: 200,
            default_filter: String::new(),
            theme: "cockpit".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    Cockpit,
    Industrial,
    Signal,
    Ocean,
}

pub fn parse_theme(raw: &str) -> ThemeName {
    match raw.trim().to_ascii_lowercase().as_str() {
        "cockpit" => ThemeName::Cockpit,
        "signal" => ThemeName::Signal,
        "ocean" => ThemeName::Ocean,
        _ => ThemeName::Industrial,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub typed_confirmation: bool,
    pub allow_yes_for_purge: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            typed_confirmation: true,
            allow_yes_for_purge: false,
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    if let Some(base) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(base).join(APP_DIR).join("config.toml"));
    }
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join(APP_DIR)
            .join("config.toml")
    })
}

fn legacy_config_path() -> Option<PathBuf> {
    if let Some(base) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(base).join(LEGACY_APP_DIR).join("config.toml"));
    }
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join(LEGACY_APP_DIR)
            .join("config.toml")
    })
}

pub fn state_dir_path() -> Option<PathBuf> {
    if let Some(base) = env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(base).join(APP_DIR));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state").join(APP_DIR))
}

pub fn audit_log_path() -> Option<PathBuf> {
    state_dir_path().map(|path| path.join("audit.log"))
}

pub fn timeline_log_path() -> Option<PathBuf> {
    state_dir_path().map(|path| path.join("timeline.jsonl"))
}

pub fn load_config() -> AppConfig {
    load_config_from_paths(config_path(), legacy_config_path())
}

fn load_config_from_paths(primary: Option<PathBuf>, legacy: Option<PathBuf>) -> AppConfig {
    let content = primary
        .and_then(|path| fs::read_to_string(path).ok())
        .or_else(|| legacy.and_then(|path| fs::read_to_string(path).ok()));
    let Some(content) = content else {
        return AppConfig::default();
    };
    toml::from_str::<AppConfig>(&content).unwrap_or_else(|_| AppConfig {
        docker: DockerConfig::default(),
        groups: parse_group_config(&content),
        ..AppConfig::default()
    })
}

pub fn write_default_config(path: &PathBuf) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(&AppConfig::default())?)?;
    Ok(())
}

pub fn parse_group_config(content: &str) -> GroupConfig {
    enum Section {
        None,
        Exact,
        Prefix,
        ImagePrefix,
    }

    let mut config = GroupConfig::default();
    let mut section = Section::None;
    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let section_name = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_ascii_lowercase();
            section = match section_name.as_str() {
                "groups.exact" | "group_exact" | "standalone_group_exact" => Section::Exact,
                "groups.prefix" | "group_prefix" | "standalone_group_prefix" => Section::Prefix,
                "groups.image_prefix" | "group_image_prefix" | "standalone_group_image_prefix" => {
                    Section::ImagePrefix
                }
                _ => Section::None,
            };
            continue;
        }

        let Some((left, right)) = line.split_once('=') else {
            continue;
        };
        let key = parse_config_atom(left);
        let value = parse_config_atom(right);
        if key.is_empty() || value.is_empty() {
            continue;
        }

        match section {
            Section::Exact => {
                config.exact.insert(key, value);
            }
            Section::Prefix => config.prefix.push((key, value)),
            Section::ImagePrefix => config.image_prefix.push((key, value)),
            Section::None => {}
        }
    }
    config
}

pub fn parse_config_atom(raw: &str) -> String {
    let mut value = raw.trim().to_string();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value = value[1..value.len() - 1].to_string();
    }
    value.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("hugdocker-config-test-{name}-{nonce}"))
    }

    #[test]
    fn load_config_falls_back_to_legacy_dockerctl_path() {
        let legacy = temp_path("legacy").join("config.toml");
        fs::create_dir_all(legacy.parent().expect("parent")).expect("legacy dir");
        fs::write(&legacy, "[tui]\ntheme = \"signal\"\n").expect("legacy config");

        let config = load_config_from_paths(Some(temp_path("missing")), Some(legacy.clone()));

        assert_eq!(config.tui.theme, "signal");
        let _ = fs::remove_dir_all(legacy.parent().expect("parent"));
    }

    #[test]
    fn load_config_prefers_hugdocker_path_over_legacy_path() {
        let primary = temp_path("primary").join("config.toml");
        let legacy = temp_path("legacy").join("config.toml");
        fs::create_dir_all(primary.parent().expect("parent")).expect("primary dir");
        fs::create_dir_all(legacy.parent().expect("parent")).expect("legacy dir");
        fs::write(&primary, "[tui]\ntheme = \"ocean\"\n").expect("primary config");
        fs::write(&legacy, "[tui]\ntheme = \"signal\"\n").expect("legacy config");

        let config = load_config_from_paths(Some(primary.clone()), Some(legacy.clone()));

        assert_eq!(config.tui.theme, "ocean");
        let _ = fs::remove_dir_all(primary.parent().expect("parent"));
        let _ = fs::remove_dir_all(legacy.parent().expect("parent"));
    }
}
