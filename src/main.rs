use anyhow::Context;
use async_openai::Client;
use dotenvy::dotenv;
use enigo::{Enigo, KeyboardControllable};
use std::env;
use tempfile::tempdir;
mod transcribe;
use std::thread::{self, sleep};
use transcribe::trans;
mod record;
use clap::{Parser, Subcommand, ValueEnum};
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use cpal::traits::{DeviceTrait, HostTrait};
use default_device_sink::DefaultDeviceSink;
use rdev::{listen, Event};
use record::rec;
use rodio::source::{SineWave, Source};
use rodio::Decoder;
use std::error::Error;
use std::io::{BufReader, Cursor};
use std::sync::mpsc;
use std::time::Duration;
mod easy_rdev_key;
use crate::easy_rdev_key::PTTKey;
use mutter::ModelType;

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

    /// Use local whisper model instead of OpenAI API
    #[arg(long)]
    local: bool,

    /// The local whisper model to use. Requires --local.
    #[arg(long, value_enum, requires = "local")]
    model: Option<LocalModel>,

    /// Ensures the first letter of the transcription is capitalized.
    #[arg(short, long)]
    cap_first: bool,

    /// Ensures the transcription ends with a space character. This lets you transcribe repeatedly without typing a space character between transcriptions to separate the words.
    ///
    /// This is a flag and not default behavior because in some natural languages it doesn't make sense to put a space after the transcription.
    #[arg(short, long)]
    space: bool,

    /// Passing this flag will emulate the keyboard for typing the characters, instead of pressing Ctrl-V and pasting the text, which is the default behavior.
    /// This may be needed to pass text to a terminal, which would not accept pasting or something else.
    #[arg(short, long)]
    type_chars: bool,

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

#[derive(ValueEnum, Clone, Debug, Copy)]
pub enum LocalModel {
    TinyEn,
    Tiny,
    BaseEn,
    Base,
    SmallEn,
    Small,
    MediumEn,
    Medium,
    LargeV1,
    LargeV2,
    LargeV3,
}

impl From<LocalModel> for ModelType {
    fn from(model: LocalModel) -> Self {
        match model {
            LocalModel::TinyEn => ModelType::TinyEn,
            LocalModel::Tiny => ModelType::Tiny,
            LocalModel::BaseEn => ModelType::BaseEn,
            LocalModel::Base => ModelType::Base,
            LocalModel::SmallEn => ModelType::SmallEn,
            LocalModel::Small => ModelType::Small,
            LocalModel::MediumEn => ModelType::MediumEn,
            LocalModel::Medium => ModelType::Medium,
            LocalModel::LargeV1 => ModelType::LargeV1,
            LocalModel::LargeV2 => ModelType::LargeV2,
            LocalModel::LargeV3 => ModelType::LargeV3,
        }
    }
}

fn capitalize_first_letter(s: &mut String) {
    let mut c = s.chars();
    if let Some(f) = c.next() {
        let uppercase: String = f.to_uppercase().collect();
        let first_char_len = f.len_utf8();
        s.replace_range(0..first_char_len, &uppercase);
    }
}

static TICK_BYTES: &[u8] = include_bytes!("../assets/tick.mp3");
static FAILED_BYTES: &[u8] = include_bytes!("../assets/failed.mp3");

fn tick_loop(stop_rx: mpsc::Receiver<()>) {
    let tick_sink = DefaultDeviceSink::new();
    loop {
        if stop_rx.try_recv().is_ok() {
            tick_sink.stop();
            break;
        }
        if tick_sink.empty() {
            let cursor = Cursor::new(TICK_BYTES);
            if let Ok(decoder) = Decoder::new(BufReader::new(cursor)) {
                tick_sink.stop();
                tick_sink.append(decoder);
            } else {
                tick_sink.stop();
                tick_sink.append(
                    SineWave::new(880.0)
                        .take_duration(Duration::from_millis(50))
                        .amplify(0.20),
                );
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn play_failure_sound() {
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

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let _ = dotenv();

    let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();

    match opt.subcommands {
        Some(subcommand) => {
            match subcommand {
                SubCommands::ShowKeyPresses => {
                    println!("Press keys to see their codes. Press Ctrl+C to exit. Once you've figured out what key you want to use for push to talk, pass it to desk-talk using the --ptt-key argument. Or pass the number to the --special-ptt-key argument if the key is Unknown(number).");

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

                    let test = host.default_output_device().unwrap();

                    println!("default output_device: {:?}", test.name());
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

                    // devices
                    //     .filter_map(|device| device.name().ok())
                    //     .for_each(|device_name| println!("{:?}", device_name));
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

            if !opt.local {
                if let Some(api_key) = opt.api_key {
                    env::set_var("OPENAI_API_KEY", api_key);
                }

                if env::var("OPENAI_API_KEY").is_err() {
                    println!("OPENAI_API_KEY not set. Please pass your API key as an argument or assign is to the 'OPENAI_API_KEY' env var using terminal or .env file.");
                    return Ok(());
                }
            } else if opt.model.is_none() {
                println!("--model must be specified when using --local");
                return Ok(());
            }

            let (tx, rx): (flume::Sender<Event>, flume::Receiver<Event>) = flume::unbounded();

            // create key handler thread
            thread::spawn(move || {
                let mut recorder = rec::Recorder::new();
                let client = Client::new();
                let runtime = tokio::runtime::Runtime::new()
                    .context("Failed to create tokio runtime")
                    .unwrap();
                let mut enigo = Enigo::new();

                let tmp_dir = tempdir().unwrap();
                // println!("{:?}", tmp_dir.path());
                let voice_tmp_path = tmp_dir.path().join("voice_tmp.wav");

                let mut recording_start = std::time::SystemTime::now();
                let mut key_pressed = false;
                let key_to_check = ptt_key;

                for event in rx.iter() {
                    // println!("Received: {:?}", event);
                    match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            if key == key_to_check && !key_pressed {
                                key_pressed = true;
                                // handle key press
                                recording_start = std::time::SystemTime::now();
                                match recorder.start_recording(&voice_tmp_path, Some(&opt.device)) {
                                    Ok(_) => (),
                                    Err(err) => {
                                        println!("Error: Failed to start recording: {:?}", err)
                                    }
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
                                        println!("Error: Failed to get elapsed recording time. Skipping transcription: \n\n{}",err);
                                        continue;
                                    }
                                };
                                match recorder.stop_recording() {
                                    Ok(_) => (),
                                    Err(err) => {
                                        println!("Error: Failed to stop recording: {:?}", err);
                                        continue;
                                    }
                                }

                                // future::timeout(
                                //     Duration::from_secs(10),
                                //     trans::transcribe(&client, &voice_tmp_path),
                                // )
                                // .await;

                                // Whisper API can't handle less than 0.1 seconds of audio.
                                // So we'll only transcribe if the recording is longer than 0.2 seconds.
                                if elapsed.as_secs_f32() > 0.2 {
                                    let (tick_tx, tick_rx) = mpsc::channel();
                                    let tick_handle = thread::spawn(move || tick_loop(tick_rx));

                                    let transcription_result = if opt.local {
                                        let model = opt
                                            .model
                                            .expect("--model required when --local is used");
                                        trans::transcribe_local(&voice_tmp_path, model.into())
                                    } else {
                                        runtime.block_on(trans::transcribe_with_retry(
                                            &client,
                                            &voice_tmp_path,
                                            3,
                                        ))
                                    };

                                    let _ = tick_tx.send(());
                                    let _ = tick_handle.join();

                                    let mut transcription = match transcription_result {
                                        Ok(transcription) => transcription,
                                        Err(err) => {
                                            println!(
                                                "Error: Failed to transcribe audio: {:?}",
                                                err
                                            );
                                            play_failure_sound();
                                            continue;
                                        }
                                    };

                                    // Transctiption post processing
                                    {
                                        if opt.cap_first {
                                            capitalize_first_letter(&mut transcription);
                                        }

                                        if opt.space {
                                            if let Some(last_char) = transcription.chars().last() {
                                                if last_char != ' ' {
                                                    transcription.push(' ');
                                                }
                                            }
                                        }

                                        // Remove ellipses.
                                        transcription = transcription.replace("...", "");
                                    }

                                    if transcription.is_empty() {
                                        println!("No transcription");
                                        play_failure_sound();
                                        continue;
                                    }

                                    if opt.type_chars {
                                        enigo.key_sequence(&transcription);
                                    } else {
                                        // paste from clipboard

                                        // get the clipboard contents so we can restore it later
                                        let clip_tmp_result = clipboard.get_contents();

                                        // Set and paste Clipboard Contents
                                        match clipboard.set_contents(transcription) {
                                            Ok(_) => {
                                                enigo.key_sequence_parse("{+CTRL}");
                                                sleep(Duration::from_millis(100));
                                                enigo.key_sequence_parse("v");
                                                sleep(Duration::from_millis(100));
                                                enigo.key_sequence_parse("{-CTRL}");
                                                sleep(Duration::from_millis(100));

                                                // restore the clipboard contents
                                                if let Ok(clip_tmp) = clip_tmp_result {
                                                    if let Err(err) =
                                                        clipboard.set_contents(clip_tmp)
                                                    {
                                                        println!(
                                                        "Error restoring clipboard contents: {}",
                                                        err
                                                    );
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                println!(
                                                    "Error: Failed to set clipboard contents: {:?}",
                                                    err
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                } else {
                                    println!("Recording too short");
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });

            // Have this main thread recieve events and send them to the key handler thread
            {
                let callback = move |event: Event| {
                    tx.send(event).unwrap();
                };

                // This will block.
                if let Err(error) = listen(callback) {
                    println!("Error: {:?}", error)
                }
            }

            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_first_letter_works() {
        let mut s = String::from("hello");
        capitalize_first_letter(&mut s);
        assert_eq!(s, "Hello");
    }
}
