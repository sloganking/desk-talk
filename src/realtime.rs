//! Realtime streaming transcription using the OpenAI Realtime API.
//!
//! Unlike the default flow (record a WAV file, then transcribe the whole thing
//! once the push-to-talk key is released), this module opens a WebSocket to
//! OpenAI, streams microphone audio as it is captured, and receives transcript
//! deltas as they are produced. Those deltas are typed into the focused window
//! live, so text appears while you are still speaking.
//!
//! This is intentionally a separate code path so the original, non-realtime
//! behavior is preserved untouched.

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use enigo::{Enigo, KeyboardControllable};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

/// Target sample rate required by the OpenAI Realtime API (pcm16, mono).
const TARGET_SAMPLE_RATE: u32 = 24_000;

/// Returns the path to the realtime debug log file (in the app config dir, or
/// the temp dir as a fallback). The release GUI build has no console, so this
/// file is the only way to see what the realtime session is doing.
pub fn log_path() -> std::path::PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("com", "desk-talk", "desk-talk") {
        let dir = dirs.config_dir();
        let _ = std::fs::create_dir_all(dir);
        dir.join("realtime.log")
    } else {
        std::env::temp_dir().join("desk-talk-realtime.log")
    }
}

/// Appends a timestamped line to the realtime log file (and stderr for debug
/// builds).
fn log_line(msg: &str) {
    use std::io::Write;
    let line = format!(
        "[{}] {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        msg
    );
    eprint!("{line}");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// How much audio (in samples at the target rate) to buffer before sending an
/// `input_audio_buffer.append` event. ~120ms keeps latency low without spamming
/// tiny messages.
const APPEND_BATCH_SAMPLES: usize = (TARGET_SAMPLE_RATE as usize * 120) / 1000;

/// Once we commit the audio buffer on key release, how long to keep draining
/// the socket waiting for trailing transcript deltas before giving up.
const DRAIN_IDLE_TIMEOUT: Duration = Duration::from_millis(1500);

/// A live realtime transcription session. Created on push-to-talk key down and
/// finished on key up via [`RealtimeSession::stop`].
pub struct RealtimeSession {
    stop_tx: flume::Sender<()>,
    result_rx: flume::Receiver<Result<String>>,
    handle: Option<JoinHandle<()>>,
}

impl RealtimeSession {
    /// Starts capturing the microphone and streaming it to OpenAI. Returns
    /// quickly; all work happens on a background thread. Transcript deltas are
    /// typed into the focused window as they arrive.
    pub fn start(
        api_key: String,
        device: String,
        model: String,
        language: Option<String>,
        cap_first: bool,
    ) -> Result<Self> {
        let (stop_tx, stop_rx) = flume::bounded::<()>(1);
        let (result_tx, result_rx) = flume::bounded::<Result<String>>(1);

        let handle = thread::Builder::new()
            .name("realtime-session".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = result_tx.send(Err(anyhow!("Failed to build runtime: {e}")));
                        return;
                    }
                };

                let result = rt.block_on(run_session(
                    api_key, device, model, language, cap_first, stop_rx,
                ));
                let _ = result_tx.send(result);
            })
            .context("Failed to spawn realtime session thread")?;

        Ok(Self {
            stop_tx,
            result_rx,
            handle: Some(handle),
        })
    }

    /// Signals the session to stop capturing, commits the audio buffer, waits
    /// for the final transcript deltas, and returns the full text that was
    /// transcribed (which mirrors what was typed live).
    pub fn stop(mut self) -> Result<String> {
        let _ = self.stop_tx.send(());
        let result = self
            .result_rx
            .recv()
            .unwrap_or_else(|_| Err(anyhow!("Realtime session ended unexpectedly")));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        result
    }
}

/// Stateful linear resampler that converts an arbitrary input rate to the
/// target rate. Carries fractional position and the last sample across calls so
/// successive audio buffers stitch together seamlessly.
struct Resampler {
    step: f64,
    pos: f64,
    prev: f32,
}

impl Resampler {
    fn new(in_rate: f64, out_rate: f64) -> Self {
        Self {
            step: in_rate / out_rate,
            pos: 0.0,
            prev: 0.0,
        }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<i16>) {
        if input.is_empty() {
            return;
        }
        let n = input.len();
        // Virtual buffer E where E[0] = prev (last sample of previous call) and
        // E[1..=n] = input[0..n]. We emit output samples at fractional positions
        // self.pos, self.pos + step, ... while floor(pos) < n.
        let mut p = self.pos;
        loop {
            let idx = p.floor() as usize;
            if idx >= n {
                break;
            }
            let frac = p - idx as f64;
            let a = if idx == 0 { self.prev } else { input[idx - 1] };
            let b = input[idx];
            let s = a as f64 + (b as f64 - a as f64) * frac;
            let clamped = s.clamp(-1.0, 1.0);
            out.push((clamped * 32767.0) as i16);
            p += self.step;
        }
        // Carry remainder relative to the next call (whose E[0] is input[n-1]).
        self.pos = p - n as f64;
        self.prev = input[n - 1];
    }
}

/// Builds a cpal input stream that downmixes to mono, resamples to the target
/// rate, converts to PCM16, and forwards batches of samples through `sender`.
/// Returns the live stream (kept alive by the caller).
fn build_input_stream(
    device_name: &str,
    sender: flume::Sender<Vec<i16>>,
) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = match if device_name == "default" {
        host.default_input_device()
    } else {
        host.input_devices()
            .context("Failed to get list of input devices")?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
    } {
        Some(d) => d,
        None => bail!("Failed to find input device '{device_name}'"),
    };

    let config = device
        .default_input_config()
        .context("Failed to get default input config")?;
    let channels = config.channels() as usize;
    let in_rate = config.sample_rate().0 as f64;

    let err_fn = |err: cpal::StreamError| eprintln!("Realtime audio stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_typed_stream::<f32>(&device, &config.into(), channels, in_rate, sender, err_fn)?
        }
        cpal::SampleFormat::I16 => {
            build_typed_stream::<i16>(&device, &config.into(), channels, in_rate, sender, err_fn)?
        }
        cpal::SampleFormat::I32 => {
            build_typed_stream::<i32>(&device, &config.into(), channels, in_rate, sender, err_fn)?
        }
        cpal::SampleFormat::I8 => {
            build_typed_stream::<i8>(&device, &config.into(), channels, in_rate, sender, err_fn)?
        }
        other => bail!("Unsupported sample format '{other}'"),
    };

    stream.play().context("Failed to start input stream")?;
    Ok(stream)
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    in_rate: f64,
    sender: flume::Sender<Vec<i16>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let mut resampler = Resampler::new(in_rate, TARGET_SAMPLE_RATE as f64);
    let stream = device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if channels == 0 {
                    return;
                }
                let mut mono: Vec<f32> = Vec::with_capacity(data.len() / channels);
                for frame in data.chunks(channels) {
                    let mut sum = 0.0f32;
                    for &s in frame {
                        sum += f32::from_sample(s);
                    }
                    mono.push(sum / channels as f32);
                }
                let mut out: Vec<i16> = Vec::new();
                resampler.process(&mono, &mut out);
                if !out.is_empty() {
                    let _ = sender.send(out);
                }
            },
            err_fn,
            None,
        )
        .context("Failed to build input stream")?;
    Ok(stream)
}

fn pcm16_to_base64(samples: &[i16]) -> String {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Extracts a transcript delta/text from a Realtime API event, if present.
fn extract_text(event: &Value, field: &str) -> Option<String> {
    event
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

async fn run_session(
    api_key: String,
    device: String,
    model: String,
    language: Option<String>,
    cap_first: bool,
    stop_rx: flume::Receiver<()>,
) -> Result<String> {
    // --- Connect ---------------------------------------------------------
    let url = "wss://api.openai.com/v1/realtime?intent=transcription";
    log_line(&format!(
        "Starting realtime session (model: {model}, device: {device})"
    ));
    let mut request = url
        .into_client_request()
        .context("Failed to build websocket request")?;
    {
        let headers = request.headers_mut();
        headers.insert(
            "Authorization",
            format!("Bearer {api_key}")
                .parse()
                .context("Invalid Authorization header")?,
        );
    }

    let (ws_stream, _resp) = match tokio_tungstenite::connect_async(request).await {
        Ok(ok) => {
            log_line("WebSocket connected");
            ok
        }
        Err(e) => {
            log_line(&format!("Failed to connect to OpenAI Realtime API: {e}"));
            return Err(anyhow!("Failed to connect to OpenAI Realtime API: {e}"));
        }
    };
    let (mut write, mut read) = ws_stream.split();

    // --- Configure the transcription session (GA schema) -----------------
    // gpt-realtime-whisper is a natively-streaming model: it emits partial
    // transcript text *while* you speak, so we disable server VAD and commit
    // manually on key release. Other models (e.g. gpt-4o-transcribe) only
    // produce text per committed turn, so we use server VAD for them.
    let is_streaming_whisper = model.contains("whisper");

    let mut transcription = json!({ "model": model });
    if let Some(lang) = language {
        transcription["language"] = Value::String(lang);
    }
    if is_streaming_whisper {
        // Latency/accuracy tradeoff: minimal | low | medium | high | xhigh.
        transcription["delay"] = Value::String("low".to_string());
    }

    let turn_detection = if is_streaming_whisper {
        Value::Null
    } else {
        json!({
            "type": "server_vad",
            "threshold": 0.5,
            "prefix_padding_ms": 300,
            "silence_duration_ms": 500
        })
    };

    let session_update = json!({
        "type": "session.update",
        "session": {
            "type": "transcription",
            "audio": {
                "input": {
                    "format": { "type": "audio/pcm", "rate": TARGET_SAMPLE_RATE },
                    "transcription": transcription,
                    "turn_detection": turn_detection
                }
            }
        }
    });
    if let Err(e) = write
        .send(Message::Text(session_update.to_string()))
        .await
    {
        log_line(&format!("Failed to send session config: {e}"));
        return Err(anyhow!("Failed to send session config: {e}"));
    }
    log_line("Sent session.update (transcription config)");

    // --- Start capturing audio -------------------------------------------
    let (audio_tx, audio_rx) = flume::unbounded::<Vec<i16>>();
    let _stream = build_input_stream(&device, audio_tx)
        .context("Failed to start microphone capture")?;

    let mut enigo = Enigo::new();
    let mut accumulated = String::new();
    let mut typed_first = false;
    // When a speech segment completes, the next segment's text won't include a
    // leading space, so we insert one ourselves before the next delta.
    let mut pending_space = false;
    let mut batch: Vec<i16> = Vec::with_capacity(APPEND_BATCH_SAMPLES * 2);

    // Types a delta into the focused window and records it.
    let mut handle_delta =
        |text: &str, enigo: &mut Enigo, accumulated: &mut String, typed_first: &mut bool| {
            if text.is_empty() {
                return;
            }
            let mut to_type = text.to_string();
            if cap_first && !*typed_first {
                capitalize_first_letter(&mut to_type);
            }
            *typed_first = true;
            accumulated.push_str(&to_type);
            enigo.key_sequence(&to_type);
        };

    // --- Phase 1: stream audio until the key is released -----------------
    loop {
        tokio::select! {
            biased;

            // Key released -> finish capturing.
            _ = stop_rx.recv_async() => {
                break;
            }

            // New audio captured -> batch & forward.
            audio = audio_rx.recv_async() => {
                if let Ok(samples) = audio {
                    batch.extend_from_slice(&samples);
                    if batch.len() >= APPEND_BATCH_SAMPLES {
                        let b64 = pcm16_to_base64(&batch);
                        batch.clear();
                        let msg = json!({ "type": "input_audio_buffer.append", "audio": b64 });
                        if write.send(Message::Text(msg.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Incoming transcript events.
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        process_event(&text, &mut enigo, &mut accumulated, &mut typed_first, &mut pending_space, &mut handle_delta)?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => bail!("Realtime websocket error: {e}"),
                }
            }
        }
    }

    // --- Flush any remaining audio and commit ----------------------------
    // Drain anything captured but not yet batched.
    while let Ok(samples) = audio_rx.try_recv() {
        batch.extend_from_slice(&samples);
    }
    if !batch.is_empty() {
        let b64 = pcm16_to_base64(&batch);
        batch.clear();
        let msg = json!({ "type": "input_audio_buffer.append", "audio": b64 });
        let _ = write.send(Message::Text(msg.to_string())).await;
    }
    let _ = write
        .send(Message::Text(
            json!({ "type": "input_audio_buffer.commit" }).to_string(),
        ))
        .await;
    log_line("Committed audio buffer, draining final transcript");

    // Stop the microphone now that we've committed.
    drop(_stream);

    // --- Phase 2: drain remaining transcript deltas ----------------------
    loop {
        match tokio::time::timeout(DRAIN_IDLE_TIMEOUT, read.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                process_event(
                    &text,
                    &mut enigo,
                    &mut accumulated,
                    &mut typed_first,
                    &mut pending_space,
                    &mut handle_delta,
                )?;
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(_))) => break,
            // No new events for DRAIN_IDLE_TIMEOUT -> assume we're done.
            Err(_) => break,
        }
    }

    let _ = write.send(Message::Close(None)).await;
    log_line(&format!("Session finished. Transcript: {accumulated:?}"));
    Ok(accumulated)
}

#[allow(clippy::too_many_arguments)]
fn process_event(
    text: &str,
    enigo: &mut Enigo,
    accumulated: &mut String,
    typed_first: &mut bool,
    pending_space: &mut bool,
    handle_delta: &mut impl FnMut(&str, &mut Enigo, &mut String, &mut bool),
) -> Result<()> {
    let event: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match event_type {
        // Incremental transcript text.
        "conversation.item.input_audio_transcription.delta" => {
            if let Some(delta) = extract_text(&event, "delta") {
                // Insert a space at a segment boundary if neither side has one.
                let needs_space = *pending_space
                    && !accumulated.is_empty()
                    && !accumulated.ends_with(char::is_whitespace)
                    && !delta.starts_with(char::is_whitespace);
                *pending_space = false;
                if needs_space {
                    handle_delta(&format!(" {delta}"), enigo, accumulated, typed_first);
                } else {
                    handle_delta(&delta, enigo, accumulated, typed_first);
                }
            }
        }
        // Full transcript for a committed segment (already typed via deltas).
        // Mark that the next segment should start with a space.
        "conversation.item.input_audio_transcription.completed" => {
            if let Some(t) = extract_text(&event, "transcript") {
                log_line(&format!("Segment completed: {t:?}"));
            }
            if *typed_first {
                *pending_space = true;
            }
        }
        // Transcription failed for a segment.
        "conversation.item.input_audio_transcription.failed" => {
            log_line(&format!("Transcription failed event: {text}"));
            bail!("Realtime transcription failed");
        }
        // Session lifecycle (helpful for confirming config was accepted).
        "session.created" | "session.updated" | "transcription_session.created"
        | "transcription_session.updated" => {
            log_line(&format!("Server: {event_type}"));
        }
        // Errors from the API.
        "error" => {
            log_line(&format!("Realtime API error event: {text}"));
            let message = event
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown realtime error");
            bail!("Realtime API error: {message}");
        }
        other => {
            log_line(&format!("Server event: {other}"));
        }
    }
    Ok(())
}

fn capitalize_first_letter(s: &mut String) {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        let uppercase: String = first.to_uppercase().collect();
        let first_char_len = first.len_utf8();
        s.replace_range(0..first_char_len, &uppercase);
    }
}
