// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod config;
mod easy_rdev_key;
mod license;
mod record;
mod tauri_commands;
mod transcribe;
mod transcription_engine;

use app_state::AppState;
use config::AppConfig;
use default_device_sink::DefaultDeviceSink;
use parking_lot::Mutex;
use rdev::{listen, Event, EventType};
use rodio::{source::SineWave, Decoder, Source};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, State,
};
use transcription_engine::TranscriptionEngine;

static FAILED_BYTES: &[u8] = include_bytes!("../assets/failed.mp3");

struct AppEngine {
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
}

pub fn play_error_sound() {
    let sink = DefaultDeviceSink::new();
    if let Ok(decoder) = Decoder::new(BufReader::new(Cursor::new(FAILED_BYTES))) {
        sink.append(decoder);
    } else {
        sink.append(
            SineWave::new(440.0)
                .take_duration(Duration::from_millis(150))
                .amplify(0.20),
        );
    }
    sink.sleep_until_end();
}

fn start_global_event_listener(app_state: AppState) {
    thread::spawn(move || {
        println!("Global event listener started");

        // Channel for error sound playback (avoid blocking rdev callback)
        let (error_tx, error_rx): (flume::Sender<()>, flume::Receiver<()>) = flume::unbounded();

        // Thread to play error sounds (so rdev callback isn't blocked)
        thread::spawn(move || {
            for _ in error_rx.iter() {
                play_error_sound();
            }
        });

        let mut ptt_key_pressed = false;

        let callback = move |event: Event| {
            // Check if engine has an active event sender (fast, cached check)
            let sender_opt = app_state.event_sender();
            let engine_ready = sender_opt.is_some();

            // Forward events to engine if it's ready
            if engine_ready {
                if let Some(sender) = sender_opt {
                    let _ = sender.send(event.clone());
                }
            }

            // Track PTT key state and play error sound if engine is NOT ready
            if let Some(ptt_key) = app_state.config.read().get_ptt_key() {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        if key == ptt_key && !ptt_key_pressed {
                            ptt_key_pressed = true;
                            // If engine doesn't have a sender, it's not ready - play error immediately
                            if !engine_ready {
                                println!(
                                    "PTT pressed but engine is NOT ready - playing error sound"
                                );
                                let _ = error_tx.send(()); // Non-blocking send to error sound thread
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if key == ptt_key {
                            ptt_key_pressed = false;
                        }
                    }
                    _ => {}
                }
            }
        };

        if let Err(error) = listen(callback) {
            eprintln!("Error in global event listener: {:?}", error);
        }
    });
}

fn auto_start_if_possible<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let config = state.config.read().clone();

    // Check for license first
    if config.license_key.is_none() {
        println!("Auto-start skipped: no active license");
        return;
    }

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
async fn start_engine<R: Runtime>(
    app_handle: AppHandle<R>,
    state: State<'_, AppState>,
    engine_state: State<'_, AppEngine>,
) -> Result<(), String> {
    if engine_state.engine.lock().is_some() {
        println!("Engine already running");
        return Ok(());
    }

    // Check for valid license before starting
    let (has_license, license_key, fingerprint) = {
        let config = state.config.read();
        (
            config.license_key.is_some(),
            config.license_key.clone(),
            config.machine_id.clone(),
        )
    };

    if !has_license {
        return Err("No active license. Please activate a license to use DeskTalk.".to_string());
    }

    // Validate the license is still active
    if let Some(client) = state.keygen_client() {
        if let Some(key) = license_key {
            match client.validate_license(&key, &fingerprint).await {
                Ok(validation) => {
                    let status = validation
                        .license
                        .status
                        .as_deref()
                        .unwrap_or("UNKNOWN")
                        .to_uppercase();
                    if status != "ACTIVE" {
                        return Err(format!(
                            "License is {}. Please contact support.",
                            status.to_lowercase()
                        ));
                    }
                    println!("License validated: {}", status);
                }
                Err(e) => {
                    return Err(format!(
                        "License validation failed: {}. Please reactivate your license.",
                        e
                    ));
                }
            }
        }
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
    let restart_item = MenuItem::with_id(app, "restart", "Restart Engine", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop", "Stop Engine", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[&open_settings, &restart_item, &stop_item, &quit_item],
    )
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
        "restart" => {
            let state = app.state::<AppState>();
            let engine_state = app.state::<AppEngine>();
            // Stop then start
            let _ = stop_engine(app.clone(), engine_state.clone());
            // Need to spawn async task for start_engine
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_clone.state::<AppState>();
                let engine_state = app_clone.state::<AppEngine>();
                let _ = start_engine(app_clone.clone(), state, engine_state).await;
            });
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
    println!(
        "Main: Initial config has API key: {}",
        config.api_key.is_some()
    );
    let keygen_config = match AppConfig::load_keygen_config() {
        Ok(cfg) => {
            println!("✓ Keygen config loaded successfully");
            Some(cfg)
        }
        Err(e) => {
            println!("✗ Failed to load Keygen config: {}", e);
            None
        }
    };
    let app_state = AppState::new(config, keygen_config);

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
            tauri_commands::test_openai_key,
            tauri_commands::detect_key_press,
            tauri_commands::fetch_license_status,
            tauri_commands::activate_license,
            tauri_commands::deactivate_license,
            tauri_commands::check_license_periodically,
            tauri_commands::open_url,
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

            // Check if we should start minimized BEFORE creating anything
            let state = app.state::<AppState>();
            let should_minimize = state.config.read().start_minimized;

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

            if let Some(window) = app.get_webview_window("settings") {
                let window_handle = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_handle.hide();
                    }
                });

                // Show window if NOT configured to start minimized
                if !should_minimize {
                    let _ = window.show();
                    println!("Starting with window visible");
                } else {
                    println!("Starting minimized to tray");
                }
            }

            // Start global event listener for PTT handling
            let state_for_listener = app.state::<AppState>();
            start_global_event_listener(state_for_listener.inner().clone());

            // Attempt to auto-start transcription if configuration is ready
            auto_start_if_possible(&handle_for_auto_start);

            Ok(())
        })
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
