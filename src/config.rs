use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// User configuration, persisted as TOML at `~/.config/tusic/config.toml`.
///
/// All `scan_dirs` are scanned for music. The **first** directory is also the
/// download target, so downloaded songs land there and show up in the library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Absolute directories scanned for music. The first one is the download
    /// target. Defaults to the OS music directory (e.g. `/home/user/Music`).
    pub scan_dirs: Vec<String>,
    /// When true, the directory the program was launched from is used as the
    /// primary (download) directory, in addition to `scan_dirs`.
    pub use_current_dir: bool,
}

impl Default for Config {
    fn default() -> Self {
        let scan_dirs = dirs::audio_dir()
            .map(|p| vec![p.join("tusic").to_string_lossy().into_owned()])
            .unwrap_or_default();

        Self {
            scan_dirs,
            use_current_dir: false,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tusic")
            .join("config.toml")
    }

    /// Load the config from disk, falling back to defaults if missing or invalid.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// All directories to scan, in priority order, deduplicated. The first
    /// entry is the primary (download) directory.
    pub fn resolved_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = Vec::new();

        if self.use_current_dir {
            if let Ok(cwd) = std::env::current_dir() {
                dirs.push(cwd);
            }
        }

        for d in &self.scan_dirs {
            if !d.is_empty() {
                dirs.push(PathBuf::from(d));
            }
        }

        dirs.dedup();

        // Ensure the directories exist so scanning/downloading don't fail.
        for d in &dirs {
            if !d.exists() {
                std::fs::create_dir_all(d).ok();
            }
        }

        dirs
    }

    /// The primary directory where downloads are saved.
    pub fn download_dir(&self) -> PathBuf {
        self.resolved_dirs()
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let cfg = Config {
            scan_dirs: vec!["/tmp/songs".to_string(), "/tmp/more".to_string()],
            use_current_dir: true,
        };
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // An empty file should deserialize to the default config.
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn download_dir_is_first_scan_dir() {
        let cfg = Config {
            scan_dirs: vec!["/tmp/a".to_string(), "/tmp/b".to_string()],
            use_current_dir: false,
        };
        assert_eq!(cfg.download_dir(), PathBuf::from("/tmp/a"));
    }
}
