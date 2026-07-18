use super::super::super::CharacterAiPreviewStep;
use super::super::super::sync_context::{
    CharacterSyncAnchorMode, build_character_sync_context_by_name,
};
use super::super::super::sync_scope::profile_sync_skip_reason;
use super::super::super::sync_types::CharacterSyncContext;
use super::super::contracts::CharacterSyncStage;
use super::super::request::build_character_sync_stage_request;
use crate::AppState;
use crate::actions::{access, character_implicit_review::IMPLICIT_UPDATE_PROMPT_NAME};
use serde_json::json;

pub(super) async fn character_sync_stage_preview_steps_for_context(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    name: &str,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    let mut steps = Vec::new();
    let mut previous_steps = json!({});
    for (index, stage) in CharacterSyncStage::ALL.iter().copied().enumerate() {
        if index == 0 {
            steps.push(CharacterAiPreviewStep::llm(
                format!("sync_{}_{}", stage.key(), context.profile.id),
                format!("{name}：{}", stage.title()),
                stage.prompt_name(),
                "ready",
                Some(format!(
                    "同步第 {}-{} 章；后续阶段依赖该阶段输出",
                    context.start_chapter, context.target_chapter
                )),
                build_character_sync_stage_request(
                    state,
                    project_id,
                    context,
                    stage,
                    &previous_steps,
                )
                .await?,
            ));
        } else {
            steps.push(CharacterAiPreviewStep::status(
                format!("sync_{}_{}", stage.key(), context.profile.id),
                format!("{name}：{}", stage.title()),
                stage.prompt_name(),
                "blocked",
                Some("该阶段依赖前序角色同步阶段的结构化输出".to_string()),
            ));
        }
        previous_steps[stage.key()] = json!({ stage.root_field(): [] });
    }
    steps.push(CharacterAiPreviewStep::status(
        format!("sync_implicit_classification_{}", context.profile.id),
        format!("{name}：行为隐性属性分类请求"),
        "implicit_classification",
        "blocked",
        Some("该请求依赖角色同步 behavior_records 阶段输出".to_string()),
    ));
    steps.push(CharacterAiPreviewStep::status(
        format!("sync_implicit_review_{}", context.profile.id),
        format!("{name}：隐性属性复审请求"),
        IMPLICIT_UPDATE_PROMPT_NAME,
        "blocked",
        Some("该请求依赖角色同步阶段输出和行为分类结果".to_string()),
    ));
    Ok(steps)
}

pub(crate) async fn character_sync_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let context = build_character_sync_context_by_name(
        state,
        project_id,
        name,
        chapter_number,
        CharacterSyncAnchorMode::RequireExistingCurrent,
    )
    .await?;
    character_sync_preview_steps_for_context(state, project_id, &context, name).await
}

pub(in crate::actions::character_ai) async fn character_sync_preview_steps_for_context(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    name: &str,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    if profile_sync_skip_reason(&context.profile, context.target_chapter).is_some() {
        return Ok(vec![CharacterAiPreviewStep::status(
            format!("sync_{}", context.profile.id),
            format!("{name}：无需同步"),
            "character_sync_evidence_events",
            "reused",
            Some(format!(
                "已同步到第 {} 章或目标章节无需处理",
                context.target_chapter
            )),
        )]);
    }

    character_sync_stage_preview_steps_for_context(state, project_id, context, name).await
}
