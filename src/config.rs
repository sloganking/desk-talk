use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::easy_rdev_key::PTTKey;
use uuid::Uuid;
#[cfg(windows)]
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ptt_key: Option<PTTKey>,
    pub special_ptt_key: Option<u32>,
    pub device: String,
    pub use_local: bool,
    pub local_model: Option<String>,
    pub cap_first: bool,
    pub space: bool,
    pub type_chars: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(skip_serializing, default)]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_expiration: Option<String>,
    #[serde(default)]
    pub trial_started: bool,
    #[serde(default = "AppConfig::default_machine_id")]
    pub machine_id: String,
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
            license_key: None,
            license_plan: None,
            license_id: None,
            trial_expiration: None,
            trial_started: false,
            machine_id: AppConfig::default_machine_id(),
        }
    }
}

impl AppConfig {
    fn default_machine_id() -> String {
        Uuid::new_v4().to_string()
    }

    fn get_config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "desk-talk", "desk-talk")
            .context("Failed to determine project directories")?;
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        let mut config = if config_path.exists() {
            let contents =
                fs::read_to_string(&config_path).context("Failed to read config file")?;
            let mut cfg: AppConfig =
                serde_json::from_str(&contents).context("Failed to parse config file")?;
            if cfg.machine_id.is_empty() {
                cfg.machine_id = AppConfig::default_machine_id();
            }
            cfg
        } else {
            AppConfig::default()
        };

        // Load API key from keyring
        config.api_key = Self::load_api_key().ok();

        Ok(config)
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

        #[cfg(windows)]
        {
            if self.auto_start {
                Self::enable_autostart()?;
            } else {
                Self::disable_autostart()?;
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

    #[cfg(windows)]
    fn enable_autostart() -> Result<()> {
        use std::env;

        let exe_path = env::current_exe()?;
        let exe_str = exe_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid exe path"))?;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let (key, _) = hkcu.create_subkey(path)?;
        key.set_value("DeskTalk", &exe_str)?;
        Ok(())
    }

    #[cfg(windows)]
    fn disable_autostart() -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        if let Ok(key) = hkcu.open_subkey(path) {
            let _ = key.delete_value("DeskTalk");
        }
        Ok(())
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

impl AppConfig {
    fn parse_env_file(path: &Path) -> Result<Vec<(String, String)>> {
        let contents = fs::read_to_string(path)?;
        Ok(contents
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }
                let mut parts = line.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key.to_string(), value.trim().to_string())),
                    _ => None,
                }
            })
            .collect())
    }

    fn get_license_env_path() -> Result<PathBuf> {
        let exe_dir = std::env::current_exe()?
            .parent()
            .context("Failed to get exe directory")?
            .to_path_buf();
        let exe_env = exe_dir.join(".env.licenses");

        // Check exe directory first
        if exe_env.exists() {
            return Ok(exe_env);
        }

        // Fallback to workspace root for development
        if let Ok(current_dir) = std::env::current_dir() {
            let workspace_env = current_dir.join(".env.licenses");
            if workspace_env.exists() {
                return Ok(workspace_env);
            }
        }

        // Return exe path even if it doesn't exist (for better error message)
        Ok(exe_env)
    }

    pub fn load_keygen_config() -> Result<KeygenConfig> {
        let env_path = Self::get_license_env_path()?;
        if !env_path.exists() {
            anyhow::bail!(".env.licenses not found at {:?}", env_path);
        }
        let kv = Self::parse_env_file(&env_path)?;
        let mut lookup: HashMap<String, String> = kv.into_iter().collect();
        Ok(KeygenConfig {
            account_id: lookup
                .remove("KEYGEN_ACCOUNT_UID")
                .context("KEYGEN_ACCOUNT_UID missing in .env.licenses")?,
            product_id: lookup
                .remove("KEYGEN_PRODUCT_ID")
                .context("KEYGEN_PRODUCT_ID missing in .env.licenses")?,
            policy_trial: lookup
                .remove("KEYGEN_POLICY_TRIAL")
                .context("KEYGEN_POLICY_TRIAL missing in .env.licenses")?,
            policy_pro: lookup
                .remove("KEYGEN_POLICY_PRO")
                .context("KEYGEN_POLICY_PRO missing in .env.licenses")?,
            admin_token: lookup
                .remove("KEYGEN_ADMIN_TOKEN")
                .context("KEYGEN_ADMIN_TOKEN missing in .env.licenses")?,
            public_key_hex: lookup
                .remove("KEYGEN_PUBLIC_KEY")
                .context("KEYGEN_PUBLIC_KEY missing in .env.licenses")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeygenConfig {
    pub account_id: String,
    pub product_id: String,
    pub policy_trial: String,
    pub policy_pro: String,
    pub admin_token: String,
    pub public_key_hex: String,
}
