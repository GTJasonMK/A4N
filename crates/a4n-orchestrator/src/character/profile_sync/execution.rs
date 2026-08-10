use a4n_api_contract::character::{
    CharacterProfileSyncedResult, CharacterProfileSyncRange, CharacterProfileSyncSkipReason,
    SyncCharacterProfileResult,
};
use a4n_core::error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    rag::{CharacterSyncRagInput, CharacterSyncRagSummary},
};

use super::{
    super::{
        AppliedCharacterSyncPersistenceRecord, CharacterImplicitClassificationInput,
        CharacterProfileSyncChapterStateDraft, CharacterProfileSyncPreviewStep,
        CharacterProfileSyncStageProgress, CharacterProfileSyncStageTracePort,
        CharacterSyncChapterStateRecordInput, CharacterSyncImplicitReviewInput,
        CharacterSyncPersistenceInput, CharacterSyncRangeMode, RecordCompletedGenerationRunInput,
        analysis_profile_sync_skip_reason, character_profile_skipped_result,
        pending_character_sync_attribute_events, profile_sync_skip_reason,
    },
    context::PreparedCharacterProfileSync,
    contracts::{
        CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL, CharacterProfileSyncApplicationInput,
        CharacterProfileSyncProgress,
    },
    write_steps::{BuildCharacterSyncWriteStepsInput, build_character_sync_write_step_records},
};

impl OrchestratorContext {
    pub async fn prepare_character_profile_sync_preview_validated(
        &self,
        input: CharacterProfileSyncApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError> {
        let range_mode = input.range_mode;
        let prepared = self.prepare_character_profile_sync_context(input).await?;
        if let Some(reason) = profile_sync_skip_reason_for_mode(
            &prepared.profile,
            prepared.target_chapter,
            range_mode,
        ) {
            return Ok(vec![CharacterProfileSyncPreviewStep {
                id: format!("sync_{}", prepared.profile.id),
                title: format!("{}：无需同步", prepared.profile.name),
                prompt_name: "character_sync_evidence_events",
                status: "reused",
                note: Some(format!(
                    "已同步到第 {} 章或目标章节无需处理（{reason}）",
                    prepared.target_chapter
                )),
                request: None,
            }]);
        }
        self.prepare_character_profile_sync_stage_preview_validated(
            prepared.stage_input(None),
            prompt_renderer,
        )
        .await
    }

    pub async fn run_character_profile_sync_validated(
        &self,
        input: CharacterProfileSyncApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<SyncCharacterProfileResult, AppError> {
        let range_mode = input.range_mode;
        let prepared = self.prepare_character_profile_sync_context(input).await?;
        let name = prepared.profile.name.clone();
        if let Some(reason) = profile_sync_skip_reason_for_mode(
            &prepared.profile,
            prepared.target_chapter,
            range_mode,
        ) {
            report_progress(
                progress,
                CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL,
                &format!("{name}：无需同步到第 {} 章，跳过", prepared.target_chapter),
            )?;
            return Ok(SyncCharacterProfileResult::Skipped(
                character_profile_skipped_result(
                    &prepared.profile,
                    prepared.target_chapter,
                    reason,
                ),
            ));
        }

        let chapter_data_json = json!(&prepared.chapter_data);
        let workflow_run = self
            .begin_character_profile_sync_workflow_validated(
                prepared.project_id,
                prepared.profile.id,
                &prepared.profile.name,
                prepared.profile.last_synced_chapter,
                prepared.start_chapter,
                prepared.target_chapter,
                prepared.clear_to_chapter,
                &prepared.selected_chapter_numbers,
                &prepared.baseline_state,
                &prepared.profile.base_attributes,
                &chapter_data_json,
                progress.and_then(|progress| progress.active_task_id),
            )
            .await?;
        let result = self
            .execute_character_profile_sync(
                workflow_run.id,
                prepared,
                prompt_renderer,
                progress,
                trace_port,
            )
            .await;
        self.finalize_workflow_result(workflow_run.id, result, Some("角色同步完成"))
            .await
    }

    async fn execute_character_profile_sync(
        &self,
        workflow_run_id: i64,
        prepared: PreparedCharacterProfileSync,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<SyncCharacterProfileResult, AppError> {
        let staged = self
            .run_character_profile_sync_stages_validated(
                prepared.stage_input(Some(workflow_run_id)),
                prompt_renderer,
                progress.map(|progress| CharacterProfileSyncStageProgress {
                    reporter: progress.reporter,
                }),
                trace_port,
            )
            .await?;

        report_progress(
            progress,
            7,
            &format!(
                "{}：正在识别行为类型并检查隐性属性变化",
                prepared.profile.name
            ),
        )?;
        let classified = self
            .classify_character_sync_behaviors_validated(
                CharacterImplicitClassificationInput {
                    project_id: prepared.project_id,
                    workflow_run_id: Some(workflow_run_id),
                    profile_name: prepared.profile.name.clone(),
                    changes: staged.extraction.changes.clone(),
                    behaviors: staged.extraction.behaviors,
                    final_attributes: prepared.baseline_state.clone(),
                },
                prompt_renderer,
            )
            .await?;
        let implicit_review_decisions = self
            .decide_implicit_reviews_for_sync_validated(
                CharacterSyncImplicitReviewInput {
                    workflow_run_id: Some(workflow_run_id),
                    profile: &prepared.profile,
                    start_chapter: prepared.start_chapter,
                    final_attributes: &classified.final_attributes,
                    behaviors: &classified.behaviors,
                    deletion_candidates: &staged.extraction.deletion_candidates,
                },
                prompt_renderer,
            )
            .await?;

        report_progress(
            progress,
            8,
            &format!(
                "{}：正在写入角色事件、状态索引和快照",
                prepared.profile.name
            ),
        )?;
        let pending_attribute_events = pending_character_sync_attribute_events(
            &prepared.baseline_state,
            &staged.extraction.changes,
            &classified.classification_suggestions,
        )
        .map_err(AppError::InvalidArgument)?;
        let write_steps =
            build_character_sync_write_step_records(BuildCharacterSyncWriteStepsInput {
                profile_id: prepared.profile.id,
                chapter_numbers: &prepared.selected_chapter_numbers,
                attribute_events: &pending_attribute_events,
                behaviors: &classified.behaviors,
                deletion_candidates: &staged.extraction.deletion_candidates,
                implicit_review_decisions: &implicit_review_decisions,
                target_chapter: prepared.target_chapter,
            })?;
        let implicit_review_decisions_json = serde_json::to_value(&implicit_review_decisions)?;
        let generation_run =
            character_profile_sync_generation_run(&prepared, &staged.generation_trace);
        let selected_chapter_count = prepared.selected_chapter_numbers.len();
        let applied = self
            .apply_character_sync_persistence_record(CharacterSyncPersistenceInput {
                project_id: prepared.project_id,
                profile_id: prepared.profile.id,
                profile_name: prepared.profile.name.clone(),
                start_chapter: prepared.start_chapter,
                target_chapter: prepared.target_chapter,
                clear_to_chapter: Some(prepared.clear_to_chapter),
                selected_chapter_numbers: prepared.selected_chapter_numbers.clone(),
                baseline_state: prepared.baseline_state.clone(),
                write_steps,
                behaviors: classified.behaviors,
                classification_suggestions: classified.classification_suggestions,
                deletion_candidates: staged.extraction.deletion_candidates,
                chapter_states: chapter_state_record_inputs(&staged.extraction.chapter_states),
                implicit_review_decisions: implicit_review_decisions_json,
                generation_run,
            })
            .await?;

        report_progress(
            progress,
            9,
            &format!(
                "{}：角色数据已写入，正在同步角色 RAG",
                prepared.profile.name
            ),
        )?;
        let rag_summary = self
            .sync_character_continuity_rag(CharacterSyncRagInput {
                project_id: prepared.project_id,
                profile_id: prepared.profile.id,
                character_name: &prepared.profile.name,
                start_chapter: prepared.start_chapter,
                clear_to_chapter: prepared.clear_to_chapter,
                applied: &applied,
            })
            .await;
        report_progress(
            progress,
            CHARACTER_PROFILE_SYNC_PROGRESS_TOTAL,
            &format!("{}：角色同步完成", prepared.profile.name),
        )?;
        Ok(SyncCharacterProfileResult::Synced(
            character_profile_sync_summary(
                &prepared,
                selected_chapter_count,
                implicit_review_decisions.len(),
                applied,
                rag_summary,
                staged.llm_trace_paths,
            ),
        ))
    }
}

fn profile_sync_skip_reason_for_mode(
    profile: &a4n_core::models::character::protagonist_profile::Model,
    target_chapter: i32,
    range_mode: CharacterSyncRangeMode,
) -> Option<CharacterProfileSyncSkipReason> {
    match range_mode {
        CharacterSyncRangeMode::Incremental => profile_sync_skip_reason(profile, target_chapter),
        CharacterSyncRangeMode::AnalysisResyncFromTarget => {
            analysis_profile_sync_skip_reason(profile, target_chapter)
        }
    }
}

fn report_progress(
    progress: Option<CharacterProfileSyncProgress<'_>>,
    current: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress.report(current, message)?;
    }
    Ok(())
}

fn chapter_state_record_inputs(
    chapter_states: &[CharacterProfileSyncChapterStateDraft],
) -> Vec<CharacterSyncChapterStateRecordInput> {
    chapter_states
        .iter()
        .map(|state| CharacterSyncChapterStateRecordInput {
            chapter_number: state.chapter_number,
            state_summary: state.state_summary.clone(),
            current_location: state.current_location.clone(),
            current_goal: state.current_goal.clone(),
            emotional_state: state.emotional_state.clone(),
        })
        .collect()
}

fn character_profile_sync_generation_run(
    prepared: &PreparedCharacterProfileSync,
    trace: &super::super::CharacterProfileSyncGenerationTrace,
) -> RecordCompletedGenerationRunInput {
    RecordCompletedGenerationRunInput {
        project_id: prepared.project_id,
        workflow: "character_profile_sync".to_string(),
        step_key: "sync_character_profile_from_chapters".to_string(),
        title: format!("角色档案同步: {}", prepared.profile.name),
        model_kind: "llm".to_string(),
        provider: None,
        model: None,
        prompt_template_names_json: Some(trace.prompt_template_names_json.clone()),
        prompt_template_hashes_json: None,
        rendered_messages_json: Some(trace.rendered_messages_json.clone()),
        rendered_prompts_json: None,
        input_variables_json: Some(json!({
            "profile_id": prepared.profile.id,
            "character_name": &prepared.profile.name,
            "last_synced_chapter_before_run": prepared.profile.last_synced_chapter,
            "start_chapter": prepared.start_chapter,
            "target_chapter": prepared.target_chapter,
            "clear_to_chapter": prepared.clear_to_chapter,
            "selected_chapter_numbers": &prepared.selected_chapter_numbers,
            "baseline_state": &prepared.baseline_state,
            "chapter_data": &prepared.chapter_data,
        })),
        source_refs_json: Some(json!({
            "profile": {
                "id": prepared.profile.id,
                "name": &prepared.profile.name,
            },
            "chapters": prepared
                .selected_chapter_numbers
                .iter()
                .map(|chapter_number| json!({ "chapter_number": chapter_number }))
                .collect::<Vec<_>>(),
        })),
        media_refs_json: None,
        request_json: Some(trace.request_json.clone()),
        response_summary_json: Some(json!({
            "profile_id": prepared.profile.id,
            "character_name": &prepared.profile.name,
            "response_chars": trace.response_chars,
        })),
        targets: vec![],
    }
}

fn character_profile_sync_summary(
    prepared: &PreparedCharacterProfileSync,
    selected_chapter_count: usize,
    implicit_reviews_checked: usize,
    applied: AppliedCharacterSyncPersistenceRecord,
    rag: CharacterSyncRagSummary,
    llm_trace_paths: Vec<String>,
) -> CharacterProfileSyncedResult {
    CharacterProfileSyncedResult {
        name: prepared.profile.name.clone(),
        chapters_analyzed: selected_chapter_count,
        sync_range: CharacterProfileSyncRange {
            from: prepared.start_chapter,
            to: prepared.target_chapter,
        },
        synced_chapter: prepared.target_chapter,
        changes_applied: applied.write_summary.changes_applied,
        behaviors_recorded: applied.write_summary.behaviors_recorded,
        classification_suggestions_applied: applied
            .write_summary
            .classification_suggestions_applied,
        deletions_marked: applied.write_summary.deletions_marked,
        deletions_executed: applied.write_summary.deletions_executed,
        deleted_deletion_marks: applied.deletion_mark_rewind.deleted,
        rewound_deletion_marks: applied.deletion_mark_rewind.rewound,
        implicit_reviews_checked,
        implicit_reviews_applied: applied.write_summary.implicit_reviews_applied,
        snapshot_hash: applied.target_snapshot_hash,
        state_indexes_upserted: applied.state_indexes.len(),
        deleted_state_rag: rag.deleted_state_rag,
        state_rag_indexed_count: rag.state_rag_indexed_count,
        state_rag_indexed: rag.state_rag_indexed,
        state_rag_error: rag.state_rag_error,
        deleted_attribute_event_rag: rag.deleted_attribute_event_rag,
        attribute_event_rag_indexed_count: rag.attribute_event_rag_indexed_count,
        attribute_event_rag_indexed: rag.attribute_event_rag_indexed,
        attribute_event_rag_error: rag.attribute_event_rag_error,
        llm_trace_paths,
    }
}
