pub mod trans {

    use anyhow::{bail, Context};
    use async_openai::{config::OpenAIConfig, types::CreateTranscriptionRequestArgs, Client};
    use std::{
        error::Error,
        path::{Path, PathBuf},
        process::Command,
    };
    use tempfile::tempdir;

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
            .build()
            .context("Failed to build transcription request.")?;

        let response = client
            .audio()
            .transcribe(request)
            .await
            .context("Failed to get OpenAI API transcription response.")?;

        Ok(response.text)
    }
}
