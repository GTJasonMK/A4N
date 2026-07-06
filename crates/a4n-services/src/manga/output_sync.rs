use super::pipeline_prompt::MangaPromptGenerationTrace;
use super::{MANGA_PAGE_PROMPT_TYPE, MangaService, PIPELINE_STATUS_PROMPT_TYPE};
use a4n_core::error::AppError;
use a4n_core::models::{
    chapter::chapter_manga_prompt,
    generation::{generation_run, generation_run_target},
};
use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

use crate::asset_lineage_sync::AssetLineageSyncService;
use crate::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    delete_generation_run_targets_for, generation_checkpoint_fingerprint_sha256,
    record_completed_generation_run_with_checkpoint,
};

#[derive(Debug)]
pub(super) struct MangaPromptDraft {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) page_number: i32,
    pub(super) panel_number: i32,
    pub(super) prompt_type: String,
    pub(super) image_prompt: String,
    pub(super) dialogue_text: Option<String>,
    pub(super) characters_in_frame: Option<Value>,
    pub(super) layout: Option<Value>,
    pub(super) camera_angle: Option<String>,
    pub(super) dialogue_position: Option<String>,
    pub(super) status: String,
    pub(super) generation_traces: Vec<MangaPromptGenerationTrace>,
}

impl MangaService {
    pub(super) async fn sync_generated_outputs_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
        drafts: Vec<MangaPromptDraft>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let existing = chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
            .filter(chapter_manga_prompt::Column::PromptType.ne("dsl"))
            .filter(chapter_manga_prompt::Column::PromptType.ne(PIPELINE_STATUS_PROMPT_TYPE))
            .order_by_asc(chapter_manga_prompt::Column::PromptType)
            .order_by_asc(chapter_manga_prompt::Column::PageNumber)
            .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
            .order_by_asc(chapter_manga_prompt::Column::Id)
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        let mut existing_by_identity =
            HashMap::<(String, i32, i32), chapter_manga_prompt::Model>::new();
        let mut duplicate_ids = Vec::new();
        for row in existing {
            let identity = (row.prompt_type.clone(), row.page_number, row.panel_number);
            if let Some(current) = existing_by_identity.get(&identity) {
                if should_prefer_existing_manga_prompt(&row, current) {
                    duplicate_ids.push(current.id);
                    existing_by_identity.insert(identity, row);
                } else {
                    duplicate_ids.push(row.id);
                }
                continue;
            }
            existing_by_identity.insert(identity, row);
        }
        if !duplicate_ids.is_empty() {
            delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &duplicate_ids).await?;
            chapter_manga_prompt::Entity::delete_many()
                .filter(chapter_manga_prompt::Column::Id.is_in(duplicate_ids))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
        }

        let now = Utc::now().naive_utc();
        let mut rows = Vec::with_capacity(drafts.len());
        let mut trace_bindings = Vec::new();
        for draft in drafts {
            let generation_traces = draft.generation_traces.clone();
            let identity = (
                draft.prompt_type.clone(),
                draft.page_number,
                draft.panel_number,
            );
            let row = if let Some(existing) = existing_by_identity.remove(&identity) {
                update_manga_prompt_from_draft(existing, draft, now, &txn).await?
            } else {
                insert_manga_prompt_from_draft(draft, now, &txn).await?
            };
            for trace in generation_traces {
                trace_bindings.push((row.clone(), trace));
            }
            rows.push(row);
        }

        let stale_ids = existing_by_identity
            .into_values()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        if !stale_ids.is_empty() {
            delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &stale_ids).await?;
            chapter_manga_prompt::Entity::delete_many()
                .filter(chapter_manga_prompt::Column::Id.is_in(stale_ids))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
        }

        for (row, trace) in trace_bindings {
            self.record_chapter_manga_prompt_generation_run(&txn, &row, trace)
                .await?;
        }
        txn.commit().await.map_err(AppError::Database)?;
        AssetLineageSyncService::new(self.db.clone())
            .sync_project_asset_lineage(project_id)
            .await?;
        Ok(rows)
    }

    async fn record_chapter_manga_prompt_generation_run<C>(
        &self,
        db: &C,
        row: &chapter_manga_prompt::Model,
        trace: MangaPromptGenerationTrace,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        if self
            .chapter_manga_prompt_generation_trace_exists(db, row.id, &trace)
            .await?
        {
            return Ok(());
        }
        let source_refs_json = row
            .layout
            .as_ref()
            .and_then(|layout| layout.get("source"))
            .cloned()
            .map(|source| json!({ "source": source }));
        let media_refs_json = row.layout.as_ref().and_then(|layout| {
            layout.get("reference_bindings").cloned().map(|bindings| {
                json!({
                    "reference_bindings": bindings,
                })
            })
        });
        let call_key = trace.call_key.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 call_key: {}",
                trace.prompt_name
            ))
        })?;
        let input_fingerprint_json = trace.input_fingerprint_json.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 input_fingerprint_json: {}",
                trace.prompt_name
            ))
        })?;
        let output_json = json!({
            "chapter_manga_prompt_id": row.id,
            "chapter_id": row.chapter_id,
            "prompt_type": row.prompt_type.clone(),
            "page_number": row.page_number,
            "panel_number": row.panel_number,
            "image_prompt": row.image_prompt.clone(),
            "dialogue_text": row.dialogue_text.clone(),
            "characters_in_frame": row.characters_in_frame.clone(),
            "layout": row.layout.clone(),
            "camera_angle": row.camera_angle.clone(),
            "dialogue_position": row.dialogue_position.clone(),
            "status": row.status.clone(),
        });
        let artifact_refs_json = json!({
            "targets": [{
                "target_kind": row.prompt_type.clone(),
                "target_table": "chapter_manga_prompt",
                "target_id": row.id,
            }],
        });
        record_completed_generation_run_with_checkpoint(
            db,
            RecordCompletedGenerationRunInput {
                project_id: row.project_id,
                workflow: "manga_prompt_generation".to_string(),
                step_key: trace.prompt_name.clone(),
                title: format!("漫画提示词生成: {}", row.prompt_type),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!([trace.prompt_name])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(trace.messages_json.clone()),
                rendered_prompts_json: None,
                input_variables_json: None,
                source_refs_json: source_refs_json.clone(),
                media_refs_json: media_refs_json.clone(),
                request_json: Some(trace.request_json.clone()),
                response_summary_json: Some(json!({
                    "chapter_manga_prompt_id": row.id,
                    "prompt_type": row.prompt_type,
                    "page_number": row.page_number,
                    "output": trace.response_text.clone(),
                    "llm_usage": trace.usage.clone(),
                })),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: row.prompt_type.clone(),
                    target_table: "chapter_manga_prompt".to_string(),
                    target_id: row.id,
                }],
            },
            GenerationRunCheckpointContext {
                workflow_run_id: trace.workflow_run_id,
                call_key: Some(call_key),
                input_fingerprint_sha256: Some(generation_checkpoint_fingerprint_sha256(
                    &input_fingerprint_json,
                )?),
                input_fingerprint_json: Some(input_fingerprint_json),
                raw_response_json: Some(json!({
                    "text": trace.response_text,
                    "usage": trace.usage,
                    "context_manifest_summary": trace.context_manifest_summary_json,
                })),
                parsed_output_json: Some(output_json),
                validation_json: None,
                artifact_refs_json: Some(artifact_refs_json),
                attempt: None,
            },
        )
        .await?;
        Ok(())
    }

    async fn chapter_manga_prompt_generation_trace_exists<C>(
        &self,
        db: &C,
        prompt_id: i64,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<bool, AppError>
    where
        C: ConnectionTrait,
    {
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq("chapter_manga_prompt"))
            .filter(generation_run_target::Column::TargetId.eq(prompt_id))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        if targets.is_empty() {
            return Ok(false);
        }
        let run_ids = targets
            .iter()
            .map(|target| target.generation_run_id)
            .collect::<BTreeSet<_>>();
        let runs = generation_run::Entity::find()
            .filter(generation_run::Column::Id.is_in(run_ids))
            .filter(generation_run::Column::StepKey.eq(&trace.prompt_name))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        Ok(runs.into_iter().any(|run| {
            run.response_summary_json
                .as_ref()
                .and_then(|summary| summary.get("output"))
                .and_then(Value::as_str)
                == Some(trace.response_text.as_str())
        }))
    }
}

fn should_prefer_existing_manga_prompt(
    candidate: &chapter_manga_prompt::Model,
    current: &chapter_manga_prompt::Model,
) -> bool {
    match (
        candidate.generated_image_id.is_some(),
        current.generated_image_id.is_some(),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate.updated_at > current.updated_at,
    }
}

fn manga_prompt_content_matches(
    existing: &chapter_manga_prompt::Model,
    draft: &MangaPromptDraft,
) -> bool {
    existing.chapter_id == draft.chapter_id
        && existing.project_id == draft.project_id
        && existing.page_number == draft.page_number
        && existing.panel_number == draft.panel_number
        && existing.prompt_type == draft.prompt_type
        && existing.image_prompt == draft.image_prompt
        && existing.dialogue_text == draft.dialogue_text
        && existing.characters_in_frame == draft.characters_in_frame
        && existing.layout == draft.layout
        && existing.camera_angle == draft.camera_angle
        && existing.dialogue_position == draft.dialogue_position
}

async fn update_manga_prompt_from_draft<C>(
    existing: chapter_manga_prompt::Model,
    draft: MangaPromptDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, AppError>
where
    C: ConnectionTrait,
{
    let content_matches = manga_prompt_content_matches(&existing, &draft);
    let keep_rendered_image = content_matches
        && existing.prompt_type == MANGA_PAGE_PROMPT_TYPE
        && existing.generated_image_id.is_some();
    if content_matches && (keep_rendered_image || existing.status == draft.status) {
        return Ok(existing);
    }

    let mut active: chapter_manga_prompt::ActiveModel = existing.into();
    active.chapter_id = Set(draft.chapter_id);
    active.project_id = Set(draft.project_id);
    active.page_number = Set(draft.page_number);
    active.panel_number = Set(draft.panel_number);
    active.prompt_type = Set(draft.prompt_type);
    active.image_prompt = Set(draft.image_prompt);
    active.dialogue_text = Set(draft.dialogue_text);
    active.characters_in_frame = Set(draft.characters_in_frame);
    active.layout = Set(draft.layout);
    if !keep_rendered_image {
        active.generated_image_id = Set(None);
        active.status = Set(draft.status);
    }
    active.camera_angle = Set(draft.camera_angle);
    active.dialogue_position = Set(draft.dialogue_position);
    active.updated_at = Set(now);
    active.update(db).await.map_err(AppError::Database)
}

async fn insert_manga_prompt_from_draft<C>(
    draft: MangaPromptDraft,
    now: chrono::NaiveDateTime,
    db: &C,
) -> Result<chapter_manga_prompt::Model, AppError>
where
    C: ConnectionTrait,
{
    chapter_manga_prompt::ActiveModel {
        chapter_id: Set(draft.chapter_id),
        project_id: Set(draft.project_id),
        page_number: Set(draft.page_number),
        panel_number: Set(draft.panel_number),
        prompt_type: Set(draft.prompt_type),
        image_prompt: Set(draft.image_prompt),
        dialogue_text: Set(draft.dialogue_text),
        characters_in_frame: Set(draft.characters_in_frame),
        layout: Set(draft.layout),
        generated_image_id: Set(None),
        camera_angle: Set(draft.camera_angle),
        dialogue_position: Set(draft.dialogue_position),
        status: Set(draft.status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}
