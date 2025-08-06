use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::time::{sleep, Duration};

use crate::base::AppError;
use crate::utils::shell::run_shell;
use crate::utils::task::TaskManager;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PlayingMonitorEvent {
    Playing,
    Paused,
    Idle,
}

pub struct PlayingMonitor;

impl PlayingMonitor {
    pub async fn start<F, Fut>(on_update: F)
    where
        F: Fn(PlayingMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let monitor = tokio::spawn(async move {
            let _ = PlayingMonitor::start_monitor(on_update).await;
        });

        TaskManager::instance().add("PlayingMonitor", monitor).await;
    }

    pub async fn stop() {
        TaskManager::instance().dispose("PlayingMonitor").await;
    }

    async fn start_monitor<F, Fut>(on_update: F) -> Result<(), AppError>
    where
        F: Fn(PlayingMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let mut last_status = PlayingMonitorEvent::Idle;
        loop {
            let res = run_shell("mphelper mute_stat").await?;
            let status = if res.stdout.contains("1") {
                PlayingMonitorEvent::Playing
            } else if res.stdout.contains("2") {
                PlayingMonitorEvent::Paused
            } else {
                PlayingMonitorEvent::Idle
            };

            if last_status != status {
                last_status = status.clone();
                let _ = on_update(status).await;
            }

            sleep(Duration::from_millis(10)).await;
        }
    }
}
