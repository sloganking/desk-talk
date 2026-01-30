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

    use tempfile::tempdir;
    use ureq;

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
