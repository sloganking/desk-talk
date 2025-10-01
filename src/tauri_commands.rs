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
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn save_config(state: tauri::State<AppState>, mut incoming: AppConfig) -> Result<(), String> {
    println!("=== SAVE CONFIG CALLED ===");
    println!("PTT Key: {:?}", incoming.ptt_key);
    println!("Device: {}", incoming.device);
    println!("API Key present: {}", incoming.api_key.is_some());

    {
        let mut current = state.config.write();
        // Preserve licensing-related fields and machine fingerprint
        let license_key = current.license_key.clone();
        let license_plan = current.license_plan.clone();
        let license_id = current.license_id.clone();
        let trial_expiration = current.trial_expiration.clone();
        let trial_started = current.trial_started;
        let machine_id = current.machine_id.clone();

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
    // Simple validation - check if it starts with sk-
    Ok(api_key.starts_with("sk-") && api_key.len() > 20)
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
    let client = state
        .keygen_client()
        .ok_or_else(|| "Licensing not configured".to_string())?;

    let license_key = state
        .config
        .read()
        .license_key
        .clone()
        .ok_or_else(|| "No license key saved".to_string())?;

    let fingerprint = state.config.read().machine_id.clone();

    let result = client
        .validate_license(&license_key, &fingerprint)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LicenseStatus {
        status: result.license.status,
        plan: result.license.plan,
        key: Some(license_key),
        expires_at: result.license.expires_at,
        max_machines: result.license.max_machines,
        machines_used: result.license.machines_used,
    })
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

    if validation.machine.is_none() {
        let _machine = client
            .activate_machine(&license_key, &license_id, &fingerprint, &host_name)
            .await
            .map_err(|e| {
                // Check if the error is about already being activated
                let err_str = e.to_string();
                if err_str.contains("FINGERPRINT_TAKEN")
                    || err_str.contains("has already been taken")
                {
                    "This device has already been activated with this license.".to_string()
                } else {
                    err_str
                }
            })?;
    }

    {
        let mut config = state.config.write();
        config.license_key = Some(license_key.clone());
        config.license_plan = validation.license.plan.clone();
        config.license_id = Some(license_id.clone());
        config.trial_started = false;
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
