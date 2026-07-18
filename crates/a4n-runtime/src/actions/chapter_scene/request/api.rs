use super::{
    super::{
        access_errors::{required_scene_generation_inputs, verify_chapter_access},
        contracts::{ChapterSceneGenerationRequestContext, GenerateChapterSceneInput},
    },
    builder::build_scene_generation_request,
    context::build_scene_generation_context,
};
use crate::{
    AppState,
    actions::{
        prompt_records::render_prompt_template,
        request_preview::{
            ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
            model_request_preview,
        },
    },
};
use a4n_core::error::AppError;
use serde_json::json;
use std::collections::HashMap;

pub(in super::super) async fn build_scene_generation_request_for_input(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ChapterSceneGenerationRequestContext, AppError> {
    let ch = verify_chapter_access(state, user_id, input.chapter_id).await?;
    let (title, summary, scene_prompt) = required_scene_generation_inputs(&ch, input)?;
    let outline = match ch.outline_id {
        Some(outline_id) => state.orchestrator.chapter_outline_by_id(outline_id).await?,
        None => {
            state
                .orchestrator
                .chapter_outline_by_project_number(ch.project_id, ch.chapter_number)
                .await?
        }
    };
    let existing_scenes = state
        .orchestrator
        .chapter_scenes_by_chapter(input.chapter_id)
        .await?;
    let generation_context =
        build_scene_generation_context(&ch, outline.as_ref(), &existing_scenes, input.scene_number);
    let variables = HashMap::from([
        ("chapter_number".to_string(), json!(ch.chapter_number)),
        ("chapter_title".to_string(), json!(title)),
        ("scene_number".to_string(), json!(input.scene_number)),
    ]);
    let system_prompt = render_prompt_template(
        state,
        "chapter_scene_generation",
        variables,
        Some(ch.project_id),
    )
    .await
    .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))?;
    let request = build_scene_generation_request(
        system_prompt,
        ch.project_id,
        ch.chapter_number,
        title,
        summary,
        Some(&generation_context),
        input.scene_number,
        scene_prompt,
    )?;
    Ok(ChapterSceneGenerationRequestContext {
        project_id: ch.project_id,
        chapter_number: ch.chapter_number,
        call_key: format!(
            "chapter_scene_generation:{}:{}",
            ch.chapter_number, input.scene_number
        ),
        request,
    })
}

pub async fn preview_chapter_scene_request(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ModelRequestPreview, AppError> {
    let context = build_scene_generation_request_for_input(state, user_id, input).await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "chapter_scene_generation".to_string(),
            title: format!(
                "第 {} 章 · 场景 {}",
                context.chapter_number, input.scene_number
            ),
            prompt_name: Some("chapter_scene_generation".to_string()),
            state: "ready".to_string(),
            note: None,
            request: context.request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        "chapter_scene_generation",
        format!("第 {} 章场景生成请求预览", context.chapter_number),
        vec![step],
    ))
}
