use std::future::Future;
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::base::AppError;

use super::config::{AudioConfig, AUDIO_CONFIG};

#[derive(PartialEq)]
enum State {
    Idle,
    Recording,
}

pub struct AudioRecorder {
    state: Arc<Mutex<State>>,
    arecord_thread: Arc<Mutex<Option<Child>>>,
    read_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
}

static INSTANCE: LazyLock<AudioRecorder> = LazyLock::new(AudioRecorder::new);

impl AudioRecorder {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Idle)),
            arecord_thread: Arc::new(Mutex::new(None)),
            read_thread: Arc::new(Mutex::new(None)),
        }
    }

    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    pub async fn stop_recording(&self) -> Result<(), AppError> {
        let mut state = self.state.lock().await;
        if *state == State::Idle {
            return Ok(());
        }

        if let Some(read_thread) = self.read_thread.lock().await.take() {
            read_thread.abort();
        }

        if let Some(mut arecord_thread) = self.arecord_thread.lock().await.take() {
            let _ = arecord_thread.kill().await;
        }

        *state = State::Idle;
        Ok(())
    }

    pub async fn start_recording<F, Fut>(
        &self,
        on_stream: F,
        config: Option<AudioConfig>,
    ) -> Result<(), AppError>
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let mut state = self.state.lock().await;
        if *state == State::Recording {
            return Ok(());
        }

        let config = config.unwrap_or_else(|| (*AUDIO_CONFIG).clone());

        let mut arecord_thread = Command::new("arecord")
            .args([
                "--quiet",
                "-t",
                "raw",
                "-D",
                &config.pcm,
                "-f",
                &format!("S{}_LE", config.bits_per_sample),
                "-r",
                &config.sample_rate.to_string(),
                "-c",
                &config.channels.to_string(),
                "--buffer-size",
                &config.buffer_size.to_string(),
                "--period-size",
                &config.period_size.to_string(),
            ])
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdout = arecord_thread.stdout.take().unwrap();
        let read_thread = tokio::spawn(async move {
            let size = (config.bits_per_sample / 8) as usize;
            let target_size = config.buffer_size as usize * size;

            let mut accumulated_data = Vec::new();
            let mut buffer = vec![0u8; config.period_size as usize * size];

            loop {
                match stdout.read(&mut buffer).await {
                    Ok(size) if size > 0 => {
                        accumulated_data.extend_from_slice(&buffer[..size]);
                        if accumulated_data.len() >= target_size {
                            let data_to_send =
                                accumulated_data.drain(..target_size).collect::<Vec<u8>>();
                            let _ = on_stream(data_to_send).await;
                        }
                    }
                    _ => break,
                }
            }

            let _ = AudioRecorder::instance().stop_recording().await;
        });

        self.arecord_thread.lock().await.replace(arecord_thread);
        self.read_thread.lock().await.replace(read_thread);

        *state = State::Recording;
        Ok(())
    }
}
