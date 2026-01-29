use crate::app_state::{AppState, Statistics};
use crate::config::AppConfig;
use crate::easy_rdev_key::PTTKey;
use cpal::traits::{DeviceTrait, HostTrait};

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
pub fn save_config(state: tauri::State<AppState>, incoming: AppConfig) -> Result<(), String> {
    println!("=== SAVE CONFIG CALLED ===");
    println!("PTT Key: {:?}", incoming.ptt_key);
    println!("Device: {}", incoming.device);
    println!("API Key present: {}", incoming.api_key.is_some());

    {
        let mut current = state.config.write();

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
pub async fn open_url(url: String) -> Result<(), String> {
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}
