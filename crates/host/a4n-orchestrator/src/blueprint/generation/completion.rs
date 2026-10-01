use a4n_db::blueprint::{
    NovelBlueprintUpsertInput, SyncProjectAfterBlueprintInput, blueprint_version_snapshot,
    next_blueprint_version_record, sync_project_after_blueprint_record,
    upsert_novel_blueprint_record, validate_blueprint_chapter_plan,
};
use a4n_db::generation::record_completed_generation_run_in_transaction;
use a4n_db::project::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    insert_project_status_history, merge_blueprint_characters, merge_blueprint_relationships,
    project_by_id_record,
};
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_domain::project::{
    BLUEPRINT_BASE_PROMPT, BLUEPRINT_GENERATE_PROGRESS_TOTAL, BLUEPRINT_GENERATION_KIND,
    BLUEPRINT_GENERATION_PROMPTS, BlueprintRecord, ParsedBlueprint, ProjectStatus,
    ProjectStatusHistoryInput, project_status_after_blueprint_generation,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_llm_trace::{
    chat_request_messages_json, context_aware_chat_request_trace_json, text_response_summary_json,
};
use a4n_progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_service_protocol::ProgressEvent;
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
    pub(super) user_payload: Value,
    pub(super) trace_requests: Vec<(&'static str, ChatRequest)>,
    pub(super) assembled_content: String,
    pub(super) parsed: ParsedBlueprint,
    pub(super) expected_source_snapshot: Value,
    pub(super) model_name: String,
}

pub(super) struct BlueprintCompletionOutcome {
    pub(super) blueprint: BlueprintRecord,
    pub(super) status_change: Option<(ProjectStatus, ProjectStatus, NaiveDateTime)>,
}

pub(super) async fn persist_generated_blueprint_in_transaction(
    tx: &DatabaseTransaction,
    input: BlueprintGenerationCompletionInput,
) -> Result<(BlueprintCompletionOutcome, Vec<ProgressEvent>), AppError> {
    let BlueprintGenerationCompletionInput {
        project_id,
        user_payload,
        trace_requests,
        assembled_content,
        parsed,
        expected_source_snapshot,
        model_name,
    } = input;
    let mut progress_events = Vec::with_capacity(4);
    let locked_sources =
        a4n_db::outline::load_outline_generation_source_rows_for_update(tx, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if locked_sources.project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE {
        return Err(AppError::InvalidState {
            current: locked_sources.project.lifecycle_gate.clone(),
            operation: "保存生成的蓝图".to_string(),
        });
    }
    if a4n_db::outline::outline_generation_source_snapshot(&locked_sources)
        != expected_source_snapshot
    {
        return Err(AppError::InvalidState {
            current: "blueprint_generation_source_changed".to_string(),
            operation: "保存生成的蓝图（项目正式资料已变化，请重新生成）".to_string(),
        });
    }
    validate_blueprint_chapter_plan(&locked_sources, parsed.total_chapters)
        .map_err(blueprint_db_error)?;
    let existing = locked_sources.blueprint.clone();
    let next_version = next_blueprint_version_record(
        tx,
        project_id,
        existing
            .as_ref()
            .map(|blueprint| blueprint.current_version.as_str()),
    )
    .await
    .map_err(blueprint_db_error)?;
    let now = Utc::now().naive_utc();
    let chapters_per_part = parsed
        .needs_part_outlines
        .then_some(parsed.chapters_per_part);
    progress_events.push(blueprint_persist_progress(9, "正在保存蓝图主体"));
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

    progress_events.push(blueprint_persist_progress(10, "正在同步主要角色"));
    merge_blueprint_characters(
        tx,
        project_id,
        blueprint_character_drafts(&parsed.characters),
        now,
    )
    .await
    .map_err(blueprint_sync_error)?;
    progress_events.push(blueprint_persist_progress(11, "正在同步人物关系与角色档案"));
    merge_blueprint_relationships(
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
    let characters = blueprint_characters_for_project(tx, project_id).await?;
    let relationships = blueprint_relationships_for_project(tx, project_id).await?;
    let snapshot = blueprint_version_snapshot(&blueprint, &characters, &relationships);
    progress_events.push(blueprint_persist_progress(
        12,
        "正在保存版本快照并同步项目状态",
    ));
    a4n_db::blueprint::insert_blueprint_version_record(
        tx,
        a4n_db::blueprint::InsertBlueprintVersionInput {
            project_id,
            version: next_version.clone(),
            change_description: if next_version == "1.0" {
                "自动生成首版蓝图".to_string()
            } else {
                "重新生成蓝图".to_string()
            },
            blueprint_data: snapshot.clone(),
            now,
            created_by: "system".to_string(),
        },
    )
    .await
    .map_err(blueprint_db_error)?;

    let project = project_by_id_record(tx, project_id)
        .await
        .map_err(project_mutation_error_to_app_error)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let previous_status = ProjectStatus::parse_storage(&project.status)?;
    let next_status = project_status_after_blueprint_generation(previous_status);
    sync_project_after_blueprint_record(
        tx,
        SyncProjectAfterBlueprintInput {
            project_id,
            title: blueprint.title.clone(),
            status: next_status,
            target_chapters: blueprint.total_chapters.ok_or_else(|| {
                AppError::InvalidArgument("蓝图缺少 total_chapters，无法同步项目".to_string())
            })?,
            total_chapters: blueprint.total_chapters,
            chapters_per_part: blueprint.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&snapshot)?,
            now,
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    let status_change =
        (previous_status != next_status).then_some((previous_status, next_status, now));
    if let Some((from_status, to_status, changed_at)) = &status_change {
        insert_project_status_history(
            tx,
            ProjectStatusHistoryInput {
                project_id,
                from_status: *from_status,
                to_status: *to_status,
                reason: "蓝图生成完成".to_string(),
                changed_by: "system".to_string(),
                changed_at: *changed_at,
            },
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;
    }

    record_completed_generation_run_in_transaction(
        tx,
        RecordCompletedGenerationRunInput {
            project_id,
            workflow: BLUEPRINT_GENERATION_KIND.to_string(),
            step_key: BLUEPRINT_BASE_PROMPT.to_string(),
            title: "生成故事蓝图".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: Some(model_name),
            prompt_template_names_json: Some(json!(BLUEPRINT_GENERATION_PROMPTS)),
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
            input_variables_json: Some(user_payload),
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

    Ok((
        BlueprintCompletionOutcome {
            blueprint: a4n_db::blueprint::blueprint_record(&blueprint),
            status_change,
        },
        progress_events,
    ))
}

fn blueprint_persist_progress(current: u64, message: &str) -> ProgressEvent {
    ProgressEvent {
        stage_code: "blueprint_persist".to_string(),
        phase_kind: ProgressPhaseKind::Save,
        message: message.to_string(),
        current: Some(current),
        total: Some(BLUEPRINT_GENERATE_PROGRESS_TOTAL as u64),
        unit_label: None,
        item_key: None,
        item_index: None,
        item_total: None,
        severity: ProgressSeverity::Info,
        estimated: false,
    }
}
