// Prevents additional console window on Windows in release mode
// TEMPORARILY DISABLED FOR DEBUGGING
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod config;
mod easy_rdev_key;
mod record;
mod tauri_commands;
mod transcribe;
mod transcription_engine;

use app_state::AppState;
use config::AppConfig;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, State,
};
use transcription_engine::TranscriptionEngine;

struct AppEngine {
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
}

fn auto_start_if_possible<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let config = state.config.read().clone();

    if config.get_ptt_key().is_none() {
        println!("Auto-start skipped: no PTT key configured");
        return;
    }

    if !config.use_local && config.api_key.is_none() {
        println!("Auto-start skipped: no OpenAI API key configured");
        return;
    }

    if config.use_local && config.local_model.is_none() {
        println!("Auto-start skipped: no local model selected");
        return;
    }

    let engine_state = app.state::<AppEngine>();
    if engine_state.engine.lock().is_some() {
        println!("Auto-start skipped: engine already running");
        return;
    }

    println!("Auto-starting transcription engine...");
    let engine = TranscriptionEngine::new(state.inner().clone());
    match engine.start() {
        Ok(_) => {
            println!("Auto-start successful");
            *engine_state.engine.lock() = Some(engine);
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_tooltip(Some("DeskTalk - Running"));
            }
        }
        Err(err) => {
            println!("Auto-start failed: {}", err);
        }
    }
}

#[tauri::command]
fn start_engine<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<AppState>,
    engine_state: State<AppEngine>,
) -> Result<(), String> {
    if engine_state.engine.lock().is_some() {
        println!("Engine already running");
        return Ok(());
    }

    println!("Starting transcription engine...");

    // Debug: print config
    let config = state.config.read();
    println!("Config - PTT Key: {:?}", config.ptt_key);
    println!("Config - Device: {}", config.device);
    println!("Config - Use Local: {}", config.use_local);
    println!("Config - Has API Key: {}", config.api_key.is_some());
    drop(config);

    let engine = TranscriptionEngine::new(state.inner().clone());

    match engine.start() {
        Ok(_) => {
            println!("Transcription engine started successfully!");
            *engine_state.engine.lock() = Some(engine);

            // Update tray icon to show running state
            if let Some(tray) = app_handle.tray_by_id("main") {
                let _ = tray.set_tooltip(Some("DeskTalk - Running"));
            }

            Ok(())
        }
        Err(e) => {
            println!("Failed to start engine: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn stop_engine<R: Runtime>(
    app_handle: AppHandle<R>,
    engine_state: State<AppEngine>,
) -> Result<(), String> {
    if let Some(engine) = engine_state.engine.lock().take() {
        engine.stop();
    }

    // Update tray icon to show stopped state
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_tooltip(Some("DeskTalk - Stopped"));
    }

    Ok(())
}

fn create_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let open_settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let start_item = MenuItem::with_id(app, "start", "Start Transcription", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop", "Stop Transcription", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(app, &[&open_settings, &start_item, &stop_item, &quit_item])
}

fn handle_tray_event<R: Runtime>(app: &AppHandle<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        if let Some(window) = app.get_webview_window("settings") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "settings" => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "start" => {
            let state = app.state::<AppState>();
            let engine_state = app.state::<AppEngine>();
            let _ = start_engine(app.clone(), state, engine_state);
        }
        "stop" => {
            let engine_state = app.state::<AppEngine>();
            let _ = stop_engine(app.clone(), engine_state);
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

fn main() {
    // Load configuration
    let config = AppConfig::load().unwrap_or_default();
    let app_state = AppState::new(config);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .manage(AppEngine {
            engine: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            tauri_commands::get_config,
            tauri_commands::save_config,
            tauri_commands::get_statistics,
            tauri_commands::get_audio_devices,
            tauri_commands::get_available_ptt_keys,
            tauri_commands::start_transcription,
            tauri_commands::stop_transcription,
            tauri_commands::is_running,
            tauri_commands::validate_api_key,
            tauri_commands::detect_key_press,
            start_engine,
            stop_engine,
        ])
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let handle_for_tray = app.handle().clone();
            let handle_for_auto_start = app.handle().clone();

            // Create tray menu
            let menu = create_tray_menu(&handle)?;
            let icon = app.default_window_icon().cloned();

            let mut builder = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("DeskTalk");
            if let Some(icon) = icon {
                builder = builder.icon(icon);
            }

            let _tray = builder
                .on_tray_icon_event(move |_tray, event| {
                    handle_tray_event(&handle_for_tray, event);
                })
                .build(app)?;

            // Attempt to auto-start transcription if configuration is ready
            auto_start_if_possible(&handle_for_auto_start);

            Ok(())
        })
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
