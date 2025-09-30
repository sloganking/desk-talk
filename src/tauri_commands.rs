use crate::app_state::{AppState, Statistics};
use crate::config::AppConfig;
use crate::easy_rdev_key::PTTKey;
use cpal::traits::{DeviceTrait, HostTrait};
use tauri::State;

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    println!("=== SAVE CONFIG CALLED ===");
    println!("PTT Key: {:?}", config.ptt_key);
    println!("Device: {}", config.device);
    println!("API Key present: {}", config.api_key.is_some());

    // Save to disk and keyring
    config.save().map_err(|e| {
        println!("ERROR saving config: {}", e);
        e.to_string()
    })?;

    println!("Config saved to disk (without API key)");

    // Reload the config to get the API key from keyring/env
    let reloaded_config = AppConfig::load().map_err(|e| {
        println!("ERROR loading config: {}", e);
        e.to_string()
    })?;

    println!(
        "Config reloaded - API key present: {}",
        reloaded_config.api_key.is_some()
    );

    // Update app state with reloaded config (which includes API key from keyring)
    *state.config.write() = reloaded_config;

    println!("=== CONFIG SAVE COMPLETE ===");

    Ok(())
}

#[tauri::command]
pub fn get_statistics(state: State<AppState>) -> Result<Statistics, String> {
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
pub fn start_transcription(state: State<AppState>) -> Result<(), String> {
    state.start_transcription();
    Ok(())
}

#[tauri::command]
pub fn stop_transcription(state: State<AppState>) -> Result<(), String> {
    state.stop_transcription();
    Ok(())
}

#[tauri::command]
pub fn is_running(state: State<AppState>) -> Result<bool, String> {
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
