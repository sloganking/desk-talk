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
    }

    static RACING_STATS: RacingStats = RacingStats {
        total_requests: AtomicUsize::new(0),
        succeeded_requests: AtomicUsize::new(0),
        failed_requests: AtomicUsize::new(0),
        total_races: AtomicUsize::new(0),
        succeeded_races: AtomicUsize::new(0),
        failed_races: AtomicUsize::new(0),
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

        let (tx, rx) = std::sync::mpsc::channel();
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
                            RACING_STATS
                                .succeeded_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let elapsed = race_start.elapsed();
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
                            Ok(text)
                        }
                        Ok(Err(e)) => {
                            RACING_STATS
                                .failed_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let elapsed = race_start.elapsed();
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
                            Err(format!("{}", e))
                        }
                        Err(_) => {
                            RACING_STATS
                                .failed_requests
                                .fetch_add(1, Ordering::Relaxed);
                            let elapsed = race_start.elapsed();
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
                            Err("Timeout".to_string())
                        }
                    }
                });
                let _ = tx.send(result);
            });
        }

        drop(tx);

        let mut first_text: Option<String> = None;
        let mut last_err = String::from("Unknown error");
        while let Ok(result) = rx.recv() {
            match result {
                Ok(text) => {
                    if first_text.is_none() {
                        first_text = Some(text);
                    }
                }
                Err(e) => last_err = e,
            }
        }

        RACING_STATS.total_races.fetch_add(1, Ordering::Relaxed);

        if first_text.is_some() {
            RACING_STATS.succeeded_races.fetch_add(1, Ordering::Relaxed);
        } else {
            RACING_STATS.failed_races.fetch_add(1, Ordering::Relaxed);
        }

        let total_req = RACING_STATS.total_requests.load(Ordering::Relaxed);
        let ok_req = RACING_STATS.succeeded_requests.load(Ordering::Relaxed);
        let _fail_req = RACING_STATS.failed_requests.load(Ordering::Relaxed);
        let total_race = RACING_STATS.total_races.load(Ordering::Relaxed);
        let ok_race = RACING_STATS.succeeded_races.load(Ordering::Relaxed);
        let _fail_race = RACING_STATS.failed_races.load(Ordering::Relaxed);

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
            "API requests: {}/{} succeeded ({:.0}%) | Races: {}/{} succeeded ({:.0}%)",
            ok_req, total_req, req_pct, ok_race, total_race, race_pct,
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
}
