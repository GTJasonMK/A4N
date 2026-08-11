use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc::UnboundedSender;

use a4n_embedding::provider::DownloadProgress;

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

    pub(super) fn update(&mut self, size: u64) -> Result<(), String> {
        let total_bytes = self.total_bytes;
        self.completed
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |completed| {
                completed
                    .checked_add(size)
                    .filter(|next| *next <= total_bytes)
            })
            .map_err(|completed| {
                format!("fastembed 下载进度超过声明总量：{completed} + {size} > {total_bytes}")
            })?;
        self.emit(false);
        Ok(())
    }

    pub(super) fn emit(&mut self, force: bool) {
        let downloaded = self.completed.load(Ordering::Relaxed);
        let delta = downloaded
            .checked_sub(self.last_emitted_bytes)
            .expect("fastembed aggregated download progress must be monotonic");
        if !force && delta < 256 * 1024 {
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
    let _ = tx.send(DownloadProgress {
        downloaded_bytes,
        total_bytes,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregated_progress_rejects_bytes_beyond_declared_total() {
        let completed = Arc::new(AtomicU64::new(4));
        let mut progress =
            AggregatedDownloadProgress::new(None, completed.clone(), 5, "model.onnx".to_string());

        assert!(progress.update(2).is_err());
        assert_eq!(completed.load(Ordering::Relaxed), 4);
    }
}
