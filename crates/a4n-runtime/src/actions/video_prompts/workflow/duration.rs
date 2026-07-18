use crate::AppState;
use a4n_core::models::ai::video_gen_config;
use a4n_orchestrator::video::{
    ChapterVideoPromptOptions, duration_capability_for_config,
    validate_video_duration_for_config_secs,
};

pub(in crate::actions::video_prompts) async fn apply_default_duration_capability(
    state: &AppState,
    p: &mut ChapterVideoPromptOptions,
) -> Result<(), String> {
    if let Some(cfg) = default_video_config_for_runtime(state).await?.as_ref() {
        validate_video_duration_for_config_secs(cfg, p.duration_secs)?;
        let c = duration_capability_for_config(cfg)?;
        c.validate_duration_options(&p.duration_secs_options)
            .map_err(|e| e.to_string())?;
        p.duration_capability = Some(c);
    }
    Ok(())
}
pub(in crate::actions::video_prompts) async fn apply_default_duration_capability_for_preview(
    state: &AppState,
    p: &mut ChapterVideoPromptOptions,
    notes: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(cfg) = default_video_config_for_runtime(state).await?.as_ref() {
        apply_preview_duration_capability(cfg, p, notes);
    }
    Ok(())
}
pub(in crate::actions::video_prompts) async fn default_video_config_for_runtime(
    state: &AppState,
) -> Result<Option<video_gen_config::Model>, String> {
    if let Some(c) = crate::runtime_config::current_video_model() {
        return Ok(Some(c));
    }
    state
        .orchestrator
        .default_video_config()
        .await
        .map(Some)
        .map_err(crate::actions::app_error_message)
}
pub(in crate::actions::video_prompts) fn apply_preview_duration_capability(
    cfg: &video_gen_config::Model,
    p: &mut ChapterVideoPromptOptions,
    notes: &mut Vec<String>,
) {
    match duration_capability_for_config(cfg) {
        Ok(c) => {
            if let Err(e) = c.validate_duration(p.duration_secs) {
                notes.push(format!(
                    "当前默认视频配置不支持 {} 秒；预览仍展示请求结构，真实生成会失败：{}",
                    p.duration_secs, e
                ));
            }
            if let Err(e) = c.validate_duration_options(&p.duration_secs_options) {
                notes.push(format!(
                    "当前默认视频配置不支持候选时长 {:?}；预览仍展示请求结构，真实生成会失败：{}",
                    p.duration_secs_options, e
                ));
            }
            p.duration_capability = Some(c);
        }
        Err(e) => notes.push(format!(
            "当前默认视频配置时长能力无效；预览仍展示请求结构，真实生成会失败：{e}"
        )),
    }
}
