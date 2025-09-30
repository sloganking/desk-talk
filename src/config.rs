use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::easy_rdev_key::PTTKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ptt_key: Option<PTTKey>,
    pub special_ptt_key: Option<u32>,
    pub device: String,
    pub use_local: bool,
    pub local_model: Option<String>,
    pub cap_first: bool,
    pub space: bool,
    pub type_chars: bool,
    pub auto_start: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub api_key: Option<String>, // Stored securely in keyring/.env, not committed to config file
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ptt_key: None,
            special_ptt_key: None,
            device: String::from("default"),
            use_local: false,
            local_model: None,
            cap_first: false,
            space: false,
            type_chars: false,
            auto_start: false,
            api_key: None,
        }
    }
}

impl AppConfig {
    fn get_config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "desk-talk", "desk-talk")
            .context("Failed to determine project directories")?;
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let contents =
                fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut config: AppConfig =
                serde_json::from_str(&contents).context("Failed to parse config file")?;

            // Load API key from keyring
            config.api_key = Self::load_api_key().ok();

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;

        // Clone so we can strip secrets before persisting JSON
        let mut config_for_disk = self.clone();
        config_for_disk.api_key = None;

        let contents =
            serde_json::to_string_pretty(&config_for_disk).context("Failed to serialize config")?;
        fs::write(&config_path, contents).context("Failed to write config file")?;

        // Save API key to keyring/.env
        if let Some(api_key) = &self.api_key {
            if let Err(e) = Self::save_api_key(api_key) {
                eprintln!(
                    "Warning: Failed to save API key to keyring: {}. Using .env file instead.",
                    e
                );
                Self::save_api_key_to_env(api_key)?;
            } else {
                // Keep .env in sync even if keyring worked (handy for debugging)
                let _ = Self::save_api_key_to_env(api_key);
            }
        }

        Ok(())
    }

    fn load_api_key() -> Result<String> {
        // Try keyring first
        if let Ok(entry) = keyring::Entry::new("desk-talk", "openai-api-key") {
            if let Ok(key) = entry.get_password() {
                return Ok(key);
            }
        }

        // Fallback to .env file
        Self::load_api_key_from_env()
    }

    fn save_api_key(api_key: &str) -> Result<()> {
        let entry = keyring::Entry::new("desk-talk", "openai-api-key")
            .context("Failed to create keyring entry")?;
        entry
            .set_password(api_key)
            .context("Failed to save API key to keyring")
    }

    fn save_api_key_to_env(api_key: &str) -> Result<()> {
        use std::env;
        let exe_dir = env::current_exe()?
            .parent()
            .context("Failed to get exe directory")?
            .to_path_buf();
        let env_path = exe_dir.join(".env");
        fs::write(&env_path, format!("OPENAI_API_KEY={}", api_key))
            .context("Failed to write .env file")?;
        Ok(())
    }

    fn load_api_key_from_env() -> Result<String> {
        use std::env;

        // Try loading from .env file next to the executable
        let exe_dir = env::current_exe()?
            .parent()
            .context("Failed to get exe directory")?
            .to_path_buf();
        let env_path = exe_dir.join(".env");

        if env_path.exists() {
            let contents = fs::read_to_string(&env_path)?;
            for line in contents.lines() {
                if let Some(key) = line.strip_prefix("OPENAI_API_KEY=") {
                    return Ok(key.to_string());
                }
            }
        }

        Err(anyhow::anyhow!("No API key found in keyring or .env file"))
    }

    pub fn delete_api_key() -> Result<()> {
        let entry = keyring::Entry::new("desk-talk", "openai-api-key")
            .context("Failed to create keyring entry")?;
        entry
            .delete_credential()
            .context("Failed to delete API key from keyring")
    }

    pub fn get_ptt_key(&self) -> Option<rdev::Key> {
        if let Some(ptt_key) = self.ptt_key {
            Some(ptt_key.into())
        } else if let Some(special_key) = self.special_ptt_key {
            Some(rdev::Key::Unknown(special_key))
        } else {
            None
        }
    }
}
