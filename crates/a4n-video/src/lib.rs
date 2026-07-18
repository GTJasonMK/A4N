use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_core::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoReferenceMediaRequest {
    pub image_files: Vec<String>,
    pub video_files: Vec<String>,
    pub audio_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoGenerationRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: i32,
    pub height: i32,
    pub duration_secs: i32,
    pub fps: i32,
    pub model: String,
    pub first_frame_image_url: Option<String>,
    pub last_frame_image_url: Option<String>,
    pub reference_image_urls: Vec<String>,
    pub reference_media: Option<VideoReferenceMediaRequest>,
}

#[derive(Debug, Clone)]
pub struct VideoGenerationResult {
    pub bytes: Vec<u8>,
    pub format: String,
    pub provider_job_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub enum VideoJobState {
    Pending,
    Running,
    Completed(VideoGenerationResult),
    Failed(String),
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
    ) -> Result<VideoGenerationResult, AppError>;

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError>;
}
