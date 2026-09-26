//! Everything that runs while a transcription is in flight: the "working"
//! tick sound, and the deadline on the transcription call itself.
//!
//! The rule both follow: nothing waits forever. The tick stops when it is told
//! to, when whoever could tell it has gone away (dropped sender, early return,
//! panic unwinding), or when it has run for `TICK_MAX`, whichever comes first.

use default_device_sink::DefaultDeviceSink;
use rodio::{source::SineWave, Decoder, Source};
use std::io::{BufReader, Cursor};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static TICK_BYTES: &[u8] = include_bytes!("../assets/tick.mp3");

/// How often the tick loop checks for a stop and refills the sink.
pub const TICK_POLL: Duration = Duration::from_millis(100);

/// Hard cap on ticking, as a backstop behind `transcription_deadline`.
/// The longest legitimate tick is the transcription deadline (at most
/// `TRANSCRIPTION_DEADLINE_MAX`, 8 min) plus the punctuation fix (20s timeout)
/// plus smart end punctuation (8s timeout), about 8.5 min. Ten minutes clears
/// that, so the cap only ever fires on a real hang.
pub const TICK_MAX: Duration = Duration::from_secs(600);

/// Floor and ceiling of the transcription deadline.
pub const TRANSCRIPTION_DEADLINE_MIN: Duration = Duration::from_secs(60);
pub const TRANSCRIPTION_DEADLINE_MAX: Duration = Duration::from_secs(480);

/// Deadline for transcribing `audio_secs` of audio: 60s plus twice the audio
/// length, clamped to [60s, 8min]. The remote path already has per-attempt
/// adaptive timeouts (max(20s, audio * p90 ratio * 1.5), three attempts), so
/// this is the outer bound that also covers ffmpeg, local models, and anything
/// that ignores those timeouts.
pub fn transcription_deadline(audio_secs: f64) -> Duration {
    let secs = 60.0 + 2.0 * audio_secs.max(0.0);
    Duration::from_secs_f64(secs).clamp(TRANSCRIPTION_DEADLINE_MIN, TRANSCRIPTION_DEADLINE_MAX)
}

/// Where the ticks go. The real one is the default audio device; tests use a
/// fake so they never play sound.
pub trait TickSink {
    fn empty(&self) -> bool;
    fn play_tick(&self);
    fn stop(&self);
}

struct DeviceTickSink(DefaultDeviceSink);

impl TickSink for DeviceTickSink {
    fn empty(&self) -> bool {
        self.0.empty()
    }

    fn play_tick(&self) {
        self.0.stop();
        if let Ok(decoder) = Decoder::new(BufReader::new(Cursor::new(TICK_BYTES))) {
            self.0.append(decoder);
        } else {
            self.0.append(
                SineWave::new(880.0)
                    .take_duration(Duration::from_millis(50))
                    .amplify(0.20),
            );
        }
    }

    fn stop(&self) {
        self.0.stop();
    }
}

/// Why a tick loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickEnd {
    /// Told to stop.
    Stopped,
    /// The sender went away without saying stop (early return, panic).
    Disconnected,
    /// Ran for the maximum duration.
    MaxDuration,
}

/// Plays ticks into `sink` until stopped, disconnected, or `max` elapses.
/// Keeps going only while the channel is empty and someone can still stop it.
pub fn tick_loop<S: TickSink>(
    sink: &S,
    stop_rx: &mpsc::Receiver<()>,
    poll: Duration,
    max: Duration,
) -> TickEnd {
    let started = Instant::now();
    loop {
        if started.elapsed() >= max {
            sink.stop();
            eprintln!(
                "Warning: tick sound hit its {}s cap, stopping it",
                max.as_secs()
            );
            return TickEnd::MaxDuration;
        }
        if sink.empty() {
            sink.play_tick();
        }
        match stop_rx.recv_timeout(poll) {
            Ok(()) => {
                sink.stop();
                return TickEnd::Stopped;
            }
            Err(RecvTimeoutError::Disconnected) => {
                sink.stop();
                return TickEnd::Disconnected;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

/// The tick sound as a scope guard. Ticking stops on `stop()` or when the
/// `Ticker` is dropped by any path, including panic unwinding, so no caller
/// has to remember to stop it on each return.
pub struct Ticker {
    stop_tx: Option<Sender<()>>,
    handle: Option<JoinHandle<TickEnd>>,
    end: Option<TickEnd>,
}

impl Ticker {
    /// Starts ticking on the default audio device.
    pub fn start() -> Self {
        Self::start_with(|| DeviceTickSink(DefaultDeviceSink::new()), TICK_POLL, TICK_MAX)
    }

    /// Starts ticking into the sink `make_sink` builds (on the tick thread,
    /// since audio sinks are not `Send`).
    pub fn start_with<S, F>(make_sink: F, poll: Duration, max: Duration) -> Self
    where
        S: TickSink,
        F: FnOnce() -> S + Send + 'static,
    {
        let (stop_tx, stop_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let sink = make_sink();
            tick_loop(&sink, &stop_rx, poll, max)
        });
        Self {
            stop_tx: Some(stop_tx),
            handle: Some(handle),
            end: None,
        }
    }

    /// Stops ticking and waits for the tick thread (at most one poll interval).
    /// Safe to call more than once; returns why the loop ended.
    pub fn stop(&mut self) -> Option<TickEnd> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            self.end = handle.join().ok();
        }
        self.end
    }
}

impl Drop for Ticker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Why a deadline-bounded call did not produce a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlineError {
    TimedOut(Duration),
    /// The work panicked (its thread ended without sending a result).
    Panicked,
}

impl std::fmt::Display for DeadlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeadlineError::TimedOut(d) => write!(f, "timed out after {:.0}s", d.as_secs_f64()),
            DeadlineError::Panicked => write!(f, "transcription thread panicked"),
        }
    }
}

/// Runs `work` on its own thread and waits at most `deadline` for it. On a
/// timeout the work thread is left to finish (or not) on its own; the caller
/// is free immediately.
pub fn run_with_deadline<T, F>(deadline: Duration, work: F) -> Result<T, DeadlineError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(work());
    });
    match rx.recv_timeout(deadline) {
        Ok(value) => Ok(value),
        Err(RecvTimeoutError::Timeout) => Err(DeadlineError::TimedOut(deadline)),
        Err(RecvTimeoutError::Disconnected) => Err(DeadlineError::Panicked),
    }
}

/// Waits for a transcription under `deadline` while `ticker` ticks. On any
/// failure (an error, a timeout, a panic) it stops the ticker, calls
/// `on_failure` exactly once, and returns `None`. On success it returns the
/// text and leaves the ticker running for post-processing.
pub fn await_transcription<F>(
    ticker: &mut Ticker,
    deadline: Duration,
    work: F,
    on_failure: impl FnOnce(&str),
) -> Option<String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    let err = match run_with_deadline(deadline, work) {
        Ok(Ok(text)) => return Some(text),
        Ok(Err(err)) => err,
        Err(err) => err.to_string(),
    };
    ticker.stop();
    on_failure(&err);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct FakeSink {
        ticks: Arc<AtomicUsize>,
        stopped: Arc<AtomicBool>,
    }

    impl TickSink for FakeSink {
        fn empty(&self) -> bool {
            true
        }
        fn play_tick(&self) {
            self.ticks.fetch_add(1, Ordering::SeqCst);
        }
        fn stop(&self) {
            self.stopped.store(true, Ordering::SeqCst);
        }
    }

    const POLL: Duration = Duration::from_millis(20);
    const LONG: Duration = Duration::from_secs(60);

    #[test]
    fn dropped_sender_ends_loop_within_one_poll() {
        let sink = FakeSink::default();
        let (tx, rx) = mpsc::channel::<()>();
        drop(tx);
        let t0 = Instant::now();
        let end = tick_loop(&sink, &rx, POLL, LONG);
        assert_eq!(end, TickEnd::Disconnected);
        assert!(t0.elapsed() <= POLL * 2, "took {:?}", t0.elapsed());
        assert!(sink.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn dropped_sender_mid_tick_ends_loop() {
        let sink = FakeSink::default();
        let s = sink.clone();
        let (tx, rx) = mpsc::channel::<()>();
        let h = thread::spawn(move || tick_loop(&s, &rx, POLL, LONG));
        thread::sleep(POLL * 3);
        let dropped_at = Instant::now();
        drop(tx);
        assert_eq!(h.join().unwrap(), TickEnd::Disconnected);
        assert!(dropped_at.elapsed() <= POLL * 2, "took {:?}", dropped_at.elapsed());
        assert!(sink.ticks.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn loop_past_max_duration_ends() {
        let sink = FakeSink::default();
        let (_tx, rx) = mpsc::channel::<()>(); // held: nobody ever says stop
        let t0 = Instant::now();
        let end = tick_loop(&sink, &rx, POLL, Duration::from_millis(150));
        assert_eq!(end, TickEnd::MaxDuration);
        assert!(t0.elapsed() < Duration::from_millis(150) + POLL * 2);
        assert!(sink.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn explicit_stop_ends_loop() {
        let sink = FakeSink::default();
        let (tx, rx) = mpsc::channel::<()>();
        tx.send(()).unwrap();
        assert_eq!(tick_loop(&sink, &rx, POLL, LONG), TickEnd::Stopped);
    }

    #[test]
    fn ticker_stops_when_dropped() {
        let sink = FakeSink::default();
        let s = sink.clone();
        let ticker = Ticker::start_with(move || s, POLL, LONG);
        thread::sleep(POLL * 2);
        drop(ticker); // joins the tick thread
        assert!(sink.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn ticker_stops_when_scope_panics() {
        let sink = FakeSink::default();
        let s = sink.clone();
        let result = std::panic::catch_unwind(move || {
            let _ticker = Ticker::start_with(move || s, POLL, LONG);
            thread::sleep(POLL * 2);
            panic!("transcription path blew up");
        });
        assert!(result.is_err());
        assert!(sink.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn ticker_stop_is_idempotent() {
        let mut ticker = Ticker::start_with(FakeSink::default, POLL, LONG);
        assert_eq!(ticker.stop(), Some(TickEnd::Stopped));
        assert_eq!(ticker.stop(), Some(TickEnd::Stopped));
    }

    #[test]
    fn timed_out_transcription_fails_once_and_returns() {
        let sink = FakeSink::default();
        let s = sink.clone();
        let mut ticker = Ticker::start_with(move || s, POLL, LONG);
        let failures = AtomicUsize::new(0);
        let t0 = Instant::now();
        let out = await_transcription(
            &mut ticker,
            Duration::from_millis(200),
            || {
                thread::sleep(Duration::from_secs(30)); // a hung API call
                Ok("never".to_string())
            },
            |err| {
                assert!(err.contains("timed out"), "{err}");
                failures.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert_eq!(out, None);
        assert_eq!(failures.load(Ordering::SeqCst), 1);
        assert!(t0.elapsed() < Duration::from_secs(2), "took {:?}", t0.elapsed());
        assert!(sink.stopped.load(Ordering::SeqCst), "ticking must stop before the failure sound");
    }

    #[test]
    fn panicking_transcription_fails_once_and_returns() {
        let mut ticker = Ticker::start_with(FakeSink::default, POLL, LONG);
        let failures = AtomicUsize::new(0);
        let out = await_transcription(
            &mut ticker,
            LONG,
            || panic!("ffmpeg not found"),
            |_| {
                failures.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert_eq!(out, None);
        assert_eq!(failures.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn successful_transcription_leaves_ticker_running() {
        let mut ticker = Ticker::start_with(FakeSink::default, POLL, LONG);
        let out = await_transcription(&mut ticker, LONG, || Ok("hi".into()), |_| panic!("no"));
        assert_eq!(out.as_deref(), Some("hi"));
        assert_eq!(ticker.stop(), Some(TickEnd::Stopped));
    }

    #[test]
    fn deadline_scales_with_audio_and_is_clamped() {
        assert_eq!(transcription_deadline(0.0), TRANSCRIPTION_DEADLINE_MIN);
        assert_eq!(transcription_deadline(30.0), Duration::from_secs(120));
        assert_eq!(transcription_deadline(3600.0), TRANSCRIPTION_DEADLINE_MAX);
        assert!(TICK_MAX > TRANSCRIPTION_DEADLINE_MAX + Duration::from_secs(28));
    }
}
