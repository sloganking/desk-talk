use anyhow::{bail, Context, Result};
use async_openai::Client;
use dotenvy::dotenv;
use enigo::{Enigo, KeyboardControllable}; // Keep KeyboardControllable for 0.1.3
use std::env;
use tempfile::tempdir;
mod transcribe;
use std::thread::{self, sleep};
use transcribe::trans;
mod record;
use async_std::future::timeout as async_std_timeout;
use clap::{Parser, Subcommand};
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use cpal::traits::{DeviceTrait, HostTrait};
use rdev::{listen, Event};
use record::rec;
use std::path::PathBuf;
use std::time::Duration;
mod easy_rdev_key;
use crate::easy_rdev_key::PTTKey;

#[cfg(windows)]
use clipboard_win::{formats, set_clipboard};

#[derive(Parser, Debug)]
#[command(version)]
struct Opt {
    #[arg(short, long, default_value_t = String::from("default"))]
    device: String,
    #[arg(short, long)]
    api_key: Option<String>,
    #[arg(short, long)]
    ptt_key: Option<PTTKey>,
    #[arg(short, long)]
    cap_first: bool,
    #[arg(short, long)]
    space: bool,
    #[arg(short, long)]
    type_chars: bool,
    #[arg(long)]
    audio: bool,
    #[arg(short, long, conflicts_with("ptt_key"))]
    special_ptt_key: Option<u32>,
    #[clap(subcommand)]
    pub subcommands: Option<SubCommands>,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    ShowKeyPresses,
    ListDevices,
}

fn capitalize_first_letter(s: &mut String) {
    if let Some(f) = s.chars().next() {
        if f.is_lowercase() {
            let first_char_len = f.len_utf8();
            let uppercase: String = f.to_uppercase().collect();
            s.replace_range(0..first_char_len, &uppercase);
        }
    }
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    let _ = dotenv();

    let mut text_clipboard: ClipboardContext = ClipboardProvider::new()
        .map_err(|e| anyhow::anyhow!("Clipboard provider init failed: {}", e))
        .context("Failed to initialize text clipboard provider")?;

    match opt.subcommands {
        Some(subcommand) => {
            match subcommand {
                SubCommands::ShowKeyPresses => {
                    println!("Press keys to see their codes. Press Ctrl+C to exit...");
                    fn show_keys_callback(event: Event) {
                        if let rdev::EventType::KeyPress(key) = event.event_type {
                            println!("Key pressed: {:?}", key);
                        }
                    }
                    listen(show_keys_callback)
                        .map_err(|e| anyhow::anyhow!("Key listener error: {:?}", e))?;
                }
                SubCommands::ListDevices => {
                    let host = cpal::default_host();
                    println!("Default output device:");
                    if let Some(device) = host.default_output_device() {
                        println!("- {}", device.name()?);
                    } else {
                        println!("- Not found.");
                    }
                    println!("\nAvailable input devices:");
                    let devices = host.input_devices()?;
                    for device in devices {
                        println!("- {}", device.name()?);
                    }
                }
            }
            Ok(())
        }
        None => {
            let ptt_key = match opt.ptt_key {
                Some(k) => k.into(),
                None => match opt.special_ptt_key {
                    Some(sk) => rdev::Key::Unknown(sk),
                    None => {
                        bail!("No push to talk key specified. Use --ptt-key or --special-ptt-key.");
                    }
                },
            };

            if let Some(api_key) = opt.api_key {
                env::set_var("OPENAI_API_KEY", api_key);
            }

            if env::var("OPENAI_API_KEY").is_err() {
                bail!("OPENAI_API_KEY not set. Pass via --api-key or environment variable.");
            }

            let (tx, rx): (flume::Sender<Event>, flume::Receiver<Event>) = flume::unbounded();

            let device_name = opt.device;
            let cap_first = opt.cap_first;
            let add_space = opt.space;
            let type_chars = opt.type_chars;
            let copy_audio = opt.audio;

            thread::spawn(move || {
                let mut recorder = rec::Recorder::new();
                let client = Client::new();
                let runtime =
                    tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                let mut enigo = Enigo::new(); // enigo 0.1.3
                let tmp_dir = tempdir().expect("Failed to create temp directory");
                let voice_tmp_path: PathBuf = tmp_dir.path().join("voice_tmp.wav");
                let mut recording_start = std::time::SystemTime::now();
                let mut key_pressed = false;

                println!("Listening for PTT key ({:?}).", ptt_key);

                for event in rx.iter() {
                    match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            if key == ptt_key && !key_pressed {
                                key_pressed = true;
                                println!("PTT pressed. Recording...");
                                recording_start = std::time::SystemTime::now();
                                if let Err(e) =
                                    recorder.start_recording(&voice_tmp_path, Some(&device_name))
                                {
                                    eprintln!("Error starting recording: {:?}", e);
                                    key_pressed = false;
                                }
                            }
                        }
                        rdev::EventType::KeyRelease(key) => {
                            if key == ptt_key && key_pressed {
                                key_pressed = false;
                                println!("PTT released. Processing...");

                                if let Err(e) = recorder.stop_recording() {
                                    eprintln!("Error stopping recording: {:?}", e);
                                    continue;
                                }

                                let elapsed = match recording_start.elapsed() {
                                    Ok(d) => d,
                                    Err(e) => {
                                        eprintln!("Error getting elapsed time: {}", e);
                                        continue;
                                    }
                                };

                                if elapsed.as_secs_f32() > 0.2 {
                                    println!(
                                        "Transcribing audio ({} seconds)...",
                                        elapsed.as_secs_f32()
                                    );

                                    let transcription_result = runtime.block_on(async_std_timeout(
                                        Duration::from_secs(20),
                                        trans::transcribe(&client, &voice_tmp_path),
                                    ));

                                    let mut transcription = match transcription_result {
                                        Ok(Ok(text)) => text,
                                        Ok(Err(e)) => {
                                            eprintln!("Transcription error: {:?}", e);
                                            continue;
                                        }
                                        Err(_) => {
                                            eprintln!("Transcription timed out.");
                                            continue;
                                        }
                                    };

                                    if cap_first {
                                        capitalize_first_letter(&mut transcription);
                                    }
                                    if add_space {
                                        if !transcription.is_empty()
                                            && !transcription.ends_with(' ')
                                        {
                                            transcription.push(' ');
                                        }
                                    }
                                    transcription =
                                        transcription.replace("...", "").trim().to_string();

                                    if transcription.is_empty() {
                                        println!("Transcription empty.");
                                        continue;
                                    }
                                    println!("Transcription: {}", transcription);

                                    // --- Audio Paste ---
                                    #[cfg(windows)]
                                    if copy_audio {
                                        println!("Copying audio file...");
                                        let clip_text_backup = text_clipboard.get_contents().ok();

                                        match (|| -> Result<()> {
                                            let abs_path = voice_tmp_path.canonicalize().context(
                                                "Failed to get absolute path for clipboard",
                                            )?;
                                            let path_str = abs_path.to_string_lossy().into_owned();

                                            // --- Create Vec and then Slice Reference ---
                                            let files_vec: Vec<String> = vec![path_str];
                                            let files_slice: &[String] = files_vec.as_slice(); // Explicitly type as &[String]

                                            // --- Call with slice reference as data ---
                                            set_clipboard(formats::FileList, files_slice).map_err(
                                                |e| {
                                                    anyhow::anyhow!(
                                                        "clipboard-win set_clipboard failed: {}",
                                                        e
                                                    )
                                                },
                                            )?;

                                            println!("File copied via clipboard-win. Pasting...");
                                            enigo.key_sequence_parse("{+CTRL}v{-CTRL}");
                                            sleep(Duration::from_millis(150));
                                            Ok(())
                                        })() {
                                            Ok(_) => {}
                                            Err(e) => {
                                                eprintln!("Audio copy/paste failed: {:?}", e);
                                            }
                                        }

                                        if let Some(backup) = clip_text_backup {
                                            let _ = text_clipboard.set_contents(backup);
                                        }
                                    }
                                    #[cfg(not(windows))]
                                    if copy_audio {
                                        eprintln!("Warning: --audio only supported on Windows.");
                                    }
                                    // --- End Audio Paste ---

                                    // --- Text Output ---
                                    if type_chars {
                                        println!("Typing text...");
                                        enigo.key_sequence(&transcription);
                                    } else {
                                        println!("Pasting text...");
                                        let clip_tmp = text_clipboard.get_contents().ok();
                                        if let Err(e) =
                                            text_clipboard.set_contents(transcription.clone())
                                        {
                                            eprintln!("Failed to set text clipboard: {:?}", e);
                                        } else {
                                            enigo.key_sequence_parse("{+CTRL}v{-CTRL}");
                                            sleep(Duration::from_millis(100));
                                            if let Some(backup) = clip_tmp {
                                                let _ = text_clipboard.set_contents(backup);
                                            }
                                        }
                                    } // --- End Text Output ---
                                } else {
                                    println!("Recording too short.");
                                }
                            }
                        }
                        _ => (),
                    }
                } // End for event loop
                println!("Key handler thread finished.");
            }); // End thread::spawn

            println!("Main thread listening...");
            let callback = move |event: Event| {
                if tx.send(event).is_err() {
                    eprintln!("Key handler thread receiver dropped. Stopping listener might require Ctrl+C.");
                }
            };
            listen(callback).map_err(|e| anyhow::anyhow!("Global key listener error: {:?}", e))?;
            println!("Main thread finished listening.");
            Ok(())
        }
    }
}
