pub mod trans {

    use anyhow::{anyhow, bail, Context};
    use async_openai::{
        config::OpenAIConfig,
        types::{
            ChatCompletionRequestMessage, CreateChatCompletionRequestArgs,
            CreateTranscriptionRequestArgs, Role,
        },
        Client,
    };
    use async_std::future;
    use directories::ProjectDirs;
    use mutter::{Model, ModelType};
    use std::fs;
    use std::time::Duration;
    use std::{
        error::Error,
        path::{Path, PathBuf},
        process::Command,
    };

    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;
    use ureq;

    struct RacingStats {
        total_requests: AtomicUsize,
        succeeded_requests: AtomicUsize,
        failed_requests: AtomicUsize,
        total_races: AtomicUsize,
        succeeded_races: AtomicUsize,
        failed_races: AtomicUsize,
        /// Races that succeeded despite at least one lane failing
        failures_avoided: AtomicUsize,
        /// Sum of winning (fastest success) times in ms
        winning_time_ms_sum: AtomicUsize,
        /// Sum of all successful lane times in ms (for computing avg without racing)
        all_success_time_ms_sum: AtomicUsize,
        /// Count of successful lanes (denominator for all_success average)
        all_success_count: AtomicUsize,
    }

    static RACING_STATS: RacingStats = RacingStats {
        total_requests: AtomicUsize::new(0),
        succeeded_requests: AtomicUsize::new(0),
        failed_requests: AtomicUsize::new(0),
        total_races: AtomicUsize::new(0),
        succeeded_races: AtomicUsize::new(0),
        failed_races: AtomicUsize::new(0),
        failures_avoided: AtomicUsize::new(0),
        winning_time_ms_sum: AtomicUsize::new(0),
        all_success_time_ms_sum: AtomicUsize::new(0),
        all_success_count: AtomicUsize::new(0),
    };

    /// Moves audio to mp3.
    /// Ignores output's extension if it is passed one.
    /// Returns the new path.
    fn move_audio_to_mp3(input: &Path, output: &Path) -> Result<PathBuf, anyhow::Error> {
        let mut output = PathBuf::from(output);
        output.set_extension("mp3");

        // `ffmpeg -i input.mp4 -q:a 0 -map a output.mp3`
        let mut cmd = Command::new("ffmpeg");

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let _ = match cmd
            .args([
                "-i",
                input
                    .to_str()
                    .context("Failed to convert input path to string")?,
                "-q:a",
                "0",
                "-map",
                "a",
                output
                    .to_str()
                    .context("Failed to convert output path to string")?,
            ])
            .output()
        {
            Ok(x) => x,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    panic!("ffmpeg not found. Please install ffmpeg and add it to your PATH");
                } else {
                    bail!("ffmpeg failed to convert audio");
                }
            }
        };

        Ok(output)
    }

    pub async fn transcribe(
        client: &Client<OpenAIConfig>,
        input: &Path,
    ) -> Result<String, Box<dyn Error>> {
        let tmp_dir = tempdir().context("Failed to create temp dir.")?;
        let tmp_mp3_path = tmp_dir.path().join("tmp.mp3");

        // Make input file an mp3 if it is not
        // We do this to get around the api file size limit:
        // Error: ApiError(ApiError { message: "Maximum content size limit (26214400) exceeded (26228340 bytes read)", type: "server_error", param: None, code: None })
        let input_mp3 = if input.extension().unwrap_or_default() != "mp3" {
            // println!("{:?}", tmp_dir.path());
            move_audio_to_mp3(input, &tmp_mp3_path).context("Failed to convert audio to mp3.")?
        } else {
            // println!("{:?}", input);
            PathBuf::from(input)
        };

        let request = CreateTranscriptionRequestArgs::default()
            .file(input_mp3)
            .model("whisper-1")
            .prompt("And now, a transcription from random language(s) that concludes with perfect punctuation: ")
            .build()
            .context("Failed to build transcription request.")?;

        let response = client
            .audio()
            .transcribe(request)
            .await
            .context("Failed to get OpenAI API transcription response.")?;

        Ok(response.text)
    }

    pub async fn transcribe_with_retry(
        client: &Client<OpenAIConfig>,
        input: &Path,
        attempts: usize,
    ) -> Result<String, Box<dyn Error>> {
        let mut last_err: Option<Box<dyn Error>> = None;

        for attempt in 0..attempts {
            match future::timeout(Duration::from_secs(10), transcribe(client, input)).await {
                Ok(res) => match res {
                    Ok(text) => return Ok(text),
                    Err(e) => {
                        eprintln!(
                            "Transcription attempt {}/{} failed: {:?}",
                            attempt + 1,
                            attempts,
                            e
                        );
                        last_err = Some(e);
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Transcription attempt {}/{} timed out: {:?}",
                        attempt + 1,
                        attempts,
                        e
                    );
                    last_err = Some(anyhow!("Timeout").into());
                }
            }

            // No delay between retries so we don't block the user
        }

        Err(last_err.unwrap_or_else(|| Box::<dyn Error>::from(anyhow!("Unknown error"))))
    }

    /// Sends `parallel` transcription requests simultaneously and returns the
    /// first successful result. When `parallel` is 1 this falls back to the
    /// normal sequential retry logic. Each lane runs in its own thread with
    /// its own tokio runtime for true parallelism.
    pub async fn transcribe_racing(
        client: &Client<OpenAIConfig>,
        input: &Path,
        parallel: usize,
    ) -> Result<String, Box<dyn Error>> {
        let parallel = parallel.clamp(1, 5);

        if parallel <= 1 {
            return transcribe_with_retry(client, input, 3).await;
        }

        eprintln!("Racing {} parallel transcription requests", parallel);

        let mp3_input = if input.extension().unwrap_or_default() != "mp3" {
            let tmp_dir = tempdir().context("Failed to create temp dir.")?;
            let tmp_mp3_path = tmp_dir.path().join("racing_tmp.mp3");
            let mp3 = move_audio_to_mp3(input, &tmp_mp3_path)
                .context("Failed to convert audio to mp3.")?;
            std::mem::forget(tmp_dir);
            mp3
        } else {
            PathBuf::from(input)
        };

        // Channel sends (result, elapsed_ms) per lane
        let (tx, rx) = std::sync::mpsc::channel::<(Result<String, String>, u128)>();
        let race_start = std::time::Instant::now();
        let first_success_time: std::sync::Arc<std::sync::Mutex<Option<std::time::Instant>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));

        RACING_STATS
            .total_requests
            .fetch_add(parallel, Ordering::Relaxed);

        for i in 0..parallel {
            let client = client.clone();
            let input = mp3_input.clone();
            let tx = tx.clone();
            let race_start = race_start;
            let first_success_time = first_success_time.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let result = rt.block_on(async {
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        transcribe(&client, &input),
                    )
                    .await
                    {
                        Ok(Ok(text)) => {
                            let elapsed = race_start.elapsed();
                            let elapsed_ms = elapsed.as_millis();
                            RACING_STATS
                                .succeeded_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let mut first = first_success_time.lock().unwrap();
                            if first.is_none() {
                                *first = Some(std::time::Instant::now());
                                eprintln!(
                                    "Parallel lane {} succeeded ({:.2}s)",
                                    i + 1,
                                    elapsed.as_secs_f64()
                                );
                            } else {
                                let wasted = first.unwrap().elapsed();
                                eprintln!(
                                    "Parallel lane {} succeeded ({:.2}s, +{:.2}s wasted)",
                                    i + 1,
                                    elapsed.as_secs_f64(),
                                    wasted.as_secs_f64()
                                );
                            }
                            drop(first);
                            (Ok(text), elapsed_ms)
                        }
                        Ok(Err(e)) => {
                            let elapsed = race_start.elapsed();
                            let elapsed_ms = elapsed.as_millis();
                            RACING_STATS
                                .failed_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let extra = first_success_time
                                .lock()
                                .unwrap()
                                .map(|t| t.elapsed().as_secs_f64());
                            if let Some(wasted) = extra {
                                eprintln!(
                                    "Parallel lane {} failed ({:.2}s, +{:.2}s wasted): {}",
                                    i + 1,
                                    elapsed.as_secs_f64(),
                                    wasted,
                                    e
                                );
                            } else {
                                eprintln!(
                                    "Parallel lane {} failed ({:.2}s): {}",
                                    i + 1,
                                    elapsed.as_secs_f64(),
                                    e
                                );
                            }
                            (Err(format!("{}", e)), elapsed_ms)
                        }
                        Err(_) => {
                            let elapsed = race_start.elapsed();
                            let elapsed_ms = elapsed.as_millis();
                            RACING_STATS
                                .failed_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let extra = first_success_time
                                .lock()
                                .unwrap()
                                .map(|t| t.elapsed().as_secs_f64());
                            if let Some(wasted) = extra {
                                eprintln!(
                                    "Parallel lane {} timed out ({:.2}s, +{:.2}s wasted)",
                                    i + 1,
                                    elapsed.as_secs_f64(),
                                    wasted
                                );
                            } else {
                                eprintln!(
                                    "Parallel lane {} timed out ({:.2}s)",
                                    i + 1,
                                    elapsed.as_secs_f64()
                                );
                            }
                            (Err("Timeout".to_string()), elapsed_ms)
                        }
                    }
                });
                let _ = tx.send(result);
            });
        }

        drop(tx);

        let mut first_text: Option<String> = None;
        let mut last_err = String::from("Unknown error");
        let mut success_times_ms: Vec<u128> = Vec::new();
        let mut had_failure = false;

        while let Ok((result, elapsed_ms)) = rx.recv() {
            match result {
                Ok(text) => {
                    success_times_ms.push(elapsed_ms);
                    if first_text.is_none() {
                        first_text = Some(text);
                    }
                }
                Err(e) => {
                    had_failure = true;
                    last_err = e;
                }
            }
        }

        RACING_STATS.total_races.fetch_add(1, Ordering::Relaxed);

        let race_succeeded = first_text.is_some();
        if race_succeeded {
            RACING_STATS.succeeded_races.fetch_add(1, Ordering::Relaxed);

            if had_failure {
                RACING_STATS
                    .failures_avoided
                    .fetch_add(1, Ordering::Relaxed);
            }

            if let Some(&fastest) = success_times_ms.iter().min() {
                RACING_STATS
                    .winning_time_ms_sum
                    .fetch_add(fastest as usize, Ordering::Relaxed);
            }
            let total_success_ms: u128 = success_times_ms.iter().sum();
            RACING_STATS
                .all_success_time_ms_sum
                .fetch_add(total_success_ms as usize, Ordering::Relaxed);
            RACING_STATS
                .all_success_count
                .fetch_add(success_times_ms.len(), Ordering::Relaxed);
        } else {
            RACING_STATS.failed_races.fetch_add(1, Ordering::Relaxed);
        }

        let total_req = RACING_STATS.total_requests.load(Ordering::Relaxed);
        let ok_req = RACING_STATS.succeeded_requests.load(Ordering::Relaxed);
        let total_race = RACING_STATS.total_races.load(Ordering::Relaxed);
        let ok_race = RACING_STATS.succeeded_races.load(Ordering::Relaxed);
        let avoided = RACING_STATS.failures_avoided.load(Ordering::Relaxed);

        let req_pct = if total_req > 0 {
            (ok_req as f64 / total_req as f64) * 100.0
        } else {
            0.0
        };
        let race_pct = if total_race > 0 {
            (ok_race as f64 / total_race as f64) * 100.0
        } else {
            0.0
        };

        eprintln!(
            "API requests: {}/{} succeeded ({:.0}%) | Races: {}/{} succeeded ({:.0}%) | Failures avoided: {}",
            ok_req, total_req, req_pct, ok_race, total_race, race_pct, avoided,
        );

        match first_text {
            Some(text) => Ok(text),
            None => {
                Err(anyhow!("All {} parallel attempts failed: {}", parallel, last_err).into())
            }
        }
    }

    fn get_model_path(model: &ModelType) -> Result<std::path::PathBuf, Box<dyn Error>> {
        let dirs = ProjectDirs::from("", "", "desk-talk")
            .ok_or_else(|| anyhow!("Unable to determine project directory"))?;
        let cache_dir = dirs.cache_dir();
        std::fs::create_dir_all(cache_dir)?;
        let url = model.to_string();
        let filename = url
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("Bad model url"))?;
        Ok(cache_dir.join(filename))
    }

    fn load_or_download_model(model: &ModelType) -> Result<Model, Box<dyn Error>> {
        use std::io::Read;

        let path = get_model_path(model)?;
        if path.exists() {
            let path_str = path.to_str().ok_or_else(|| anyhow!("Invalid model path"))?;
            Ok(Model::new(path_str).map_err(|e| anyhow!("{:?}", e))?)
        } else {
            let resp = ureq::get(&model.to_string())
                .call()
                .map_err(|e| anyhow!("Download error: {:?}", e))?;
            let mut bytes = Vec::new();
            resp.into_reader().read_to_end(&mut bytes)?;
            std::fs::write(&path, &bytes)?;
            let path_str = path.to_str().ok_or_else(|| anyhow!("Invalid model path"))?;
            Ok(Model::new(path_str).map_err(|e| anyhow!("{:?}", e))?)
        }
    }

    pub fn transcribe_local(input: &Path, model_type: ModelType) -> Result<String, Box<dyn Error>> {
        let model = load_or_download_model(&model_type)?;
        let bytes = fs::read(input)?;
        let res = model
            .transcribe_audio(bytes, false, false, None)
            .map_err(|e| anyhow!("{:?}", e))?;

        let mut res = res.as_text();
        res = res.replace("\n", " "); // Remove double spaces
        res = res.trim().to_string();
        Ok(res)
    }

    /// Uses GPT-4o-mini to add punctuation to text that is missing it.
    async fn fix_punctuation_inner(
        client: &Client<OpenAIConfig>,
        text: &str,
    ) -> Result<String, Box<dyn Error>> {
        let system_message = ChatCompletionRequestMessage {
            role: Role::System,
            content: Some(
                "You are a punctuation restoration assistant. Add punctuation (periods, commas, question marks, exclamation points) and fix capitalization where needed to make the text readable. Return ONLY the corrected text without any additional comments, explanations, or conversational responses. Preserve the original wording and do not add or remove content."
                    .to_string(),
            ),
            name: None,
            function_call: None,
        };

        let user_message = ChatCompletionRequestMessage {
            role: Role::User,
            content: Some(text.to_string()),
            name: None,
            function_call: None,
        };

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(vec![system_message, user_message])
            .temperature(0.2)
            .build()
            .context("Failed to build punctuation request")?;

        let response = client
            .chat()
            .create(request)
            .await
            .context("Failed to get punctuation response")?;

        response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| Box::<dyn Error>::from(anyhow!("No response from OpenAI")))
    }

    /// Uses GPT-4o-mini to add punctuation to text that is missing it.
    /// Has a 10-second timeout to prevent hanging.
    pub async fn fix_punctuation_with_openai(
        client: &Client<OpenAIConfig>,
        text: &str,
    ) -> Result<String, Box<dyn Error>> {
        match future::timeout(Duration::from_secs(10), fix_punctuation_inner(client, text)).await {
            Ok(result) => result,
            Err(_) => {
                eprintln!("Punctuation fix timed out after 10 seconds");
                Err(anyhow!("Punctuation fix timed out").into())
            }
        }
    }

    pub struct RacingStatsSnapshot {
        pub total_requests: usize,
        pub succeeded_requests: usize,
        pub failed_requests: usize,
        pub total_races: usize,
        pub succeeded_races: usize,
        pub failed_races: usize,
        pub failures_avoided: usize,
        /// Average time of the winning (fastest) lane in ms
        pub avg_winning_time_ms: f64,
        /// Average time across all successful lanes in ms
        pub avg_all_success_time_ms: f64,
    }

    pub fn get_racing_stats() -> RacingStatsSnapshot {
        let total_requests = RACING_STATS.total_requests.load(Ordering::Relaxed);
        let succeeded_requests = RACING_STATS.succeeded_requests.load(Ordering::Relaxed);
        let failed_requests = RACING_STATS.failed_requests.load(Ordering::Relaxed);
        let total_races = RACING_STATS.total_races.load(Ordering::Relaxed);
        let succeeded_races = RACING_STATS.succeeded_races.load(Ordering::Relaxed);
        let failed_races = RACING_STATS.failed_races.load(Ordering::Relaxed);
        let failures_avoided = RACING_STATS.failures_avoided.load(Ordering::Relaxed);
        let winning_sum = RACING_STATS.winning_time_ms_sum.load(Ordering::Relaxed);
        let all_sum = RACING_STATS.all_success_time_ms_sum.load(Ordering::Relaxed);
        let all_count = RACING_STATS.all_success_count.load(Ordering::Relaxed);

        let avg_winning_time_ms = if succeeded_races > 0 {
            winning_sum as f64 / succeeded_races as f64
        } else {
            0.0
        };
        let avg_all_success_time_ms = if all_count > 0 {
            all_sum as f64 / all_count as f64
        } else {
            0.0
        };

        RacingStatsSnapshot {
            total_requests,
            succeeded_requests,
            failed_requests,
            total_races,
            succeeded_races,
            failed_races,
            failures_avoided,
            avg_winning_time_ms,
            avg_all_success_time_ms,
        }
    }
}
