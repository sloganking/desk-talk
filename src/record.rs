pub mod rec {

    use anyhow::{bail, Context};
    // use clap::Parser;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, Sample};
    use hound::WavWriter;
    // use no_panic::no_panic;
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    // #[derive(Parser, Debug)]
    // #[command(version, about = "CPAL record_wav example", long_about = None)]
    // struct Opt {
    //     /// The audio device to use
    //     #[arg(short, long, default_value_t = String::from("default"))]
    //     device: String,

    //     /// Use the JACK host
    //     #[cfg(all(
    //         any(
    //             target_os = "linux",
    //             target_os = "dragonfly",
    //             target_os = "freebsd",
    //             target_os = "netbsd"
    //         ),
    //         feature = "jack"
    //     ))]
    //     #[arg(short, long)]
    //     #[allow(dead_code)]
    //     jack: bool,
    // }

    pub struct Recorder {
        utils: Option<(Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>, cpal::Stream)>,
    }

    impl Recorder {
        // #[no_panic]
        pub fn new() -> Self {
            Recorder { utils: None }
        }

        // #[no_panic]
        // pub fn start_recording_test(&mut self) -> Result<(), anyhow::Error> {
        //     if self.utils.is_some() {
        //         bail!("Attempted to start recording when already recording!")
        //     }

        //     Ok(())
        // }

        // #[no_panic]
        pub fn start_recording(
            &mut self,
            save_location: &Path,
            device: Option<&str>,
        ) -> Result<(), anyhow::Error> {
            if self.utils.is_some() {
                bail!("Attempted to start recording when already recording!")
            }

            let device = device.unwrap_or("default");

            // ========================
            // let opt = Opt::parse();

            // // Conditionally compile with jack if the feature is specified.
            // #[cfg(all(
            //     any(
            //         target_os = "linux",
            //         target_os = "dragonfly",
            //         target_os = "freebsd",
            //         target_os = "netbsd"
            //     ),
            //     feature = "jack"
            // ))]
            // // Manually check for flags. Can be passed through cargo with -- e.g.
            // // cargo run --release --example beep --features jack -- --jack
            // let host = if opt.jack {
            //     cpal::host_from_id(cpal::available_hosts()
            //         .into_iter()
            //         .find(|id| *id == cpal::HostId::Jack)
            //         .expect(
            //             "make sure --features jack is specified. only works on OSes where jack is available",
            //         )).expect("jack host unavailable")
            // } else {
            //     cpal::default_host()
            // };

            // #[cfg(any(
            //     not(any(
            //         target_os = "linux",
            //         target_os = "dragonfly",
            //         target_os = "freebsd",
            //         target_os = "netbsd"
            //     )),
            //     not(feature = "jack")
            // ))]
            let host = cpal::default_host();

            // Set up the input device and stream with the default input config.
            let device = match if device == "default" {
                host.default_input_device()
            } else {
                host.input_devices()
                    .context("Failed to get list of input devices")?
                    .find(|x| x.name().map(|y| y == device).unwrap_or(false))
            } {
                Some(x) => x,
                None => {
                    bail!(format!("Failed to find input device '{}'", device))
                }
            };

            match device.name() {
                Ok(name) => println!("Input device: {}", name),
                Err(e) => println!("Failed to get device name: {}", e),
            }

            let config = device
                .default_input_config()
                .context("Failed to get default input config")?;

            println!("Default input config: {:?}", config);

            // The WAV file we're recording to.
            let spec = wav_spec_from_config(&config);
            let writer = hound::WavWriter::create(save_location, spec)
                .context("Failed to create WAV writer")?;
            let writer = Arc::new(Mutex::new(Some(writer)));

            // A flag to indicate that recording is in progress.
            // println!("Begin recording...");

            // Run the input stream on a separate thread.
            let writer_2 = writer.clone();

            let err_fn = move |err| {
                eprintln!("an error occurred on stream: {}", err);
            };

            let stream = match config.sample_format() {
                cpal::SampleFormat::I8 => device
                    .build_input_stream(
                        &config.into(),
                        move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
                        err_fn,
                        None,
                    )
                    .context("Failed to build_input_stream (i8)")?,
                cpal::SampleFormat::I16 => device
                    .build_input_stream(
                        &config.into(),
                        move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
                        err_fn,
                        None,
                    )
                    .context("Failed to build_input_stream (i16)")?,
                cpal::SampleFormat::I32 => device
                    .build_input_stream(
                        &config.into(),
                        move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
                        err_fn,
                        None,
                    )
                    .context("Failed to build_input_stream (i32)")?,
                cpal::SampleFormat::F32 => device
                    .build_input_stream(
                        &config.into(),
                        move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
                        err_fn,
                        None,
                    )
                    .context("Failed to build_input_stream (f32)")?,
                sample_format => {
                    bail!(format!("Unsupported sample format '{sample_format}'"))
                }
            };
            // ========================

            stream.play().context("Failed to play stream")?;
            self.utils = Some((writer, stream));
            Ok(())
        }
        pub fn stop_recording(&mut self) -> Result<(), anyhow::Error> {
            match self.utils.take() {
                Some((writer, stream)) => {
                    stream.pause().context("Failed to pause stream")?;
                    // writer.lock().unwrap().take().unwrap().finalize().unwrap();
                    // Here's your modified match statement
                    match writer.lock() {
                        Ok(mut guard) => {
                            if let Some(writer) = guard.take() {
                                writer.finalize().context("Error finalizing WavWriter")?;
                            } else {
                                // Handle the case where `take()` returns `None`
                                bail!("WavWriter was already taken");
                            }
                        }
                        Err(e) => {
                            // Handle the error case from `lock()`
                            bail!(format!("Mutex is poisoned: {:?}", e));
                        }
                    }

                    Ok(())
                }
                None => Err(anyhow::Error::msg(
                    "Attempted to stop recording when not recording!",
                )),
            }
        }
    }

    fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
        if format.is_float() {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        }
    }

    fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
        hound::WavSpec {
            channels: config.channels() as _,
            sample_rate: config.sample_rate().0 as _,
            bits_per_sample: (config.sample_format().sample_size() * 8) as _,
            sample_format: sample_format(config.sample_format()),
        }
    }

    type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

    fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
    where
        T: Sample,
        U: Sample + hound::Sample + FromSample<T>,
    {
        if let Ok(mut guard) = writer.try_lock() {
            if let Some(writer) = guard.as_mut() {
                for &sample in input.iter() {
                    let sample: U = U::from_sample(sample);
                    writer.write_sample(sample).ok();
                }
            }
        }
    }
}
