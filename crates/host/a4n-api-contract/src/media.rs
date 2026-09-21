use a4n_error::AppError;
use async_trait::async_trait;

/// Host-owned media probes used by application services that persist files.
#[async_trait]
pub trait MediaAudioProbeHostPort: Send + Sync {
    async fn probe_audio_duration_secs(&self, relative_data_path: &str) -> Result<f32, AppError>;
}
