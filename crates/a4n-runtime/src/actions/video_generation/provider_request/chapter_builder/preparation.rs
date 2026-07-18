use a4n_core::models::{ai::video_gen_config, chapter::chapter_video_prompt};
use a4n_orchestrator::video::validate_video_duration_for_config_secs;

use super::super::{
    append_preview_duration_note, apply_chapter_video_prompt_preview_override,
    ensure_chapter_video_prompt_preview_metadata, ensure_chapter_video_prompt_preview_texts,
    provider_request_preview_config_id,
};
use crate::{
    AppState,
    actions::video_generation::{
        PreviewChapterVideoPromptOverrideInput, VideoProviderRequestPurpose,
        ensure_prompt_matches_current_chapter, resolve_project_chapter_video_source,
        resolve_video_config,
    },
};

pub(super) struct PreparedChapterProviderRequest {
    pub(super) prompt: chapter_video_prompt::Model,
    pub(super) cfg: video_gen_config::Model,
    pub(super) provider_input_note: Option<String>,
}

pub(super) async fn prepare_chapter_provider_request(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    config_id: Option<i64>,
    purpose: VideoProviderRequestPurpose,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<PreparedChapterProviderRequest, String> {
    let mut prompt = state
        .orchestrator
        .chapter_video_prompt(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    if prompt.project_id != project_id || prompt.chapter_id != chapter_id {
        return Err("视频提示词不属于该章节".into());
    }
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            resolve_project_chapter_video_source(state, project_id, chapter_id, "生成视频", true)
                .await?;
        }
        VideoProviderRequestPurpose::Preview => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "预览视频生成请求",
                false,
            )
            .await?;
        }
    }
    if prompt.prompt_type != "video_prompt" {
        return Err("请选择 video_prompt 类型的提示词生成视频".into());
    }
    let mut provider_input_note = None;
    if purpose == VideoProviderRequestPurpose::Preview {
        apply_chapter_video_prompt_preview_override(
            &mut prompt,
            prompt_override,
            &mut provider_input_note,
        );
        ensure_chapter_video_prompt_preview_texts(&mut prompt, &mut provider_input_note);
        ensure_chapter_video_prompt_preview_metadata(&mut prompt, &mut provider_input_note)?;
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        if prompt.status == "running" {
            return Err("当前视频提示词正在生成视频，请先等待完成后再重新生成".into());
        }
        if prompt.generated_video_id.is_some() {
            return Err("该视频提示词已生成视频，请先删除对应视频后再重新生成".into());
        }
        ensure_prompt_matches_current_chapter(state, &prompt).await?;
    }
    let config_id = match purpose {
        VideoProviderRequestPurpose::Generate => config_id,
        VideoProviderRequestPurpose::Preview => {
            provider_request_preview_config_id(state, &prompt, config_id).await?
        }
    };
    let cfg = resolve_video_config(state, config_id).await?;
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            validate_video_duration_for_config_secs(&cfg, prompt.duration_secs)?;
        }
        VideoProviderRequestPurpose::Preview => {
            append_preview_duration_note(&mut provider_input_note, &cfg, prompt.duration_secs);
        }
    }

    Ok(PreparedChapterProviderRequest {
        prompt,
        cfg,
        provider_input_note,
    })
}
