use anyhow::Context;
use async_openai::Client;
use dotenvy::dotenv;
use enigo::{Enigo, EnigoSettings, KeyboardControllable};
use std::env;
use tempfile::tempdir;
mod transcribe;
use transcribe::trans;
mod record;
use clap::{Parser, Subcommand};
use rdev::{listen, Event};
use record::rec;
use std::error::Error;

use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Parser, Debug)]
#[command(version)]
struct Opt {
    /// The audio device to use for recording. Leaving this blank will use the default device.
    #[arg(short, long, default_value_t = String::from("default"))]
    device: String,

    /// Your OpenAI API key
    #[arg(short, long)]
    api_key: Option<String>,

    /// The push to talk key
    #[arg(short, long)]
    ptt_key: Option<PTTKey>,

    /// The push to talk key.
    /// Use this if you want to use a key that is not supported by the PTTKey enum.
    #[arg(short, long, conflicts_with("ptt_key"))]
    special_ptt_key: Option<u32>,

    #[clap(subcommand)]
    pub subcommands: Option<SubCommands>,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    /// Displays keys as you press them so you can figure out what key to use for push to talk.
    ShowKeyPresses,
    /// Lists the audio input devices on your system.
    ListDevices,
}

/// This is just a straight copy of rdev::Key, so that #[derive(clap::ValueEnum)] works.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum PTTKey {
    /// Alt key on Linux and Windows (option key on macOS)
    Alt,
    AltGr,
    Backspace,
    CapsLock,
    ControlLeft,
    ControlRight,
    Delete,
    DownArrow,
    End,
    Escape,
    F1,
    F10,
    F11,
    F12,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    Home,
    LeftArrow,
    /// also known as "windows", "super", and "command"
    MetaLeft,
    /// also known as "windows", "super", and "command"
    MetaRight,
    PageDown,
    PageUp,
    Return,
    RightArrow,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    UpArrow,
    PrintScreen,
    ScrollLock,
    Pause,
    NumLock,
    BackQuote,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    Minus,
    Equal,
    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    LeftBracket,
    RightBracket,
    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    SemiColon,
    Quote,
    BackSlash,
    IntlBackslash,
    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    Comma,
    Dot,
    Slash,
    Insert,
    KpReturn,
    KpMinus,
    KpPlus,
    KpMultiply,
    KpDivide,
    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpDelete,
    Function,
    #[clap(skip)]
    Unknown(u32),
}

impl From<PTTKey> for rdev::Key {
    fn from(item: PTTKey) -> Self {
        match item {
            PTTKey::Alt => rdev::Key::Alt,
            PTTKey::AltGr => rdev::Key::AltGr,
            PTTKey::Backspace => rdev::Key::Backspace,
            PTTKey::CapsLock => rdev::Key::CapsLock,
            PTTKey::ControlLeft => rdev::Key::ControlLeft,
            PTTKey::ControlRight => rdev::Key::ControlRight,
            PTTKey::Delete => rdev::Key::Delete,
            PTTKey::DownArrow => rdev::Key::DownArrow,
            PTTKey::End => rdev::Key::End,
            PTTKey::Escape => rdev::Key::Escape,
            PTTKey::F1 => rdev::Key::F1,
            PTTKey::F10 => rdev::Key::F10,
            PTTKey::F11 => rdev::Key::F11,
            PTTKey::F12 => rdev::Key::F12,
            PTTKey::F2 => rdev::Key::F2,
            PTTKey::F3 => rdev::Key::F3,
            PTTKey::F4 => rdev::Key::F4,
            PTTKey::F5 => rdev::Key::F5,
            PTTKey::F6 => rdev::Key::F6,
            PTTKey::F7 => rdev::Key::F7,
            PTTKey::F8 => rdev::Key::F8,
            PTTKey::F9 => rdev::Key::F9,
            PTTKey::Home => rdev::Key::Home,
            PTTKey::LeftArrow => rdev::Key::LeftArrow,
            PTTKey::MetaLeft => rdev::Key::MetaLeft,
            PTTKey::MetaRight => rdev::Key::MetaRight,
            PTTKey::PageDown => rdev::Key::PageDown,
            PTTKey::PageUp => rdev::Key::PageUp,
            PTTKey::Return => rdev::Key::Return,
            PTTKey::RightArrow => rdev::Key::RightArrow,
            PTTKey::ShiftLeft => rdev::Key::ShiftLeft,
            PTTKey::ShiftRight => rdev::Key::ShiftRight,
            PTTKey::Space => rdev::Key::Space,
            PTTKey::Tab => rdev::Key::Tab,
            PTTKey::UpArrow => rdev::Key::UpArrow,
            PTTKey::PrintScreen => rdev::Key::PrintScreen,
            PTTKey::ScrollLock => rdev::Key::ScrollLock,
            PTTKey::Pause => rdev::Key::Pause,
            PTTKey::NumLock => rdev::Key::NumLock,
            PTTKey::BackQuote => rdev::Key::BackQuote,
            PTTKey::Num1 => rdev::Key::Num1,
            PTTKey::Num2 => rdev::Key::Num2,
            PTTKey::Num3 => rdev::Key::Num3,
            PTTKey::Num4 => rdev::Key::Num4,
            PTTKey::Num5 => rdev::Key::Num5,
            PTTKey::Num6 => rdev::Key::Num6,
            PTTKey::Num7 => rdev::Key::Num7,
            PTTKey::Num8 => rdev::Key::Num8,
            PTTKey::Num9 => rdev::Key::Num9,
            PTTKey::Num0 => rdev::Key::Num0,
            PTTKey::Minus => rdev::Key::Minus,
            PTTKey::Equal => rdev::Key::Equal,
            PTTKey::KeyQ => rdev::Key::KeyQ,
            PTTKey::KeyW => rdev::Key::KeyW,
            PTTKey::KeyE => rdev::Key::KeyE,
            PTTKey::KeyR => rdev::Key::KeyR,
            PTTKey::KeyT => rdev::Key::KeyT,
            PTTKey::KeyY => rdev::Key::KeyY,
            PTTKey::KeyU => rdev::Key::KeyU,
            PTTKey::KeyI => rdev::Key::KeyI,
            PTTKey::KeyO => rdev::Key::KeyO,
            PTTKey::KeyP => rdev::Key::KeyP,
            PTTKey::LeftBracket => rdev::Key::LeftBracket,
            PTTKey::RightBracket => rdev::Key::RightBracket,
            PTTKey::KeyA => rdev::Key::KeyA,
            PTTKey::KeyS => rdev::Key::KeyS,
            PTTKey::KeyD => rdev::Key::KeyD,
            PTTKey::KeyF => rdev::Key::KeyF,
            PTTKey::KeyG => rdev::Key::KeyG,
            PTTKey::KeyH => rdev::Key::KeyH,
            PTTKey::KeyJ => rdev::Key::KeyJ,
            PTTKey::KeyK => rdev::Key::KeyK,
            PTTKey::KeyL => rdev::Key::KeyL,
            PTTKey::SemiColon => rdev::Key::SemiColon,
            PTTKey::Quote => rdev::Key::Quote,
            PTTKey::BackSlash => rdev::Key::BackSlash,
            PTTKey::IntlBackslash => rdev::Key::IntlBackslash,
            PTTKey::KeyZ => rdev::Key::KeyZ,
            PTTKey::KeyX => rdev::Key::KeyX,
            PTTKey::KeyC => rdev::Key::KeyC,
            PTTKey::KeyV => rdev::Key::KeyV,
            PTTKey::KeyB => rdev::Key::KeyB,
            PTTKey::KeyN => rdev::Key::KeyN,
            PTTKey::KeyM => rdev::Key::KeyM,
            PTTKey::Comma => rdev::Key::Comma,
            PTTKey::Dot => rdev::Key::Dot,
            PTTKey::Slash => rdev::Key::Slash,
            PTTKey::Insert => rdev::Key::Insert,
            PTTKey::KpReturn => rdev::Key::KpReturn,
            PTTKey::KpMinus => rdev::Key::KpMinus,
            PTTKey::KpPlus => rdev::Key::KpPlus,
            PTTKey::KpMultiply => rdev::Key::KpMultiply,
            PTTKey::KpDivide => rdev::Key::KpDivide,
            PTTKey::Kp0 => rdev::Key::Kp0,
            PTTKey::Kp1 => rdev::Key::Kp1,
            PTTKey::Kp2 => rdev::Key::Kp2,
            PTTKey::Kp3 => rdev::Key::Kp3,
            PTTKey::Kp4 => rdev::Key::Kp4,
            PTTKey::Kp5 => rdev::Key::Kp5,
            PTTKey::Kp6 => rdev::Key::Kp6,
            PTTKey::Kp7 => rdev::Key::Kp7,
            PTTKey::Kp8 => rdev::Key::Kp8,
            PTTKey::Kp9 => rdev::Key::Kp9,
            PTTKey::KpDelete => rdev::Key::KpDelete,
            PTTKey::Function => rdev::Key::Function,
            PTTKey::Unknown(code) => rdev::Key::Unknown(code),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let _ = dotenv();

    match opt.subcommands {
        Some(subcommand) => {
            match subcommand {
                SubCommands::ShowKeyPresses => {
                    println!("Press keys to see their codes. Press Ctrl+C to exit. Once you've figured out what key you want to use for push to talk, pass it to easy-tran using the --ptt-key argument. Or pass the number to the --special-ptt-key argument if the key is Unknown(number).");

                    fn show_keys_callback(event: Event) {
                        if let rdev::EventType::KeyPress(key) = event.event_type {
                            println!("Key pressed: {:?}", key);
                        }
                    }

                    // This will block.
                    if let Err(error) = listen(show_keys_callback) {
                        println!("Error: {:?}", error)
                    }
                }
                SubCommands::ListDevices => {
                    let host = cpal::default_host();

                    // Set up the input device and stream with the default input config.
                    host.default_input_device();
                    let devices = host
                        .input_devices()
                        .context("Failed to get list of input devices")?;

                    for device in devices {
                        let device_name = match device.name() {
                            Ok(name) => name,
                            Err(err) => {
                                println!("Error: Failed to get device name: {:?}", err);
                                continue;
                            }
                        };
                        println!("{:?}", device_name);
                    }
                }
            }

            Ok(())
        }
        // Run transcription
        None => {
            // figure out ptt key
            let ptt_key = match opt.ptt_key {
                Some(ptt_key) => ptt_key.into(),
                None => match opt.special_ptt_key {
                    Some(special_ptt_key) => rdev::Key::Unknown(special_ptt_key),
                    None => {
                        println!("No push to talk key specified. Please pass a key using the --ptt-key argument or the --special-ptt-key argument.");
                        return Ok(());
                    }
                },
            };

            if let Some(api_key) = opt.api_key {
                env::set_var("OPENAI_API_KEY", api_key);
            }

            // Fail if OPENAI_API_KEY is not set
            if env::var("OPENAI_API_KEY").is_err() {
                println!("OPENAI_API_KEY not set. Please pass your API key as an argument or assign is to the 'OPENAI_API_KEY' env var using terminal or .env file.");
                return Ok(());
            }

            let mut recorder = rec::Recorder::new();
            let client = Client::new();
            let runtime =
                tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
            let mut enigo =
                Enigo::new(&EnigoSettings::default()).context("Failed to create enigo")?;

            let tmp_dir = tempdir()?;
            // println!("{:?}", tmp_dir.path());
            let voice_tmp_path = tmp_dir.path().join("voice_tmp.wav");

            let mut recording_start = std::time::SystemTime::now();
            let mut key_pressed = false;

            let callback = move |event: Event| {
                let key_to_check = ptt_key;
                match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        if key == key_to_check && !key_pressed {
                            key_pressed = true;
                            // handle key press
                            recording_start = std::time::SystemTime::now();
                            match recorder.start_recording(&voice_tmp_path, Some(&opt.device)) {
                                Ok(_) => (),
                                Err(err) => println!("Error: Failed to start recording: {:?}", err),
                            }
                        }
                    }
                    rdev::EventType::KeyRelease(key) => {
                        if key == key_to_check && key_pressed {
                            key_pressed = false;
                            // handle key release

                            // get elapsed time since recording started
                            let elapsed = match recording_start.elapsed() {
                                Ok(elapsed) => elapsed,
                                Err(err) => {
                                    println!(
                            "Error: Failed to get elapsed recording time. Skipping transcription: \n\n{}",err
                        );
                                    return;
                                }
                            };
                            match recorder.stop_recording() {
                                Ok(_) => (),
                                Err(err) => {
                                    println!("Error: Failed to stop recording: {:?}", err);
                                    return;
                                }
                            }

                            // Whisper API can't handle less than 0.1 seconds of audio.
                            // So we'll only transcribe if the recording is longer than 0.2 seconds.
                            if elapsed.as_secs_f32() > 0.2 {
                                let mut transcription = match runtime
                                    .block_on(trans::transcribe(&client, &voice_tmp_path))
                                {
                                    Ok(transcription) => transcription,
                                    Err(err) => {
                                        println!("Error: Failed to transcribe audio: {:?}", err);
                                        return;
                                    }
                                };

                                // if let Some(last_char) = transcription.chars().last() {
                                //     if last_char != '.'
                                //         && last_char != '?'
                                //         && last_char != '!'
                                //         && last_char != ','
                                //     {
                                //         transcription.push('.');
                                //     }
                                // }
                                transcription.push(' ');

                                enigo.key_sequence(&transcription);
                            } else {
                                println!("Recording too short");
                            }
                        }
                    }
                    _ => (),
                }
            };

            // This will block.
            if let Err(error) = listen(callback) {
                println!("Error: {:?}", error)
            }

            Ok(())
        }
    }
}
