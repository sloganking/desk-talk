use crate::config::{AppConfig, KeygenConfig};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub keygen: Arc<RwLock<Option<KeygenConfig>>>,
    pub is_running: Arc<RwLock<bool>>,
    pub statistics: Arc<RwLock<Statistics>>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Statistics {
    pub total_words: usize,
    pub total_recording_time_secs: f64,
    pub average_wpm: f64,
    pub session_count: usize,
}

impl AppState {
    pub fn new(config: AppConfig, keygen: Option<KeygenConfig>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            keygen: Arc::new(RwLock::new(keygen)),
            is_running: Arc::new(RwLock::new(false)),
            statistics: Arc::new(RwLock::new(Statistics::default())),
        }
    }

    pub fn start_transcription(&self) {
        *self.is_running.write() = true;
    }

    pub fn stop_transcription(&self) {
        *self.is_running.write() = false;
    }

    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }

    pub fn update_statistics(&self, words: usize, duration_secs: f64, wpm: f64) {
        let mut stats = self.statistics.write();
        stats.total_words += words;
        stats.total_recording_time_secs += duration_secs;
        stats.session_count += 1;

        // Update average WPM using latest sample
        let total_sessions = stats.session_count as f64;
        stats.average_wpm = ((stats.average_wpm * (total_sessions - 1.0)) + wpm) / total_sessions;
    }

    pub fn get_statistics(&self) -> Statistics {
        self.statistics.read().clone()
    }

    pub fn keygen_config(&self) -> Option<KeygenConfig> {
        self.keygen.read().clone()
    }
}
