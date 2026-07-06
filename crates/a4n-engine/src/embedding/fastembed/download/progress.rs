use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc::UnboundedSender;

use crate::embedding::provider::DownloadProgress;

pub(super) struct AggregatedDownloadProgress {
    tx: Option<UnboundedSender<DownloadProgress>>,
    completed: Arc<AtomicU64>,
    total_bytes: u64,
    file_name: String,
    last_emitted_bytes: u64,
}

impl AggregatedDownloadProgress {
    pub(super) fn new(
        tx: Option<UnboundedSender<DownloadProgress>>,
        completed: Arc<AtomicU64>,
        total_bytes: u64,
        file_name: String,
    ) -> Self {
        Self {
            tx,
            completed,
            total_bytes,
            file_name,
            last_emitted_bytes: 0,
        }
    }

    pub(super) fn update(&mut self, size: u64) {
        self.completed.fetch_add(size, Ordering::Relaxed);
        self.emit(false);
    }

    pub(super) fn emit(&mut self, force: bool) {
        let downloaded = self.completed.load(Ordering::Relaxed);
        if !force && downloaded.saturating_sub(self.last_emitted_bytes) < 256 * 1024 {
            return;
        }
        self.last_emitted_bytes = downloaded;
        send_download_progress(
            self.tx.as_ref(),
            downloaded,
            self.total_bytes,
            &format!("正在下载 {}", self.file_name),
        );
    }
}

pub(super) fn send_download_progress(
    tx: Option<&UnboundedSender<DownloadProgress>>,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: &str,
) {
    let Some(tx) = tx else {
        return;
    };
    let percent = if total_bytes > 0 {
        (downloaded_bytes.min(total_bytes) as f32 / total_bytes as f32) * 100.0
    } else {
        0.0
    };
    let _ = tx.send(DownloadProgress {
        downloaded_bytes: downloaded_bytes.min(total_bytes),
        total_bytes,
        percent,
        message: message.to_string(),
    });
}

pub(super) fn format_bytes(value: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = value as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
