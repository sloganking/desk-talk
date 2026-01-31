use crate::config::AppConfig;
use anyhow::Context;
use directories::ProjectDirs;
use flume::Sender;
use parking_lot::RwLock;
use rdev::Event;
use std::fs;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub is_running: Arc<RwLock<bool>>,
    pub statistics: Arc<RwLock<Statistics>>,
    pub lifetime_statistics: Arc<RwLock<LifetimeStatistics>>,
    pub event_sender: Arc<RwLock<Option<Sender<Event>>>>,
}

/// Session statistics (reset each time app starts)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Statistics {
    pub total_words: usize,
    pub total_recording_time_secs: f64,
    pub average_wpm: f64,
    pub session_count: usize,
}

/// Statistics for a single day
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DailyStats {
    pub words: usize,
    pub recording_time_secs: f64,
    pub transcription_count: usize,
}

/// Lifetime statistics (persisted to disk)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LifetimeStatistics {
    pub total_words: usize,
    pub total_recording_time_secs: f64,
    pub session_count: usize,
    // Store sum of all WPMs to calculate accurate average
    pub wpm_sum: f64,
    // Unix timestamp of first transcription (for calculating daily averages)
    #[serde(default)]
    pub first_recorded_at: Option<i64>,
    // Daily breakdown (date string "YYYY-MM-DD" -> stats)
    #[serde(default)]
    pub daily_stats: std::collections::HashMap<String, DailyStats>,
}

impl LifetimeStatistics {
    fn get_stats_path() -> Option<std::path::PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "desk-talk", "desk-talk")?;
        let data_dir = proj_dirs.data_dir();
        let _ = fs::create_dir_all(data_dir);
        Some(data_dir.join("lifetime_stats.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::get_stats_path() {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Ok(stats) = serde_json::from_str(&contents) {
                        println!("Loaded lifetime statistics from {:?}", path);
                        return stats;
                    }
                }
            }
        }
        println!("No existing lifetime statistics found, starting fresh");
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_stats_path().context("Failed to determine stats path")?;
        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize lifetime stats")?;
        fs::write(&path, contents).context("Failed to write lifetime stats")?;
        Ok(())
    }

    pub fn average_wpm(&self) -> f64 {
        if self.session_count > 0 {
            self.wpm_sum / (self.session_count as f64)
        } else {
            0.0
        }
    }

    /// Returns the number of days since tracking started (minimum 1)
    pub fn days_since_start(&self) -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        if let Some(first_at) = self.first_recorded_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(first_at);
            
            let elapsed_secs = (now - first_at).max(0) as f64;
            let days = elapsed_secs / 86400.0; // seconds per day
            days.max(1.0) // minimum 1 day to avoid division issues
        } else {
            1.0 // No data yet, assume 1 day
        }
    }
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        // Load lifetime statistics from disk
        let lifetime_stats = LifetimeStatistics::load();

        Self {
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(RwLock::new(false)),
            statistics: Arc::new(RwLock::new(Statistics::default())),
            lifetime_statistics: Arc::new(RwLock::new(lifetime_stats)),
            event_sender: Arc::new(RwLock::new(None)),
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

    pub fn set_event_sender(&self, sender: Sender<Event>) {
        *self.event_sender.write() = Some(sender);
    }

    pub fn clear_event_sender(&self) {
        self.event_sender.write().take();
    }

    pub fn event_sender(&self) -> Option<Sender<Event>> {
        self.event_sender.read().clone()
    }

    pub fn update_statistics(&self, words: usize, duration_secs: f64, wpm: f64) {
        // Update session statistics
        {
            let mut stats = self.statistics.write();
            stats.total_words += words;
            stats.total_recording_time_secs += duration_secs;
            stats.session_count += 1;

            // Update average WPM using latest sample
            let total_sessions = stats.session_count as f64;
            stats.average_wpm =
                ((stats.average_wpm * (total_sessions - 1.0)) + wpm) / total_sessions;
        }

        // Update lifetime statistics and save to disk
        {
            use chrono::Local;
            
            let mut lifetime = self.lifetime_statistics.write();
            
            // Set first_recorded_at on first transcription
            if lifetime.first_recorded_at.is_none() {
                use std::time::{SystemTime, UNIX_EPOCH};
                lifetime.first_recorded_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .ok();
                println!("First transcription recorded at: {:?}", lifetime.first_recorded_at);
            }
            
            lifetime.total_words += words;
            lifetime.total_recording_time_secs += duration_secs;
            lifetime.session_count += 1;
            lifetime.wpm_sum += wpm;
            
            // Update daily stats
            let today = Local::now().format("%Y-%m-%d").to_string();
            let daily = lifetime.daily_stats.entry(today).or_default();
            daily.words += words;
            daily.recording_time_secs += duration_secs;
            daily.transcription_count += 1;
            
            // Save to disk (fire and forget - don't block on errors)
            if let Err(e) = lifetime.save() {
                eprintln!("Warning: Failed to save lifetime statistics: {}", e);
            }
        }
    }

    pub fn get_statistics(&self) -> Statistics {
        self.statistics.read().clone()
    }

    pub fn get_lifetime_statistics(&self) -> LifetimeStatistics {
        self.lifetime_statistics.read().clone()
    }
}
