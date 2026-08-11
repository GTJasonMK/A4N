use super::{
    ChapterMaterialSyncState, ChapterMaterialSyncStatus, ChapterMaterialUpdate,
    ChapterMaterialUpdateAction, ChapterMaterialUpdateKind, ChapterSummary,
};
use crate::AppState;
use a4n_core::{
    error::AppError,
    models::{
        chapter::chapter,
        generation::{workflow_checkpoint, workflow_run},
    },
};
use a4n_orchestrator::{
    chapter_analysis::{
        CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
        CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
        CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC, CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
        CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
    },
    chapter_content::{
        CHAPTER_CONTENT_SYNC_RAG_STEP, CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
        chapter_content_sync_run_input_matches,
    },
    workflow::{
        WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
        WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING,
    },
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn chapter_summary(ch: &chapter::Model) -> ChapterSummary {
    let initial_material_status = if ch.analysis_data.is_some() {
        ChapterMaterialSyncStatus::Unknown
    } else {
        ChapterMaterialSyncStatus::NotAnalyzed
    };
    ChapterSummary {
        id: ch.id,
        project_id: ch.project_id,
        outline_id: ch.outline_id,
        chapter_number: ch.chapter_number,
        title: ch.title.clone(),
        content: ch.content.clone(),
        word_count: ch.word_count,
        status: ch.status.clone(),
        summary: ch.summary.clone(),
        style_tags: ch.style_tags.clone(),
        analysis_data: ch.analysis_data.clone(),
        analysis_sync_status: None,
        analysis_sync_can_resume: false,
        analysis_sync_has_saved_analysis: false,
        analysis_sync_resume_block_reason: None,
        analysis_sync_error: None,
        derived_sync_status: None,
        derived_sync_workflow_run_id: None,
        derived_sync_can_resume: false,
        derived_sync_error: None,
        material_sync: ChapterMaterialSyncState::all(initial_material_status),
        material_updates: Vec::new(),
        created_at: Some(ch.created_at.to_string()),
        updated_at: Some(ch.updated_at.to_string()),
    }
}

pub(super) async fn attach_chapter_sync_state(
    state: &AppState,
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
) -> Result<(), AppError> {
    let Some(project_id) = chapters.first().map(|chapter| chapter.project_id) else {
        return Ok(());
    };
    attach_chapter_content_sync_state(state, project_id, chapters, summaries).await?;
    let runs = state
        .orchestrator()
        .workflow_runs_by_project_kind(project_id, "chapter_analysis")
        .await?;
    let mut latest_by_chapter = HashMap::new();
    for run in runs {
        let Some(chapter_id) = run.chapter_id else {
            continue;
        };
        latest_by_chapter.entry(chapter_id).or_insert(run);
    }
    for (chapter, summary) in chapters.iter().zip(summaries.iter_mut()) {
        let Some(run) = latest_by_chapter.get(&chapter.id) else {
            continue;
        };
        if !a4n_orchestrator::chapter_analysis::chapter_analysis_run_input_matches(
            &run.input_fingerprint_json,
            chapter,
        ) {
            summary.material_sync.outline_and_foreshadowing =
                ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.objects = ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.characters = ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.scenes = ChapterMaterialSyncStatus::NeedsAnalysis;
            continue;
        }
        let has_saved_analysis = state
            .orchestrator()
            .chapter_analysis_run_has_saved_analysis(run, chapter)
            .await?;
        let checkpoints = state
            .orchestrator()
            .workflow_checkpoint_models(run.id)
            .await?;

        summary.analysis_sync_status = Some(run.status.clone());
        summary.analysis_sync_can_resume = run.can_resume;
        summary.analysis_sync_has_saved_analysis = has_saved_analysis;
        summary.analysis_sync_resume_block_reason = run.resume_block_reason.clone();
        summary.analysis_sync_error = run.error_message.clone();
        let (material_sync, material_updates) =
            chapter_material_projection(run, &checkpoints, has_saved_analysis)?;
        summary.material_sync = material_sync;
        summary.material_updates = material_updates;
    }
    Ok(())
}

async fn attach_chapter_content_sync_state(
    state: &AppState,
    project_id: i64,
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
) -> Result<(), AppError> {
    let runs = state
        .orchestrator()
        .workflow_runs_by_project_kind(project_id, CHAPTER_CONTENT_SYNC_WORKFLOW_KIND)
        .await?;
    let mut latest_by_chapter = HashMap::new();
    for run in runs {
        let Some(chapter_id) = run.chapter_id else {
            continue;
        };
        latest_by_chapter.entry(chapter_id).or_insert(run);
    }
    for (chapter, summary) in chapters.iter().zip(summaries.iter_mut()) {
        let Some(run) = latest_by_chapter.get(&chapter.id) else {
            continue;
        };
        if !chapter_content_sync_run_input_matches(&run.input_fingerprint_json, chapter) {
            continue;
        }
        let checkpoints = state
            .orchestrator()
            .workflow_checkpoint_models(run.id)
            .await?;
        let checkpoint_by_step = checkpoints
            .iter()
            .map(|checkpoint| (checkpoint.step_key.as_str(), checkpoint))
            .collect::<HashMap<_, _>>();
        summary.derived_sync_status = Some(run.status.clone());
        summary.derived_sync_workflow_run_id = Some(run.id);
        summary.derived_sync_can_resume = run.can_resume;
        summary.derived_sync_error = run.error_message.clone();
        summary.material_sync.content_index =
            phase_status(run, &checkpoint_by_step, &[CHAPTER_CONTENT_SYNC_RAG_STEP]);
    }
    Ok(())
}

fn chapter_material_projection(
    run: &workflow_run::Model,
    checkpoints: &[workflow_checkpoint::Model],
    has_saved_analysis: bool,
) -> Result<(ChapterMaterialSyncState, Vec<ChapterMaterialUpdate>), AppError> {
    if !has_saved_analysis {
        let status = if run.status == WORKFLOW_STATUS_RUNNING {
            ChapterMaterialSyncStatus::Running
        } else if workflow_failed(&run.status) {
            ChapterMaterialSyncStatus::Failed
        } else if run.status == WORKFLOW_STATUS_COMPLETED {
            ChapterMaterialSyncStatus::Unknown
        } else {
            ChapterMaterialSyncStatus::NotAnalyzed
        };
        return Ok((ChapterMaterialSyncState::all(status), Vec::new()));
    }

    let checkpoint_by_step = checkpoints
        .iter()
        .map(|checkpoint| (checkpoint.step_key.as_str(), checkpoint))
        .collect::<HashMap<_, _>>();
    let prerequisites_failed = run.current_step_key.as_deref()
        == Some(CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP)
        && workflow_failed(&run.status);
    let material_sync = if prerequisites_failed {
        ChapterMaterialSyncState::all(ChapterMaterialSyncStatus::Failed)
    } else {
        ChapterMaterialSyncState {
            content_index: phase_status(
                run,
                &checkpoint_by_step,
                &[CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS],
            ),
            outline_and_foreshadowing: phase_status(
                run,
                &checkpoint_by_step,
                &[
                    CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC,
                    CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
                ],
            ),
            objects: phase_status(
                run,
                &checkpoint_by_step,
                &[CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY],
            ),
            characters: phase_status(
                run,
                &checkpoint_by_step,
                &[
                    CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
                    CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
                ],
            ),
            scenes: phase_status(
                run,
                &checkpoint_by_step,
                &[CHAPTER_ANALYSIS_STEP_SCENE_SYNC],
            ),
        }
    };
    let material_updates = material_updates(&checkpoint_by_step)?;
    Ok((material_sync, material_updates))
}

fn phase_status(
    run: &workflow_run::Model,
    checkpoints: &HashMap<&str, &workflow_checkpoint::Model>,
    step_keys: &[&str],
) -> ChapterMaterialSyncStatus {
    if step_keys
        .iter()
        .all(|step_key| completed_checkpoint(checkpoints.get(step_key).copied()))
    {
        return ChapterMaterialSyncStatus::Completed;
    }

    if run
        .current_step_key
        .as_deref()
        .is_some_and(|current| step_keys.contains(&current))
    {
        return if run.status == WORKFLOW_STATUS_RUNNING {
            ChapterMaterialSyncStatus::Running
        } else if workflow_failed(&run.status) {
            ChapterMaterialSyncStatus::Failed
        } else {
            ChapterMaterialSyncStatus::Unknown
        };
    }

    match run.status.as_str() {
        WORKFLOW_STATUS_RUNNING
        | WORKFLOW_STATUS_FAILED
        | WORKFLOW_STATUS_CANCELLED
        | WORKFLOW_STATUS_INTERRUPTED => ChapterMaterialSyncStatus::Pending,
        WORKFLOW_STATUS_COMPLETED => ChapterMaterialSyncStatus::Unknown,
        _ => ChapterMaterialSyncStatus::Unknown,
    }
}

fn completed_checkpoint(checkpoint: Option<&workflow_checkpoint::Model>) -> bool {
    checkpoint.is_some_and(|checkpoint| {
        checkpoint.status == WORKFLOW_STATUS_COMPLETED
            && checkpoint
                .checkpoint_json
                .as_ref()
                .and_then(|value| value.get("completed"))
                .and_then(Value::as_bool)
                == Some(true)
    })
}

fn workflow_failed(status: &str) -> bool {
    matches!(
        status,
        WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
    )
}

fn material_updates(
    checkpoints: &HashMap<&str, &workflow_checkpoint::Model>,
) -> Result<Vec<ChapterMaterialUpdate>, AppError> {
    let mut updates = Vec::new();
    let mut seen = HashSet::new();
    for (field, action) in [
        ("created", ChapterMaterialUpdateAction::Created),
        ("updated", ChapterMaterialUpdateAction::Updated),
        ("existing", ChapterMaterialUpdateAction::Existing),
        ("preserved", ChapterMaterialUpdateAction::Preserved),
    ] {
        append_named_updates(
            &mut updates,
            &mut seen,
            checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC),
            field,
            ChapterMaterialUpdateKind::Outline,
            action,
        )?;
    }
    for (field, action) in [
        ("created", ChapterMaterialUpdateAction::Created),
        ("updated", ChapterMaterialUpdateAction::Updated),
        ("existing", ChapterMaterialUpdateAction::Existing),
        ("resolved", ChapterMaterialUpdateAction::Resolved),
    ] {
        append_named_updates(
            &mut updates,
            &mut seen,
            checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS)
                .and_then(|result| result.get("foreshadowing")),
            field,
            ChapterMaterialUpdateKind::Foreshadowing,
            action,
        )?;
    }
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY),
        "created",
        ChapterMaterialUpdateKind::Object,
        ChapterMaterialUpdateAction::Created,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY),
        "updated",
        ChapterMaterialUpdateKind::Object,
        ChapterMaterialUpdateAction::Updated,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY),
        "created",
        ChapterMaterialUpdateKind::Character,
        ChapterMaterialUpdateAction::Created,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY),
        "existing",
        ChapterMaterialUpdateKind::Character,
        ChapterMaterialUpdateAction::Existing,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC),
        "synced",
        ChapterMaterialUpdateKind::Character,
        ChapterMaterialUpdateAction::Synced,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC),
        "preserved",
        ChapterMaterialUpdateKind::Character,
        ChapterMaterialUpdateAction::Preserved,
    )?;
    append_named_updates(
        &mut updates,
        &mut seen,
        checkpoint_result(checkpoints, CHAPTER_ANALYSIS_STEP_SCENE_SYNC),
        "profiles",
        ChapterMaterialUpdateKind::Scene,
        ChapterMaterialUpdateAction::Synced,
    )?;
    Ok(updates)
}

fn checkpoint_result<'a>(
    checkpoints: &'a HashMap<&str, &workflow_checkpoint::Model>,
    step_key: &str,
) -> Option<&'a Value> {
    checkpoints
        .get(step_key)
        .copied()
        .filter(|checkpoint| completed_checkpoint(Some(checkpoint)))
        .and_then(|checkpoint| checkpoint.checkpoint_json.as_ref())
        .and_then(|checkpoint| checkpoint.get("result"))
}

fn append_named_updates(
    updates: &mut Vec<ChapterMaterialUpdate>,
    seen: &mut HashSet<(
        ChapterMaterialUpdateKind,
        ChapterMaterialUpdateAction,
        String,
    )>,
    result: Option<&Value>,
    field: &str,
    material_kind: ChapterMaterialUpdateKind,
    action: ChapterMaterialUpdateAction,
) -> Result<(), AppError> {
    let Some(value) = result.and_then(|result| result.get(field)) else {
        return Ok(());
    };
    let rows = value.as_array().ok_or_else(|| AppError::InvalidState {
        current: "chapter_analysis_checkpoint".to_string(),
        operation: format!("章节分析资料更新字段 result.{field} 必须是数组"),
    })?;
    for row in rows {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| AppError::InvalidState {
                current: "chapter_analysis_checkpoint".to_string(),
                operation: format!("章节分析资料更新字段 result.{field}[].name 无效"),
            })?
            .to_string();
        if seen.insert((material_kind, action, name.clone())) {
            updates.push(ChapterMaterialUpdate {
                material_kind,
                action,
                name,
            });
        }
    }
    Ok(())
}

pub async fn chapter_summary_with_sync_state(
    state: &AppState,
    chapter: &chapter::Model,
) -> Result<ChapterSummary, AppError> {
    let mut summaries = vec![chapter_summary(chapter)];
    attach_chapter_sync_state(state, std::slice::from_ref(chapter), &mut summaries).await?;
    Ok(summaries.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn run(status: &str, current_step_key: &str) -> workflow_run::Model {
        let now = Utc::now().naive_utc();
        workflow_run::Model {
            id: 9,
            business_key: "chapter_analysis:3:7".to_string(),
            workflow_kind: "chapter_analysis".to_string(),
            project_id: Some(3),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(7),
            chapter_id: Some(7),
            mode: None,
            input_fingerprint_json: None,
            status: status.to_string(),
            current_step_key: Some(current_step_key.to_string()),
            progress_json: None,
            active_task_id: None,
            can_resume: status != WORKFLOW_STATUS_COMPLETED,
            resume_block_reason: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            completed_at: (status != WORKFLOW_STATUS_RUNNING).then_some(now),
        }
    }

    fn checkpoint(step_key: &str, result: Value) -> workflow_checkpoint::Model {
        let now = Utc::now().naive_utc();
        workflow_checkpoint::Model {
            id: 1,
            workflow_run_id: 9,
            step_key: step_key.to_string(),
            step_order: 10,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: None,
            checkpoint_json: Some(json!({ "completed": true, "result": result })),
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            completed_at: Some(now),
        }
    }

    #[test]
    fn material_projection_uses_phase_checkpoints_instead_of_run_success_guessing() {
        let checkpoints = vec![
            checkpoint(
                CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC,
                json!({"preserved": [{"name": "第一章"}]}),
            ),
            checkpoint(
                CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
                json!({
                    "foreshadowing": {
                        "created": [{"name": "破损怀表"}],
                        "updated": [{"name": "钟楼来历"}],
                        "existing": [{"name": "雨夜脚印"}],
                        "resolved": [{"name": "匿名来信"}]
                    }
                }),
            ),
            checkpoint(
                CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
                json!({
                    "created": [{"name": "旧钥匙"}],
                    "updated": [{"name": "怀表"}]
                }),
            ),
            checkpoint(
                CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
                json!({
                    "created": [{"name": "林夏"}],
                    "existing": [{"name": "沈默"}]
                }),
            ),
            checkpoint(
                CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
                json!({"profiles": [{"name": "废弃钟楼"}]}),
            ),
            checkpoint(
                CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
                json!({
                    "synced": [{"name": "林夏"}],
                    "preserved": [{"name": "沈默"}]
                }),
            ),
        ];

        let (state, updates) = chapter_material_projection(
            &run(
                WORKFLOW_STATUS_COMPLETED,
                CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
            ),
            &checkpoints,
            true,
        )
        .expect("valid checkpoints should project");

        assert_eq!(
            state,
            ChapterMaterialSyncState::all(ChapterMaterialSyncStatus::Completed)
        );
        assert_eq!(updates.len(), 12);
        assert_eq!(updates[0].name, "第一章");
        assert_eq!(updates[1].name, "破损怀表");
        assert_eq!(updates[4].name, "匿名来信");
        assert_eq!(updates[11].name, "废弃钟楼");
    }

    #[test]
    fn failed_current_phase_does_not_claim_future_materials_completed() {
        let checkpoints = vec![
            checkpoint(CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC, json!({})),
            checkpoint(CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS, json!({})),
            checkpoint(CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY, json!({})),
            checkpoint(CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY, json!({})),
        ];

        let (state, _) = chapter_material_projection(
            &run(WORKFLOW_STATUS_FAILED, CHAPTER_ANALYSIS_STEP_SCENE_SYNC),
            &checkpoints,
            true,
        )
        .expect("valid partial checkpoints should project");

        assert_eq!(state.content_index, ChapterMaterialSyncStatus::Completed);
        assert_eq!(state.objects, ChapterMaterialSyncStatus::Completed);
        assert_eq!(state.scenes, ChapterMaterialSyncStatus::Failed);
        assert_eq!(state.characters, ChapterMaterialSyncStatus::Pending);
    }

    #[test]
    fn failure_before_analysis_is_saved_is_visible() {
        let (state, updates) = chapter_material_projection(
            &run(WORKFLOW_STATUS_FAILED, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS),
            &[],
            false,
        )
        .expect("missing analysis is a user-visible state");

        assert_eq!(
            state,
            ChapterMaterialSyncState::all(ChapterMaterialSyncStatus::Failed)
        );
        assert!(updates.is_empty());
    }

    #[test]
    fn running_analysis_is_not_presented_as_stale_content() {
        let (state, _) = chapter_material_projection(
            &run(WORKFLOW_STATUS_RUNNING, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS),
            &[],
            false,
        )
        .expect("running analysis is a user-visible state");

        assert_eq!(
            state,
            ChapterMaterialSyncState::all(ChapterMaterialSyncStatus::Running)
        );
    }
}
