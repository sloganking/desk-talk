// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod config;
mod easy_rdev_key;
mod record;
mod transcribe;
mod transcription_engine;
mod tauri_commands;

use app_state::AppState;
use config::AppConfig;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{
    image::Image, menu::{Menu, MenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, AppHandle, Manager, Runtime, State
};
use transcription_engine::TranscriptionEngine;

struct AppEngine {
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
}

#[tauri::command]
fn start_engine(
    app_handle: AppHandle,
    state: State<AppState>,
    engine_state: State<AppEngine>,
) -> Result<(), String> {
    let engine = TranscriptionEngine::new(state.inner().clone());
    
    engine.start().map_err(|e| e.to_string())?;
    
    *engine_state.engine.lock() = Some(engine);
    
    // Update tray icon to show running state
    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_tooltip(Some("DeskTalk - Running"));
    }
    
    Ok(())
}

#[tauri::command]
fn stop_engine(
    app_handle: AppHandle,
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
    
    Menu::with_items(
        app,
        &[
            &open_settings,
            &start_item,
            &stop_item,
            &quit_item,
        ],
    )
}

fn handle_tray_event<R: Runtime>(app: &AppHandle<R>, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        TrayIconEvent::MenuItemClick { id, .. } => {
            match id.as_ref() {
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
            start_engine,
            stop_engine,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Create tray menu
            let menu = create_tray_menu(&handle)?;
            
            // Build tray icon
            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("DeskTalk")
                .icon(app.default_window_icon().unwrap().clone())
                .on_tray_icon_event(move |tray, event| {
                    handle_tray_event(&handle, event);
                })
                .build(app)?;
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
