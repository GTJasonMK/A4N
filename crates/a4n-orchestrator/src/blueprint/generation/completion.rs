use a4n_core::blueprint::{ParsedBlueprint, project_status_after_blueprint_generation};
use a4n_core::error::AppError;
use a4n_core::models::project::novel_blueprint;
use a4n_core::state::validator::StateValidator;
use a4n_db::blueprint::{
    NovelBlueprintUpsertInput, SyncProjectAfterBlueprintInput, sync_project_after_blueprint_record,
    upsert_novel_blueprint_record,
};
use a4n_db::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, record_completed_generation_run,
};
use a4n_db::project::{
    ProjectStatusHistoryInput, insert_project_status_history, project_by_id_record,
    sync_blueprint_characters, sync_blueprint_relationships,
};
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::{
    chat_request_messages_json, context_aware_chat_request_trace_json, text_response_summary_json,
};
use chrono::{NaiveDateTime, Utc};
use sea_orm::DatabaseTransaction;
use serde_json::{Value, json};

use crate::blueprint::persistence::{blueprint_character_drafts, blueprint_relationship_drafts};
use crate::blueprint::{blueprint_db_error, blueprint_sync_error, character_profile_seed_error};
use crate::generation::generation_error;
use crate::project::{
    project_mutation_error_to_app_error, project_status_write_error_to_app_error,
};

pub(super) struct BlueprintGenerationCompletionInput {
    pub(super) project_id: i64,
    pub(super) novel_title: String,
    pub(super) genre: String,
    pub(super) target_length: i32,
    pub(super) user_payload: Value,
    pub(super) trace_requests: Vec<(&'static str, ChatRequest)>,
    pub(super) assembled_content: String,
    pub(super) parsed: ParsedBlueprint,
    pub(super) existing: Option<novel_blueprint::Model>,
    pub(super) next_version: String,
    pub(super) model_name: String,
}

pub(super) struct BlueprintCompletionOutcome {
    pub(super) blueprint: novel_blueprint::Model,
    pub(super) status_change: Option<(String, String, NaiveDateTime)>,
}

pub(super) async fn persist_generated_blueprint_in_transaction(
    tx: &DatabaseTransaction,
    input: BlueprintGenerationCompletionInput,
) -> Result<BlueprintCompletionOutcome, AppError> {
    let BlueprintGenerationCompletionInput {
        project_id,
        novel_title,
        genre,
        target_length,
        user_payload,
        trace_requests,
        assembled_content,
        parsed,
        existing,
        next_version,
        model_name,
    } = input;
    let now = Utc::now().naive_utc();
    let chapters_per_part = parsed
        .needs_part_outlines
        .then_some(parsed.chapters_per_part);
    let blueprint = upsert_novel_blueprint_record(
        tx,
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

    a4n_db::blueprint::insert_blueprint_version_record(
        tx,
        a4n_db::blueprint::InsertBlueprintVersionInput {
            project_id,
            version: next_version.clone(),
            change_description: Some(if next_version == "1.0" {
                "自动生成首版蓝图".to_string()
            } else {
                "重新生成蓝图".to_string()
            }),
            blueprint_data: parsed.raw.clone(),
            now,
            created_by: Some("system".to_string()),
        },
    )
    .await
    .map_err(blueprint_db_error)?;

    sync_blueprint_characters(
        tx,
        project_id,
        blueprint_character_drafts(&parsed.characters),
        now,
    )
    .await
    .map_err(blueprint_sync_error)?;
    sync_blueprint_relationships(
        tx,
        project_id,
        blueprint_relationship_drafts(&parsed.relationships),
        now,
    )
    .await
    .map_err(blueprint_sync_error)?;
    a4n_db::character::sync_character_profiles_from_blueprint(tx, project_id, now)
        .await
        .map_err(character_profile_seed_error)?;

    let project = project_by_id_record(tx, project_id)
        .await
        .map_err(project_mutation_error_to_app_error)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let previous_status = StateValidator::parse_status(&project.status)?;
    let next_status = project_status_after_blueprint_generation(previous_status);
    sync_project_after_blueprint_record(
        tx,
        SyncProjectAfterBlueprintInput {
            project_id,
            title: blueprint.title.clone(),
            status: next_status.as_str().to_string(),
            target_chapters: blueprint.total_chapters.ok_or_else(|| {
                AppError::InvalidArgument("蓝图缺少 total_chapters，无法同步项目".to_string())
            })?,
            total_chapters: blueprint.total_chapters,
            chapters_per_part: blueprint.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&parsed.raw)?,
            now,
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    let status_change = (previous_status != next_status).then(|| {
        (
            previous_status.as_str().to_string(),
            next_status.as_str().to_string(),
            now,
        )
    });
    if let Some((from_status, to_status, changed_at)) = &status_change {
        insert_project_status_history(
            tx,
            ProjectStatusHistoryInput {
                project_id,
                from_status: from_status.clone(),
                to_status: to_status.clone(),
                reason: Some("蓝图生成完成".to_string()),
                changed_by: Some("system".to_string()),
                changed_at: *changed_at,
            },
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;
    }

    record_completed_generation_run(
        tx,
        RecordCompletedGenerationRunInput {
            project_id,
            workflow: "blueprint_generation".to_string(),
            step_key: "blueprint".to_string(),
            title: "生成故事蓝图".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: Some(model_name),
            prompt_template_names_json: Some(json!([
                "blueprint",
                a4n_core::blueprint::BLUEPRINT_STAGE_SYSTEM_PROMPT,
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

    Ok(BlueprintCompletionOutcome {
        blueprint,
        status_change,
    })
}
