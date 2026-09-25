//! Pure chapter-content projection and workflow identity rules.
//!
//! Database access stays in the worker/Host adapter. Keeping the projection
//! here prevents the chapter read worker and the in-process workflow owner
//! from inventing different summary semantics.

use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_api_contract::chapter::{
    ChapterMaterialSyncState, ChapterMaterialSyncStatus, ChapterMaterialUpdate,
    ChapterMaterialUpdateAction, ChapterMaterialUpdateKind, ChapterSummary,
};
use a4n_db_models::models::{
    chapter::chapter,
    character::protagonist_profile,
    generation::{workflow_checkpoint, workflow_run},
};
use a4n_domain::source_text_hash;
use a4n_domain::workflow::WORKFLOW_MODE_DERIVED_REPLAY;
use a4n_error::AppError;
use serde_json::{Value, json};

mod cleanup;
mod sync;

pub use cleanup::cleanup_chapter_derived_data_in;
pub use sync::{
    CHAPTER_CONTENT_SYNC_CLEANUP_STEP, CHAPTER_CONTENT_SYNC_EXTERNAL_STEP,
    begin_chapter_content_sync_in_transaction, chapter_content_sync_checkpoint_artifacts,
    chapter_content_sync_checkpoint_input, chapter_content_sync_workflow_business_key,
    chapter_content_sync_workflow_run_input,
};

pub const CHAPTER_CONTENT_SYNC_WORKFLOW_KIND: &str = "chapter_content_sync";
pub const CHAPTER_CONTENT_SYNC_RAG_STEP: &str = "chapter_content_sync.current_rag";
pub const CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP: &str = "chapter_analysis.replacement_commit";
pub const CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP: &str = "chapter_analysis.replacement_cleanup";
pub const CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS: &str = "chapter_analysis.sync_outputs";
pub const CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC: &str = "chapter_analysis.outline_sync";
pub const CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY: &str = "chapter_analysis.object_discovery";
pub const CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY: &str = "chapter_analysis.character_discovery";
pub const CHAPTER_ANALYSIS_STEP_SCENE_SYNC: &str = "chapter_analysis.scene_sync";
pub const CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC: &str = "chapter_analysis.character_sync";

pub const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";
pub const WORKFLOW_STATUS_COMPLETED: &str = "completed";
pub const WORKFLOW_STATUS_FAILED: &str = "failed";
pub const WORKFLOW_STATUS_INTERRUPTED: &str = "interrupted";
pub const WORKFLOW_STATUS_RUNNING: &str = "running";

pub fn chapter_summary(chapter: &chapter::Model) -> ChapterSummary {
    let initial_material_status = if chapter.analysis_data.is_some() {
        ChapterMaterialSyncStatus::Unknown
    } else {
        ChapterMaterialSyncStatus::NotAnalyzed
    };
    ChapterSummary {
        id: chapter.id,
        project_id: chapter.project_id,
        outline_id: chapter.outline_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        content: chapter.content.clone(),
        edit_revision: a4n_domain::chapter::chapter_edit_revision(
            &chapter.content,
            chapter.title.as_deref(),
        ),
        word_count: chapter.word_count,
        status: chapter.status.clone(),
        summary: chapter.summary.clone(),
        style_tags: chapter.style_tags.clone(),
        analysis_data: chapter.analysis_data.clone(),
        analysis_sync_status: None,
        analysis_sync_can_resume: false,
        analysis_sync_has_saved_analysis: false,
        analysis_sync_resume_block_reason: None,
        analysis_sync_error: None,
        derived_sync_status: None,
        derived_sync_workflow_run_id: None,
        derived_sync_can_resume: false,
        derived_sync_error: None,
        material_sync: material_sync_state(initial_material_status),
        material_updates: Vec::new(),
        created_at: Some(chapter.created_at.to_string()),
        updated_at: Some(chapter.updated_at.to_string()),
    }
}

pub fn chapter_analysis_workflow_fingerprint(chapter: &chapter::Model) -> Value {
    json!({
        "schema_version": 1,
        "workflow_kind": "chapter_analysis",
        "project_id": chapter.project_id,
        "chapter_id": chapter.id,
        "chapter_number": chapter.chapter_number,
        "content_sha256": source_text_hash(&chapter.content),
        "title": chapter.title,
        "updated_at": chapter.updated_at,
    })
}

pub fn chapter_analysis_run_input_matches(
    run_fingerprint_json: &Option<Value>,
    chapter: &chapter::Model,
) -> bool {
    let Some(fingerprint) = run_fingerprint_json.as_ref() else {
        return false;
    };
    let source = if fingerprint.get("schema_version").and_then(Value::as_i64) == Some(2)
        && fingerprint.get("workflow_kind").and_then(Value::as_str) == Some("chapter_analysis")
        && fingerprint
            .get("analysis_revision")
            .and_then(Value::as_str)
            .is_some_and(|revision| !revision.is_empty())
    {
        fingerprint.get("source_fingerprint")
    } else {
        Some(fingerprint)
    };
    source == Some(&chapter_analysis_workflow_fingerprint(chapter))
}

pub fn chapter_analysis_run_uses_replacement_commit(run_fingerprint_json: &Option<Value>) -> bool {
    run_fingerprint_json.as_ref().is_some_and(|fingerprint| {
        fingerprint.get("schema_version").and_then(Value::as_i64) == Some(2)
            && fingerprint.get("workflow_kind").and_then(Value::as_str) == Some("chapter_analysis")
            && fingerprint
                .get("analysis_revision")
                .and_then(Value::as_str)
                .is_some_and(|revision| !revision.is_empty())
            && fingerprint.get("source_fingerprint").is_some()
    })
}

pub fn chapter_analysis_phase_fingerprint(chapter: &chapter::Model, step_key: &str) -> Value {
    json!({
        "schema_version": 1,
        "workflow": "chapter_analysis.derived_sync",
        "step_key": step_key,
        "chapter": chapter_analysis_workflow_fingerprint(chapter),
        "analysis_data": chapter.analysis_data.as_ref(),
    })
}

pub fn chapter_content_sync_workflow_fingerprint(chapter: &chapter::Model) -> Value {
    json!({
        "schema_version": 1,
        "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
        "project_id": chapter.project_id,
        "chapter_id": chapter.id,
        "chapter_number": chapter.chapter_number,
        "title": chapter.title,
        "content_sha256": source_text_hash(&chapter.content),
    })
}

pub fn chapter_content_sync_run_input_matches(
    run_fingerprint_json: &Option<Value>,
    chapter: &chapter::Model,
) -> bool {
    run_fingerprint_json.as_ref().is_some_and(|fingerprint| {
        fingerprint.get("schema_version").and_then(Value::as_i64) == Some(3)
            && fingerprint.get("workflow_kind").and_then(Value::as_str)
                == Some(CHAPTER_CONTENT_SYNC_WORKFLOW_KIND)
            && fingerprint
                .get("sync_revision")
                .and_then(Value::as_str)
                .is_some_and(|revision| !revision.is_empty())
            && fingerprint.get("source_fingerprint")
                == Some(&chapter_content_sync_workflow_fingerprint(chapter))
    })
}

pub fn chapter_cleanup_requires_snapshot(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    profile.last_synced_chapter >= chapter_number
        && !profile_state_should_restore_base(profile, chapter_number - 1)
}

pub fn validate_chapter_content_snapshot_preflight(
    profiles: &[protagonist_profile::Model],
    existing_snapshot_profile_ids: &BTreeSet<i64>,
    chapter_number: i32,
) -> Result<(), AppError> {
    let snapshot_chapter = chapter_number - 1;
    let missing_names = profiles
        .iter()
        .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
        .filter(|profile| !existing_snapshot_profile_ids.contains(&profile.id))
        .map(|profile| profile.name.clone())
        .collect::<Vec<_>>();
    if missing_names.is_empty() {
        return Ok(());
    }

    Err(AppError::InvalidState {
        current: "章节派生数据无法清理".to_string(),
        operation: format!(
            "保存章节正文前预检：以下角色缺少第 {snapshot_chapter} 章快照，无法在重写第 {chapter_number} 章前回滚档案状态：{}",
            missing_names.join("、")
        ),
    })
}

fn profile_state_should_restore_base(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    chapter_number <= 0
        || profile
            .first_seen_chapter()
            .is_some_and(|first_seen| chapter_number < first_seen)
}

pub fn chapter_analysis_run_has_saved_analysis_from_checkpoints(
    run: &workflow_run::Model,
    chapter: &chapter::Model,
    checkpoints: &[workflow_checkpoint::Model],
) -> Result<bool, AppError> {
    if run.workflow_kind != "chapter_analysis"
        || run.project_id != Some(chapter.project_id)
        || run.chapter_id != Some(chapter.id)
        || !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter)
    {
        return Ok(false);
    }
    if chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json)
        && run.mode.as_deref() != Some(WORKFLOW_MODE_DERIVED_REPLAY)
    {
        let fingerprint =
            chapter_analysis_phase_fingerprint(chapter, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP);
        let fingerprint_sha256 =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&fingerprint)
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let checkpoint = checkpoints.iter().find(|checkpoint| {
            checkpoint.step_key == CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP
                && checkpoint.status == WORKFLOW_STATUS_COMPLETED
                && checkpoint.input_fingerprint_sha256.as_deref() == Some(&fingerprint_sha256)
        });
        let Some(checkpoint) = checkpoint else {
            return Ok(false);
        };
        let Some(checkpoint_json) = checkpoint.checkpoint_json.as_ref() else {
            return Ok(false);
        };
        if checkpoint_json.get("completed").and_then(Value::as_bool) != Some(true)
            || checkpoint_json.get("cleanup_artifacts").is_none()
        {
            return Ok(false);
        }
    }
    let Some(analysis) = chapter.analysis_data.as_ref() else {
        return Ok(false);
    };
    a4n_chapter_analysis::validate_saved_chapter_analysis(analysis).map_err(|error| {
        AppError::InvalidState {
            current: format!("invalid_saved_chapter_analysis: {error}"),
            operation: "读取章节分析工作流保存状态".to_string(),
        }
    })?;
    Ok(true)
}

pub fn attach_chapter_sync_state(
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
    analysis_runs: &[workflow_run::Model],
    content_runs: &[workflow_run::Model],
    checkpoints: &[workflow_checkpoint::Model],
) -> Result<(), AppError> {
    let mut checkpoints_by_run = HashMap::<i64, Vec<workflow_checkpoint::Model>>::new();
    for checkpoint in checkpoints {
        checkpoints_by_run
            .entry(checkpoint.workflow_run_id)
            .or_default()
            .push(checkpoint.clone());
    }
    attach_chapter_content_sync_state(chapters, summaries, content_runs, &checkpoints_by_run)?;

    let mut latest_by_chapter = HashMap::new();
    for run in analysis_runs {
        let Some(chapter_id) = run.chapter_id else {
            continue;
        };
        latest_by_chapter.entry(chapter_id).or_insert(run);
    }
    for (chapter, summary) in chapters.iter().zip(summaries.iter_mut()) {
        let Some(run) = latest_by_chapter.get(&chapter.id) else {
            continue;
        };
        if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter) {
            summary.material_sync.outline_and_foreshadowing =
                ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.objects = ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.characters = ChapterMaterialSyncStatus::NeedsAnalysis;
            summary.material_sync.scenes = ChapterMaterialSyncStatus::NeedsAnalysis;
            continue;
        }
        let run_checkpoints = checkpoints_by_run
            .get(&run.id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let has_saved_analysis = chapter_analysis_run_has_saved_analysis_from_checkpoints(
            run,
            chapter,
            run_checkpoints,
        )?;
        summary.analysis_sync_status = Some(run.status.clone());
        summary.analysis_sync_can_resume = run.can_resume;
        summary.analysis_sync_has_saved_analysis = has_saved_analysis;
        summary.analysis_sync_resume_block_reason = run.resume_block_reason.clone();
        summary.analysis_sync_error = run.error_message.clone();
        let (material_sync, material_updates) =
            chapter_material_projection(run, run_checkpoints, has_saved_analysis)?;
        summary.material_sync = material_sync;
        summary.material_updates = material_updates;
    }
    Ok(())
}

fn material_sync_state(status: ChapterMaterialSyncStatus) -> ChapterMaterialSyncState {
    ChapterMaterialSyncState {
        content_index: status,
        outline_and_foreshadowing: status,
        objects: status,
        characters: status,
        scenes: status,
    }
}

fn attach_chapter_content_sync_state(
    chapters: &[chapter::Model],
    summaries: &mut [ChapterSummary],
    runs: &[workflow_run::Model],
    checkpoints_by_run: &HashMap<i64, Vec<workflow_checkpoint::Model>>,
) -> Result<(), AppError> {
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
        let checkpoints = checkpoints_by_run
            .get(&run.id)
            .map(Vec::as_slice)
            .unwrap_or_default();
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
        return Ok((material_sync_state(status), Vec::new()));
    }

    let checkpoint_by_step = checkpoints
        .iter()
        .map(|checkpoint| (checkpoint.step_key.as_str(), checkpoint))
        .collect::<HashMap<_, _>>();
    let prerequisites_failed = run.current_step_key.as_deref()
        == Some(CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP)
        && workflow_failed(&run.status);
    let material_sync = if prerequisites_failed {
        material_sync_state(ChapterMaterialSyncStatus::Failed)
    } else {
        ChapterMaterialSyncState {
            content_index: phase_status(
                run,
                &checkpoint_by_step,
                &[a4n_domain::fact_feedback::CHAPTER_FACT_INDEX_STEP],
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
    Ok((material_sync, material_updates(&checkpoint_by_step)?))
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
    for (step, field, kind, action) in [
        (
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
            "created",
            ChapterMaterialUpdateKind::Object,
            ChapterMaterialUpdateAction::Created,
        ),
        (
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
            "updated",
            ChapterMaterialUpdateKind::Object,
            ChapterMaterialUpdateAction::Updated,
        ),
        (
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            "created",
            ChapterMaterialUpdateKind::Character,
            ChapterMaterialUpdateAction::Created,
        ),
        (
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            "existing",
            ChapterMaterialUpdateKind::Character,
            ChapterMaterialUpdateAction::Existing,
        ),
        (
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
            "synced",
            ChapterMaterialUpdateKind::Character,
            ChapterMaterialUpdateAction::Synced,
        ),
        (
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
            "preserved",
            ChapterMaterialUpdateKind::Character,
            ChapterMaterialUpdateAction::Preserved,
        ),
        (
            CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
            "profiles",
            ChapterMaterialUpdateKind::Scene,
            ChapterMaterialUpdateAction::Synced,
        ),
    ] {
        append_named_updates(
            &mut updates,
            &mut seen,
            checkpoint_result(checkpoints, step),
            field,
            kind,
            action,
        )?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::BTreeSet;

    #[test]
    fn chapter_projection_defaults_to_not_analyzed() {
        let now = Utc::now().naive_utc();
        let chapter = chapter::Model {
            id: 1,
            project_id: 2,
            outline_id: None,
            chapter_number: 1,
            title: Some("第一章".into()),
            content: "正文".into(),
            word_count: 2,
            status: "draft".into(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(
            chapter_summary(&chapter).material_sync.content_index,
            ChapterMaterialSyncStatus::NotAnalyzed
        );
    }

    fn profile(
        name: &str,
        last_synced_chapter: i32,
        first_seen_chapter: i32,
    ) -> protagonist_profile::Model {
        protagonist_profile::Model {
            id: 7,
            project_id: 2,
            name: name.to_string(),
            gender: None,
            age: None,
            base_attributes: json!({
                "explicit_attributes": {"first_seen_chapter": first_seen_chapter}
            }),
            current_state: json!({}),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn chapter_content_preflight_requires_existing_snapshot_for_rewind() {
        let profiles = vec![profile("林夏", 3, 1)];
        let error = validate_chapter_content_snapshot_preflight(&profiles, &BTreeSet::new(), 3)
            .expect_err("rewriting a synced chapter requires its previous snapshot");
        assert!(error.to_string().contains("林夏"));
        assert!(error.to_string().contains("第 2 章快照"));

        validate_chapter_content_snapshot_preflight(&profiles, &BTreeSet::from([7]), 3)
            .expect("an existing snapshot clears the preflight");
    }

    #[test]
    fn chapter_content_preflight_skips_profiles_first_seen_at_target_chapter() {
        validate_chapter_content_snapshot_preflight(
            &[profile("新角色", 3, 3)],
            &BTreeSet::new(),
            3,
        )
        .expect("a profile first discovered at the target chapter is deleted, not rewound");
    }

    #[test]
    fn derived_replay_is_visible_as_resumable_pending_material() {
        let now = Utc::now().naive_utc();
        let chapter = chapter::Model {
            id: 11,
            project_id: 2,
            outline_id: None,
            chapter_number: 3,
            title: Some("第三章".into()),
            content: "正文".into(),
            word_count: 2,
            status: "draft".into(),
            summary: None,
            style_tags: None,
            analysis_data: Some(json!({
                "metadata": {"characters": [], "locations": [], "items": [], "tags": [], "tone": null, "timeline_marker": null},
                "summaries": {"compressed": "摘要", "one_line": "摘要", "keywords": []},
                "character_states": {},
                "foreshadowing": {"planted": [], "resolved": [], "tensions": []},
                "key_events": []
            })),
            created_at: now,
            updated_at: now,
        };
        let source_fingerprint = chapter_analysis_workflow_fingerprint(&chapter);
        let run = workflow_run::Model {
            id: 7,
            business_key: "chapter_analysis:2:11".into(),
            workflow_kind: "chapter_analysis".into(),
            project_id: Some(2),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".into()),
            scope_id: Some(11),
            chapter_id: Some(11),
            mode: Some(WORKFLOW_MODE_DERIVED_REPLAY.into()),
            input_fingerprint_json: Some(json!({
                "schema_version": 2,
                "workflow_kind": "chapter_analysis",
                "analysis_revision": "replay-1",
                "source_fingerprint": source_fingerprint,
            })),
            status: WORKFLOW_STATUS_INTERRUPTED.into(),
            current_step_key: Some("chapter_analysis.derived_sync".into()),
            progress_json: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: Some("正文变化后等待重新同步章节派生资料".into()),
            created_at: now,
            updated_at: now,
            completed_at: Some(now),
        };
        let mut summary = chapter_summary(&chapter);
        attach_chapter_sync_state(
            std::slice::from_ref(&chapter),
            std::slice::from_mut(&mut summary),
            std::slice::from_ref(&run),
            &[],
            &[],
        )
        .expect("replay status should project");

        assert_eq!(
            summary.analysis_sync_status.as_deref(),
            Some(WORKFLOW_STATUS_INTERRUPTED)
        );
        assert!(summary.analysis_sync_can_resume);
        assert!(summary.analysis_sync_has_saved_analysis);
        assert_eq!(
            summary.material_sync.objects,
            ChapterMaterialSyncStatus::Pending
        );
        assert_eq!(
            summary.material_sync.scenes,
            ChapterMaterialSyncStatus::Pending
        );
    }
}
