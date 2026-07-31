use super::super::GalgameMediaBindingResult;
use super::ImportGalgameAudioInput;
use crate::{
    AppState,
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::{persist_created_media_file, probe_created_audio_duration},
};
use a4n_core::files::paths::ProjectPaths;
use a4n_orchestrator::galgame::{GalgameAudioHostPort, PersistedGalgameAudioFile};
use async_trait::async_trait;
use chrono::Utc;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub async fn import_galgame_audio(
    state: &AppState,
    user_id: i64,
    input: ImportGalgameAudioInput,
) -> Result<GalgameMediaBindingResult, String> {
    let host_port = RuntimeGalgameAudioHostPort { state };
    state
        .galgame()
        .import_galgame_audio_for_user_validated(&host_port, user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

struct RuntimeGalgameAudioHostPort<'a> {
    state: &'a AppState,
}

#[async_trait]
impl GalgameAudioHostPort for RuntimeGalgameAudioHostPort<'_> {
    fn persist_audio_file(
        &self,
        project_id: i64,
        extension: &str,
        bytes: &[u8],
    ) -> Result<PersistedGalgameAudioFile, String> {
        let (path, relative_path, file_size) =
            persist_imported_galgame_audio(self.state.data_dir(), project_id, extension, bytes)?;
        let duration_secs = probe_created_audio_duration(&path, "Galgame 音频文件")?;
        Ok(PersistedGalgameAudioFile {
            relative_path,
            file_size,
            duration_secs,
        })
    }

    async fn remove_audio_file(&self, relative_path: &str) -> Result<(), String> {
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!("galgame-audio-cleanup:{relative_path}"),
            &[relative_path.to_string()],
        )
        .await
    }
}

pub(super) fn persist_imported_galgame_audio(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
    bytes: &[u8],
) -> Result<(PathBuf, String, i64), String> {
    let (path, mut output_file) = create_galgame_audio_output(data_dir, project_id, extension)?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(bytes)
                .map_err(|err| format!("写入 Galgame 音频文件失败：{err}"))
        },
        "Galgame 音频文件",
        "Galgame 音频文件",
        "Galgame 音频文件",
    )?;
    Ok((path, relative_path, file_size))
}

fn create_galgame_audio_output(
    data_dir: &Path,
    project_id: i64,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), String> {
    let dir = ProjectPaths::new(data_dir, project_id).root().join("audio");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建 Galgame 音频目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("galgame_audio_{stamp}{suffix}.{extension}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("创建 Galgame 音频文件失败：{err}")),
        }
    }
    Err("创建 Galgame 音频文件失败：文件名冲突过多".to_string())
}
