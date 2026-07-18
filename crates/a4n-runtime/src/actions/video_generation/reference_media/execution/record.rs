use super::super::super::{
    AppState, CreateVideoRecord, provider_request_snapshot_value,
    started_video_generation_run_input,
};
use super::super::contracts::ReferenceMediaProviderRequestContext;
#[cfg(feature = "embedded-media-providers")]
use super::super::contracts::ReferenceMediaProviderRequestPreviewContext;
#[cfg(feature = "embedded-media-providers")]
use super::super::preview::reference_media_provider_request_snapshot;
use a4n_core::models::ai::generated_video;
use serde_json::json;

pub(super) async fn create_reference_media_video_generation_record(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    root_project_id: Option<i64>,
    context: &ReferenceMediaProviderRequestContext,
) -> Result<generated_video::Model, String> {
    let provider_request_snapshot = if state.runtime_profile.uses_sidecars() {
        let provider_request = crate::sidecar_clients::preview_video_provider_request(
            state.capabilities(),
            &context.cfg,
            context.request.clone(),
            Some(project_id),
            root_project_id,
        )
        .await?;
        let note = match context.provider_input_note.as_deref() {
            Some(note) => format!("生成时由 video sidecar 生成真实 provider 请求体；{note}"),
            None => "生成时由 video sidecar 生成真实 provider 请求体".to_string(),
        };
        provider_request_snapshot_value(
            "reference_media_video",
            "参考媒体视频 provider 请求",
            "video_provider_request",
            Some("reference_media_video"),
            Some(note),
            provider_request,
        )
    } else {
        #[cfg(feature = "embedded-media-providers")]
        {
            reference_media_provider_request_snapshot(
                &ReferenceMediaProviderRequestPreviewContext {
                    request: context.request.clone(),
                    config_id: Some(context.cfg.id),
                    provider_backend: Some(context.cfg.backend.clone()),
                    provider_input_note: context.provider_input_note.clone(),
                },
            )?
        }
        #[cfg(not(feature = "embedded-media-providers"))]
        {
            return Err("参考媒体视频生成不可用：当前 runtime 既未启用 sidecar profile，也未编译 embedded-media-providers".to_string());
        }
    };
    let reference_media = context
        .request
        .reference_media
        .as_ref()
        .ok_or_else(|| "参考媒体请求缺少 reference_media".to_string())?;
    let metadata = json!({
        "config_id": context.cfg.id,
        "provider_backend": context.cfg.backend,
        "reference_media": {
            "functionMode": "omni_reference",
            "image_file_count": reference_media.image_files.len(),
            "video_file_count": reference_media.video_files.len(),
            "audio_file_count": reference_media.audio_files.len(),
            "image_files": &reference_media.image_files,
            "video_files": &reference_media.video_files,
            "audio_files": &reference_media.audio_files,
        },
        "provider_request_snapshot": provider_request_snapshot,
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let video_record = CreateVideoRecord {
        project_id,
        chapter_id: Some(chapter_id),
        video_type: "reference_media_video".into(),
        workflow_mode: Some("reference_media".to_string()),
        prompt: context.request.prompt.clone(),
        negative_prompt: None,
        model: context.cfg.model.clone(),
        width: context.request.width,
        height: context.request.height,
        duration_secs: context.request.duration_secs as f32,
        fps: context.request.fps,
        source_video_id: None,
        source_image_id: None,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let generation_run = started_video_generation_run_input(
        &video_record,
        "generate_reference_media_video",
        "参考媒体视频 provider 生成",
        context.cfg.backend.clone(),
        Some("reference_media_video"),
        provider_request_snapshot,
    );
    state
        .orchestrator
        .create_video_record_with_generation_run(video_record, generation_run)
        .await
        .map_err(crate::actions::app_error_message)
}
