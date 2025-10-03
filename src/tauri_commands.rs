use crate::app_state::{AppState, Statistics};
use crate::config::AppConfig;
use crate::easy_rdev_key::PTTKey;
use cpal::traits::{DeviceTrait, HostTrait};
use hostname;
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct LicenseStatus {
    pub status: Option<String>,
    pub plan: Option<String>,
    pub key: Option<String>,
    pub expires_at: Option<String>,
    pub max_machines: Option<u32>,
    pub machines_used: Option<u32>,
}

#[tauri::command]
pub fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().clone();
    println!("get_config: api_key present: {}", config.api_key.is_some());
    if let Some(ref key) = config.api_key {
        println!("get_config: api_key length: {}", key.len());
    }
    Ok(config)
}

#[tauri::command]
pub fn save_config(state: tauri::State<AppState>, mut incoming: AppConfig) -> Result<(), String> {
    println!("=== SAVE CONFIG CALLED ===");
    println!("PTT Key: {:?}", incoming.ptt_key);
    println!("Device: {}", incoming.device);
    println!("API Key present: {}", incoming.api_key.is_some());

    {
        let mut current = state.config.write();
        // Preserve licensing-related fields, machine fingerprint, AND api_key
        let license_key = current.license_key.clone();
        let license_plan = current.license_plan.clone();
        let license_id = current.license_id.clone();
        let trial_expiration = current.trial_expiration.clone();
        let trial_started = current.trial_started;
        let machine_id = current.machine_id.clone();

        // If incoming has an API key, save it to keyring/env
        if let Some(ref api_key) = incoming.api_key {
            if !api_key.is_empty() {
                println!("Saving new API key to keyring/env...");
                AppConfig::save_api_key(api_key).map_err(|e| {
                    println!("ERROR saving API key: {}", e);
                    e.to_string()
                })?;
                println!("API key saved successfully");
            }
        }

        incoming.license_key = license_key;
        incoming.license_plan = license_plan;
        incoming.license_id = license_id;
        incoming.trial_expiration = trial_expiration;
        incoming.trial_started = trial_started;
        incoming.machine_id = machine_id;

        *current = incoming;
        current.save().map_err(|e| {
            println!("ERROR saving config: {}", e);
            e.to_string()
        })?;

        // Reload API key from keyring/env back into memory
        current.api_key = AppConfig::load_api_key().ok();
        println!(
            "API key reloaded from keyring/env: {}",
            current.api_key.is_some()
        );
    }

    // reload keygen client in state after saving (e.g. if .env.licenses added later)
    if let Ok(cfg) = AppConfig::load_keygen_config() {
        if let Ok(client) = crate::license::KeygenClient::new(cfg) {
            *state.keygen.write() = Some(Arc::new(client));
        }
    }

    println!("Config saved to disk (without API key)");
    println!("=== CONFIG SAVE COMPLETE ===");

    Ok(())
}

#[tauri::command]
pub fn get_statistics(state: tauri::State<AppState>) -> Result<Statistics, String> {
    Ok(state.get_statistics())
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| format!("Failed to get input devices: {}", e))?;

    let device_names: Vec<String> = devices.filter_map(|device| device.name().ok()).collect();

    Ok(device_names)
}

#[tauri::command]
pub fn get_available_ptt_keys() -> Result<Vec<String>, String> {
    use clap::ValueEnum;
    Ok(PTTKey::value_variants()
        .iter()
        .map(|k| format!("{:?}", k))
        .collect())
}

#[tauri::command]
pub fn start_transcription(state: tauri::State<AppState>) -> Result<(), String> {
    state.start_transcription();
    Ok(())
}

#[tauri::command]
pub fn stop_transcription(state: tauri::State<AppState>) -> Result<(), String> {
    state.stop_transcription();
    Ok(())
}

#[tauri::command]
pub fn is_running(state: tauri::State<AppState>) -> Result<bool, String> {
    Ok(state.is_running())
}

#[tauri::command]
pub fn validate_api_key(api_key: String) -> Result<bool, String> {
    // Validate OpenAI API key format
    // Standard format: sk-[48 alphanumeric characters] or sk-proj-[64 characters]
    if !api_key.starts_with("sk-") {
        return Ok(false);
    }

    // Check minimum length (sk- + at least 20 chars)
    if api_key.len() < 23 {
        return Ok(false);
    }

    // OpenAI keys are typically 48-64 characters after 'sk-' or 'sk-proj-'
    let key_part = if api_key.starts_with("sk-proj-") {
        &api_key[8..]
    } else {
        &api_key[3..]
    };

    // Check it contains only valid characters (alphanumeric and possibly some symbols)
    let is_valid_format = key_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_');

    Ok(is_valid_format && key_part.len() >= 20)
}

#[tauri::command]
pub async fn test_openai_key(api_key: String) -> Result<bool, String> {
    // Actually test the key with OpenAI API
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(true)
            } else if resp.status() == 401 {
                Err("Invalid API key - authentication failed".to_string())
            } else {
                Err(format!("API key test failed: HTTP {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Network error testing API key: {}", e)),
    }
}

#[tauri::command]
pub async fn detect_key_press() -> Result<String, String> {
    // Disabled to prevent crashes - users can select from dropdown
    Err("Please select a key from the dropdown menu. Auto-detection is disabled to prevent crashes.".to_string())
}

#[tauri::command]
pub async fn fetch_license_status(
    state: tauri::State<'_, AppState>,
) -> Result<LicenseStatus, String> {
    // Read config and extract what we need without holding the lock
    let (license_key, license_plan, fingerprint) = {
        let config = state.config.read();
        let key = config
            .license_key
            .clone()
            .ok_or_else(|| "No license key saved".to_string())?;
        let plan = config
            .license_plan
            .clone()
            .unwrap_or_else(|| "Pro".to_string());
        let fp = config.machine_id.clone();
        (key, plan, fp)
    }; // Lock is dropped here

    // Return locally stored license info (don't fail if online validation fails)
    // This ensures the UI shows the license even offline or if Keygen is unreachable
    let local_status = LicenseStatus {
        status: Some("Active".to_string()), // Assume active if we have a key
        plan: Some(license_plan),
        key: Some(license_key.clone()),
        expires_at: None,      // Not stored locally
        max_machines: Some(3), // Default from Keygen policy
        machines_used: None,   // Not stored locally
    };

    // Try to validate online for fresh data, but don't fail if it doesn't work
    let client = match state.keygen_client() {
        Some(c) => c,
        None => return Ok(local_status), // No client configured, return local info
    };

    match client.validate_license(&license_key, &fingerprint).await {
        Ok(result) => Ok(LicenseStatus {
            status: result.license.status,
            plan: result.license.plan,
            key: Some(license_key),
            expires_at: result.license.expires_at,
            max_machines: result.license.max_machines,
            machines_used: result.license.machines_used,
        }),
        Err(_) => Ok(local_status), // Validation failed, return local info
    }
}

#[tauri::command]
pub async fn activate_license(
    state: tauri::State<'_, AppState>,
    license_key: String,
) -> Result<LicenseStatus, String> {
    let client = state
        .keygen_client()
        .ok_or_else(|| "Licensing not configured".to_string())?;

    let fingerprint = state.config.read().machine_id.clone();
    let host_name = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let validation = client
        .validate_license(&license_key, &fingerprint)
        .await
        .map_err(|e| e.to_string())?;

    let license_id = validation.license.id.clone();

    // Try to activate machine if not already activated
    if validation.machine.is_none() {
        match client
            .activate_machine(&license_key, &license_id, &fingerprint, &host_name)
            .await
        {
            Ok(_) => {
                // Successfully activated
            }
            Err(e) => {
                // Check if the error is about already being activated
                let err_str = e.to_string();
                if err_str.contains("FINGERPRINT_TAKEN")
                    || err_str.contains("has already been taken")
                {
                    // This is fine - device was already activated, treat as success
                    println!("Device already activated, continuing...");
                } else {
                    // Other errors are actual failures
                    return Err(err_str);
                }
            }
        }
    }

    {
        let mut config = state.config.write();
        config.license_key = Some(license_key.clone());
        config.license_plan = validation.license.plan.clone();
        config.license_id = Some(license_id.clone());
        // Keep trial_started as-is (don't reset it)
        // If user had a trial before, that info should persist
        config
            .save()
            .map_err(|e| format!("Failed to persist license: {}", e))?;
    }

    Ok(LicenseStatus {
        status: validation.license.status,
        plan: validation.license.plan,
        key: Some(license_key),
        expires_at: validation.license.expires_at,
        max_machines: validation.license.max_machines,
        machines_used: validation.license.machines_used,
    })
}

#[tauri::command]
pub async fn check_license_periodically(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if state.keygen_client().is_none() {
        return Ok(());
    }
    Ok(())
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
pub async fn deactivate_license(state: tauri::State<'_, AppState>) -> Result<(), String> {
    {
        let mut config = state.config.write();
        config.license_key = None;
        config.license_plan = None;
        config.license_id = None;
        // DON'T reset trial_started or trial_expiration
        // Trial can only be used once per machine, even after license deactivation
        // This prevents users from getting free trials repeatedly
        config
            .save()
            .map_err(|e| format!("Failed to save config: {}", e))?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub struct TrialStatus {
    pub is_trial: bool,
    pub days_remaining: i64,
    pub expired: bool,
    pub expiration_date: Option<String>,
    pub human_remaining: Option<String>,
}

#[tauri::command]
pub fn start_trial(state: tauri::State<'_, AppState>) -> Result<TrialStatus, String> {
    use chrono::{Duration, Utc};

    let mut config = state.config.write();

    // Don't start trial if already has license or trial already started
    if config.license_key.is_some() {
        return Err("Already have an active license".to_string());
    }

    // IMPORTANT: Trial can only be started ONCE per machine, ever
    // This prevents users from abusing the system by:
    // 1. Starting trial
    // 2. Buying license
    // 3. Deactivating license
    // 4. Starting trial again (BLOCKED HERE)
    if config.trial_started {
        // Trial already used (even if expired)
        return Err(
            "Trial has already been used on this device. Please purchase a license to continue."
                .to_string(),
        );
    }

    // Start new 7-day trial (first and only time)
    let expiration = Utc::now() + Duration::days(7);
    config.trial_started = true;
    config.trial_expiration = Some(expiration.to_rfc3339());
    config.license_plan = Some("Trial".to_string());

    config
        .save()
        .map_err(|e| format!("Failed to save trial config: {}", e))?;

    println!("Started 7-day trial (first time), expires: {}", expiration);

    Ok(TrialStatus {
        is_trial: true,
        days_remaining: 7,
        expired: false,
        expiration_date: Some(expiration.to_rfc3339()),
        human_remaining: Some(humanize_duration(Duration::days(7))),
    })
}

#[tauri::command]
pub fn get_trial_status(state: tauri::State<'_, AppState>) -> Result<TrialStatus, String> {
    let config = state.config.read();
    get_trial_status_internal(&config)
}

pub fn get_trial_status_internal(config: &crate::config::AppConfig) -> Result<TrialStatus, String> {
    use chrono::{DateTime, Utc};

    // If has license key, not in trial
    if config.license_key.is_some() {
        return Ok(TrialStatus {
            is_trial: false,
            days_remaining: 0,
            expired: false,
            expiration_date: None,
            human_remaining: None,
        });
    }

    // If trial never started, return not-in-trial status
    if !config.trial_started || config.trial_expiration.is_none() {
        return Ok(TrialStatus {
            is_trial: false,
            days_remaining: 0,
            expired: false,
            expiration_date: None,
            human_remaining: None,
        });
    }

    // Parse expiration date
    let expiration_str = config.trial_expiration.as_ref().unwrap();
    let expiration = DateTime::parse_from_rfc3339(expiration_str)
        .map_err(|e| format!("Invalid expiration date: {}", e))?
        .with_timezone(&Utc);

    let now = Utc::now();
    let remaining_duration = expiration - now;
    let days_remaining = remaining_duration.num_days();
    let expired = now >= expiration;

    Ok(TrialStatus {
        is_trial: true,
        days_remaining: days_remaining.max(0),
        expired,
        expiration_date: Some(expiration_str.clone()),
        human_remaining: if expired {
            None
        } else {
            Some(humanize_duration(remaining_duration))
        },
    })
}

fn humanize_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds();
    let secs = if secs <= 0 { 0 } else { secs as u64 };
    humantime::format_duration(std::time::Duration::from_secs(secs)).to_string()
}

#[tauri::command]
pub fn format_trial_remaining(expiration: String) -> Result<String, String> {
    use chrono::{DateTime, Utc};

    let expiration = DateTime::parse_from_rfc3339(&expiration)
        .map_err(|e| format!("Invalid expiration date: {}", e))?
        .with_timezone(&Utc);
    let now = Utc::now();

    if now >= expiration {
        return Ok("Expired".to_string());
    }

    let remaining = expiration - now;
    Ok(humanize_duration(remaining))
}
