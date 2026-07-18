use super::stage::report;
use crate::blueprint::{
    BlueprintProgressReporter, OrchestratorContext, blueprint_db_error,
    character_profile_seed_error,
};
use crate::generation::generation_error;
use a4n_core::blueprint::{BLUEPRINT_STAGE_SYSTEM_PROMPT, ParsedBlueprint};
use a4n_core::error::AppError;
use a4n_core::models::project::novel_blueprint;
use a4n_db::blueprint::{NovelBlueprintUpsertInput, upsert_novel_blueprint_record};
use a4n_db::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_llm::{
    provider::ChatRequest,
    trace::{chat_request_messages_json, text_response_summary_json},
};
use chrono::Utc;
use serde_json::{Value, json};

pub(super) struct BlueprintGenerationCompletionInput<'a> {
    pub(super) project_id: i64,
    pub(super) novel_title: &'a str,
    pub(super) genre: &'a str,
    pub(super) target_length: i32,
    pub(super) user_payload: Value,
    pub(super) trace_requests: Vec<(&'static str, ChatRequest)>,
    pub(super) assembled_content: String,
    pub(super) parsed: ParsedBlueprint,
    pub(super) progress: BlueprintProgressReporter<'a>,
}

pub(super) async fn persist_generated_blueprint(
    context: &OrchestratorContext,
    input: BlueprintGenerationCompletionInput<'_>,
) -> Result<novel_blueprint::Model, AppError> {
    let BlueprintGenerationCompletionInput {
        project_id,
        novel_title,
        genre,
        target_length,
        user_payload,
        trace_requests,
        assembled_content,
        parsed,
        progress,
    } = input;

    report(progress, 9, "保存蓝图主体")?;
    let now = Utc::now().naive_utc();
    let existing = context.load_blueprint(project_id).await?;
    let next_version = context
        .next_blueprint_version(
            project_id,
            existing
                .as_ref()
                .map(|blueprint| blueprint.current_version.as_str()),
        )
        .await?;
    let chapters_per_part = parsed
        .needs_part_outlines
        .then_some(parsed.chapters_per_part);
    let blueprint = upsert_novel_blueprint_record(
        context.db.connection(),
        NovelBlueprintUpsertInput {
            project_id,
            title: parsed.title.clone(),
            genre: parsed.genre.clone(),
            style: parsed.style.clone(),
            tone: parsed.tone.clone(),
            target_audience: parsed.target_audience.clone(),
            one_sentence_summary: parsed.one_sentence_summary.clone(),
            full_synopsis: parsed.full_synopsis.clone(),
            world_setting: parsed.world_setting.clone(),
            needs_part_outlines: parsed.needs_part_outlines,
            total_chapters: parsed.total_chapters,
            chapters_per_part,
            current_version: next_version.clone(),
            now,
        },
        existing,
    )
    .await
    .map_err(blueprint_db_error)?;

    report(progress, 10, "保存蓝图版本快照")?;
    context
        .save_blueprint_version(
            project_id,
            next_version.clone(),
            Some(if next_version == "1.0" {
                "自动生成首版蓝图".to_string()
            } else {
                "重新生成蓝图".to_string()
            }),
            &parsed.raw,
        )
        .await?;

    report(progress, 11, "同步主要角色")?;
    context
        .replace_blueprint_characters(project_id, &parsed.characters)
        .await?;
    report(progress, 12, "同步人物关系、角色档案与项目状态")?;
    context
        .replace_blueprint_relationships(project_id, &parsed.relationships)
        .await?;
    a4n_db::character::sync_character_profiles_from_blueprint(
        context.db.connection(),
        project_id,
        now,
    )
    .await
    .map_err(character_profile_seed_error)?;
    context
        .sync_project_after_blueprint_generation(project_id, &blueprint, &parsed.raw)
        .await?;
    a4n_db::generation::record_completed_generation_run(
        context.db.connection(),
        RecordCompletedGenerationRunInput {
            project_id,
            workflow: "blueprint_generation".to_string(),
            step_key: "blueprint".to_string(),
            title: "生成故事蓝图".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: Some(context.llm.model_name().to_string()),
            prompt_template_names_json: Some(json!([
                "blueprint",
                BLUEPRINT_STAGE_SYSTEM_PROMPT,
                "blueprint_stage_user"
            ])),
            prompt_template_hashes_json: None,
            rendered_messages_json: Some(Value::Array(
                trace_requests
                    .iter()
                    .map(|(stage, request)| {
                        Ok(json!({
                            "stage": stage,
                            "messages": chat_request_messages_json(request)?,
                        }))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?,
            )),
            rendered_prompts_json: None,
            input_variables_json: Some(json!({
                "novel_title": novel_title,
                "genre": genre,
                "target_length": target_length,
                "generation_payload": user_payload,
            })),
            source_refs_json: Some(json!({
                "source_node": "inspiration",
                "source_kind": "inspiration_conversation",
            })),
            media_refs_json: None,
            request_json: Some(Value::Array(
                trace_requests
                    .iter()
                    .map(|(stage, request)| {
                        Ok(json!({
                            "stage": stage,
                            "request": context_aware_chat_request_trace_json(request)?,
                        }))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?,
            )),
            response_summary_json: Some(text_response_summary_json(
                &assembled_content,
                None,
                json!({
                    "title": parsed.title,
                    "current_version": blueprint.current_version,
                    "stage_count": trace_requests.len(),
                }),
            )),
            targets: vec![GenerationRunTraceTarget {
                target_kind: "novel_blueprint".to_string(),
                target_table: "novel_blueprint".to_string(),
                target_id: blueprint.id,
            }],
        },
    )
    .await
    .map_err(generation_error)?;
    Ok(blueprint)
}
