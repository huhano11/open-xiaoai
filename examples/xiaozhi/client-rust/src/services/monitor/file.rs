use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::sync::LazyLock;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::time::{sleep, Duration};

use crate::base::AppError;
use crate::utils::task::TaskManager;

#[derive(Debug, Serialize, Deserialize)]
pub enum FileMonitorEvent {
    NewFile,
    NewLine(String),
}

pub struct FileMonitor;

static INSTANCE: LazyLock<FileMonitor> = LazyLock::new(FileMonitor::new);

impl FileMonitor {
    fn new() -> Self {
        Self {}
    }

    pub fn instance() -> &'static Self {
        &INSTANCE
    }

    pub async fn start<F, Fut>(&self, file_path: &str, on_update: F)
    where
        F: Fn(FileMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let file_path_clone = file_path.to_string();

        let monitor = tokio::spawn(async move {
            let _ = FileMonitor::start_monitor(file_path_clone.as_str(), on_update).await;
        });

        TaskManager::instance()
            .add(&format!("FileMonitor-{}", file_path), monitor)
            .await;
    }

    pub async fn stop(&self, file_path: &str) {
        TaskManager::instance()
            .dispose(&format!("FileMonitor-{}", file_path))
            .await;
    }

    async fn start_monitor<F, Fut>(file_path: &str, on_update: F) -> Result<(), AppError>
    where
        F: Fn(FileMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        while !Path::new(file_path).exists() {
            sleep(Duration::from_millis(10)).await;
        }

        let file = OpenOptions::new().read(true).open(file_path).await?;
        let mut reader = BufReader::new(file);

        let metadata = reader.get_ref().metadata().await.unwrap();
        let mut position = metadata.len();

        loop {
            let metadata = reader.get_ref().metadata().await.unwrap();

            let current_size = metadata.len();
            if current_size < position {
                position = 0;
                let _ = on_update(FileMonitorEvent::NewFile).await;
            }

            if reader.stream_position().await? != position {
                reader.seek(SeekFrom::Start(position)).await?;
            }

            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }

                let trimmed_line = line.trim();
                if !trimmed_line.is_empty() {
                    let _ = on_update(FileMonitorEvent::NewLine(trimmed_line.to_string())).await;
                }

                position = reader.stream_position().await?;
                line.clear();
            }

            sleep(Duration::from_millis(10)).await;
        }
    }
}
