use super::*;

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("runtime_profile", &self.runtime_profile)
            .finish()
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::media::MediaAudioProbeHostPort for AppState {
    async fn probe_audio_duration_secs(
        &self,
        relative_data_path: &str,
    ) -> Result<f32, a4n_error::AppError> {
        let path = self.config.data_dir.join(relative_data_path);
        let input = self
            .data_media_file_ref(&path, "音频文件")
            .map_err(|current| a4n_error::AppError::InvalidState {
                current,
                operation: "构造音频探测路径".into(),
            })?;
        let output = self
            .execute_media(a4n_media_contract::MediaExecuteInput::ProbeAudioDuration { input })
            .await
            .map_err(|current| a4n_error::AppError::InvalidState {
                current,
                operation: "探测音频时长".into(),
            })?;
        match output {
            a4n_media_contract::MediaExecuteOutput::AudioDuration { duration_secs } => {
                Ok(duration_secs)
            }
            output => Err(a4n_error::AppError::InvalidState {
                current: format!("media worker 返回了不匹配的输出：{output:?}"),
                operation: "探测音频时长".into(),
            }),
        }
    }
}
