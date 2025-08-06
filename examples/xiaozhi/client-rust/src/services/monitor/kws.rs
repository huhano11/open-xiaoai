use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::base::AppError;

use super::file::{FileMonitor, FileMonitorEvent};

#[derive(Debug, Serialize, Deserialize)]
pub enum KwsMonitorEvent {
    Started,
    Keyword(String),
}

pub struct KwsMonitor;

pub static KWS_FILE_PATH: &str = "/tmp/open-xiaoai/kws.log";

static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

impl KwsMonitor {
    pub async fn start<F, Fut>(on_update: F)
    where
        F: Fn(KwsMonitorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let on_update = Arc::new(on_update);
        FileMonitor::instance()
            .start(KWS_FILE_PATH, move |event| {
                let on_update = Arc::clone(&on_update);
                async move {
                    if let FileMonitorEvent::NewLine(content) = event {
                        let data = content.split('@').collect::<Vec<&str>>();
                        let timestamp = data[0].parse::<u64>().unwrap();
                        let keyword = data[1].to_string();
                        let last_timestamp = LAST_TIMESTAMP.load(Ordering::Relaxed);
                        if timestamp != last_timestamp {
                            LAST_TIMESTAMP.store(timestamp, Ordering::Relaxed);
                            let kws_event = if keyword == "__STARTED__" {
                                KwsMonitorEvent::Started
                            } else {
                                KwsMonitorEvent::Keyword(keyword)
                            };
                            let _ = on_update(kws_event).await;
                        }
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub async fn stop() {
        LAST_TIMESTAMP.store(0, Ordering::Relaxed);
        FileMonitor::instance().stop(KWS_FILE_PATH).await;
    }
}
