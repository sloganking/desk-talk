pub mod trans {

    use anyhow::{anyhow, bail, Context};
    use async_openai::{config::OpenAIConfig, types::CreateTranscriptionRequestArgs, Client};
    use async_std::future;
    use directories::ProjectDirs;
    use mutter::ModelType;
    use rodio::{source::UniformSourceIterator, Decoder, Source};
    use std::io::Cursor;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
    use num_cpus;
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
        let _ = match Command::new("ffmpeg")
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

    fn load_or_download_context(
        model: &ModelType,
        use_gpu: bool,
    ) -> Result<WhisperContext, Box<dyn Error>> {
        use std::io::Read;

        let path = get_model_path(model)?;
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu);

        if path.exists() {
            let path_str = path.to_str().ok_or_else(|| anyhow!("Invalid model path"))?;
            Ok(WhisperContext::new_with_params(path_str, params).map_err(|e| anyhow!("{:?}", e))?)
        } else {
            let resp = ureq::get(&model.to_string())
                .call()
                .map_err(|e| anyhow!("Download error: {:?}", e))?;
            let mut bytes = Vec::new();
            resp.into_reader().read_to_end(&mut bytes)?;
            std::fs::write(&path, &bytes)?;
            Ok(WhisperContext::new_from_buffer_with_params(&bytes, params).map_err(|e| anyhow!("{:?}", e))?)
        }
    }

    fn decode_audio(bytes: Vec<u8>) -> Result<Vec<f32>, Box<dyn Error>> {
        let input = Cursor::new(bytes);
        let source = Decoder::new(input).unwrap();
        let output_sample_rate = 16000;
        let channels = 1;
        let resample = UniformSourceIterator::new(source, channels, output_sample_rate);
        let pass_filter = resample.low_pass(3000).high_pass(200).convert_samples();
        let samples: Vec<i16> = pass_filter.collect::<Vec<i16>>();
        let mut output: Vec<f32> = vec![0.0f32; samples.len()];
        whisper_rs::convert_integer_to_float_audio(&samples, &mut output)
            .map(|()| output)
            .map_err(|e| anyhow!("{:?}", e).into())
    }

    pub fn transcribe_local(
        input: &Path,
        model_type: ModelType,
        use_gpu: bool,
    ) -> Result<String, Box<dyn Error>> {
        let ctx = load_or_download_context(&model_type, use_gpu)?;
        let bytes = fs::read(input)?;
        let samples = decode_audio(bytes)?;

        let mut params = FullParams::new(SamplingStrategy::BeamSearch { beam_size: 5, patience: 1.0 });
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_token_timestamps(false);
        params.set_split_on_word(true);
        params.set_n_threads(num_cpus::get() as i32);

        let mut state = ctx.create_state().expect("failed to create state");
        state.full(params, &samples).expect("failed to transcribe");

        let num_segments = state.full_n_segments().expect("failed to get segments");
        let mut result = String::new();
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow!("{:?}", e))?;
            result.push_str(&segment);
            result.push(' ');
        }

        let mut res = result.replace('\n', " ");
        res = res.trim().to_string();
        Ok(res)
    }
}
