use crate::app_state::AppState;
use crate::config::AppConfig;
use crate::record::rec;
use crate::transcribe::trans;
use async_openai::Client;
use clipboard::{ClipboardContext, ClipboardProvider};
use default_device_sink::DefaultDeviceSink;
use enigo::{Enigo, KeyboardControllable};
use parking_lot::Mutex;
use rdev::{listen, Event};
use rodio::{source::SineWave, Decoder, Source};
use std::collections::VecDeque;
use std::io::{BufReader, Cursor};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;
use tempfile::tempdir;

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

fn capitalize_first_letter(s: &mut String) {
    let mut c = s.chars();
    if let Some(f) = c.next() {
        let uppercase: String = f.to_uppercase().collect();
        let first_char_len = f.len_utf8();
        s.replace_range(0..first_char_len, &uppercase);
    }
}

pub struct TranscriptionEngine {
    app_state: AppState,
    stop_signal: Arc<Mutex<bool>>,
}

impl TranscriptionEngine {
    pub fn new(app_state: AppState) -> Self {
        Self {
            app_state,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) -> anyhow::Result<()> {
        use std::env;

        let config = self.app_state.config.read().clone();

        // Validate configuration
        let ptt_key = config
            .get_ptt_key()
            .ok_or_else(|| anyhow::anyhow!("No push-to-talk key configured"))?;

        println!("Transcription engine using PTT key: {:?}", ptt_key);

        if !config.use_local {
            if let Some(ref api_key) = config.api_key {
                // Set environment variable for OpenAI client
                env::set_var("OPENAI_API_KEY", api_key);
                println!("OpenAI API key set from config");
            } else {
                return Err(anyhow::anyhow!("No OpenAI API key configured"));
            }
        }

        if config.use_local && config.local_model.is_none() {
            return Err(anyhow::anyhow!("No local model selected"));
        }

        println!("Configuration validated successfully");

        let (tx, rx): (flume::Sender<Event>, flume::Receiver<Event>) = flume::unbounded();
        let app_state = self.app_state.clone();
        let stop_signal_for_key_thread = self.stop_signal.clone();

        // Start key handler thread
        thread::spawn(move || {
            Self::key_handler_thread(rx, app_state, config, stop_signal_for_key_thread);
        });

        // Start event listener thread
        let stop_signal_listener = self.stop_signal.clone();
        thread::spawn(move || {
            println!("Event listener thread started");
            let callback = move |event: Event| {
                if *stop_signal_listener.lock() {
                    // Signal to stop listening
                    return;
                }
                if let Err(e) = tx.send(event) {
                    eprintln!("Failed to send event to key handler: {}", e);
                }
            };

            if let Err(error) = listen(callback) {
                eprintln!("Error in event listener: {:?}", error);
            }
        });

        self.app_state.start_transcription();
        println!("Transcription engine fully initialized - listening for key presses...");
        Ok(())
    }

    pub fn stop(&self) {
        *self.stop_signal.lock() = true;
        self.app_state.stop_transcription();
    }

    fn key_handler_thread(
        rx: flume::Receiver<Event>,
        app_state: AppState,
        opt: AppConfig,
        stop_signal: Arc<Mutex<bool>>,
    ) {
        let mut recorder = rec::Recorder::new();
        let client = Client::new();
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let mut enigo = Enigo::new();
        let mut clipboard: ClipboardContext = match ClipboardProvider::new() {
            Ok(provider) => provider,
            Err(err) => {
                eprintln!("Failed to access clipboard: {:?}", err);
                return;
            }
        };

        let mut wpm_history: VecDeque<f64> = VecDeque::new();
        let mut wpm_sum: f64 = 0.0;
        const WPM_ROLLING_MAX: usize = 1000;

        let tmp_dir = tempdir().unwrap();
        let voice_tmp_path = tmp_dir.path().join("voice_tmp.wav");

        let mut recording_start = std::time::SystemTime::now();
        let mut key_pressed = false;
        let key_to_check = opt.get_ptt_key().unwrap();

        println!(
            "Key handler thread started, waiting for PTT key: {:?}",
            key_to_check
        );

        for event in rx.iter() {
            if *stop_signal.lock() {
                println!("Stop signal received - shutting down key handler");
                break;
            }
            match event.event_type {
                rdev::EventType::KeyPress(key) => {
                    if key == key_to_check && !key_pressed {
                        println!("PTT key pressed - starting recording");
                        key_pressed = true;
                        recording_start = std::time::SystemTime::now();
                        match recorder.start_recording(&voice_tmp_path, Some(&opt.device)) {
                            Ok(_) => println!("Recording started successfully"),
                            Err(err) => {
                                eprintln!("Error: Failed to start recording: {:?}", err);
                                continue;
                            }
                        }
                    }
                }
                rdev::EventType::KeyRelease(key) => {
                    if key == key_to_check && key_pressed {
                        println!("PTT key released - stopping recording");
                        key_pressed = false;

                        let elapsed = match recording_start.elapsed() {
                            Ok(elapsed) => elapsed,
                            Err(err) => {
                                eprintln!("Error: Failed to get elapsed recording time: {}", err);
                                continue;
                            }
                        };

                        match recorder.stop_recording() {
                            Ok(_) => (),
                            Err(err) => {
                                eprintln!("Error: Failed to stop recording: {:?}", err);
                                continue;
                            }
                        }

                        if elapsed.as_secs_f32() > 0.2 {
                            let (tick_tx, tick_rx) = mpsc::channel();
                            let tick_handle = thread::spawn(move || tick_loop(tick_rx));

                            let transcription_result = if opt.use_local {
                                let model = opt
                                    .local_model
                                    .as_ref()
                                    .and_then(|m| Self::parse_model(m))
                                    .expect("Valid model required");
                                trans::transcribe_local(&voice_tmp_path, model)
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
                                    eprintln!("Error: Failed to transcribe audio: {:?}", err);
                                    play_failure_sound();
                                    continue;
                                }
                            };

                            // Post-processing
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

                            transcription = transcription.replace("...", "");

                            if transcription.is_empty() {
                                println!("No transcription");
                                play_failure_sound();
                                continue;
                            }

                            let word_count = transcription.split_whitespace().count();
                            let duration_secs = elapsed.as_secs_f64();

                            if opt.type_chars {
                                enigo.key_sequence(&transcription);
                            } else {
                                let clip_tmp_result = clipboard.get_contents();

                                match clipboard.set_contents(transcription.clone()) {
                                    Ok(_) => {
                                        enigo.key_sequence_parse("{+CTRL}");
                                        sleep(Duration::from_millis(100));
                                        enigo.key_sequence_parse("v");
                                        sleep(Duration::from_millis(100));
                                        enigo.key_sequence_parse("{-CTRL}");
                                        sleep(Duration::from_millis(100));

                                        if let Ok(clip_tmp) = clip_tmp_result {
                                            let _ = clipboard.set_contents(clip_tmp);
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Error: Failed to set clipboard: {:?}", err);
                                        continue;
                                    }
                                }
                            }

                            if duration_secs > 0.0 {
                                let wpm = (word_count as f64) * 60.0 / duration_secs;
                                wpm_history.push_back(wpm);
                                wpm_sum += wpm;
                                if wpm_history.len() > WPM_ROLLING_MAX {
                                    if let Some(removed) = wpm_history.pop_front() {
                                        wpm_sum -= removed;
                                    }
                                }
                                let avg_wpm = if !wpm_history.is_empty() {
                                    wpm_sum / (wpm_history.len() as f64)
                                } else {
                                    0.0
                                };

                                app_state.update_statistics(word_count, duration_secs, wpm);

                                println!(
                                    "WPM: {:.1} | Avg: {:.1} | Total: {} words",
                                    wpm, avg_wpm, word_count
                                );
                            }
                        } else {
                            println!("Recording too short");
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn parse_model(model: &str) -> Option<mutter::ModelType> {
        use mutter::ModelType;
        match model.to_lowercase().as_str() {
            "tiny-en" => Some(ModelType::TinyEn),
            "tiny" => Some(ModelType::Tiny),
            "base-en" => Some(ModelType::BaseEn),
            "base" => Some(ModelType::Base),
            "small-en" => Some(ModelType::SmallEn),
            "small" => Some(ModelType::Small),
            "medium-en" => Some(ModelType::MediumEn),
            "medium" => Some(ModelType::Medium),
            "large-v1" => Some(ModelType::LargeV1),
            "large-v2" => Some(ModelType::LargeV2),
            "large-v3" => Some(ModelType::LargeV3),
            _ => None,
        }
    }
}
