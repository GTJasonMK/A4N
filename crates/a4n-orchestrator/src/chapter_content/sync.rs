use a4n_core::{
    models::{
        chapter::chapter,
        generation::{workflow_checkpoint, workflow_run},
    },
    source_spans::source_text_hash,
};
use a4n_db::workflow::{
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::project::ChapterDerivedCleanupArtifacts;

pub const CHAPTER_CONTENT_SYNC_WORKFLOW_KIND: &str = "chapter_content_sync";
pub(crate) const CHAPTER_CONTENT_SYNC_CLEANUP_STEP: &str = "chapter_content_sync.cleanup_plan";
const CHAPTER_CONTENT_SYNC_EXTERNAL_STEP: &str = "chapter_content_sync.external_cleanup";

pub(crate) fn chapter_content_sync_workflow_business_key(
    project_id: i64,
    chapter_id: i64,
) -> String {
    format!("chapter_content_sync:{project_id}:{chapter_id}")
}

fn chapter_content_sync_workflow_fingerprint(chapter: &chapter::Model) -> Value {
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
        match fingerprint.get("source_fingerprint") {
            Some(source) => {
                let schema_version = fingerprint.get("schema_version").and_then(Value::as_i64);
                matches!(schema_version, Some(2 | 3))
                    && (schema_version == Some(2)
                        || fingerprint
                            .get("sync_current_chapter_rag")
                            .and_then(Value::as_bool)
                            .is_some())
                    && fingerprint.get("workflow_kind").and_then(Value::as_str)
                        == Some(CHAPTER_CONTENT_SYNC_WORKFLOW_KIND)
                    && fingerprint
                        .get("sync_revision")
                        .and_then(Value::as_str)
                        .is_some_and(|revision| !revision.is_empty())
                    && source == &chapter_content_sync_workflow_fingerprint(chapter)
            }
            None => fingerprint == &chapter_content_sync_workflow_fingerprint(chapter),
        }
    })
}

pub(crate) fn chapter_content_sync_should_sync_current_rag(
    run_fingerprint_json: &Option<Value>,
) -> bool {
    chapter_content_sync_current_rag_intent(run_fingerprint_json) == Some(true)
}

pub fn chapter_content_sync_current_rag_intent(
    run_fingerprint_json: &Option<Value>,
) -> Option<bool> {
    let fingerprint = run_fingerprint_json.as_ref()?;
    (fingerprint.get("schema_version").and_then(Value::as_i64) == Some(3))
        .then(|| {
            fingerprint
                .get("sync_current_chapter_rag")
                .and_then(Value::as_bool)
        })
        .flatten()
}

pub(crate) fn chapter_content_sync_invalidates_analysis(run: &workflow_run::Model) -> bool {
    chapter_content_sync_current_rag_intent(&run.input_fingerprint_json) != Some(false)
}

fn chapter_content_sync_run_fingerprint(
    chapter: &chapter::Model,
    sync_current_chapter_rag: bool,
) -> Value {
    json!({
        "schema_version": 3,
        "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
        "sync_revision": Uuid::new_v4(),
        "sync_current_chapter_rag": sync_current_chapter_rag,
        "source_fingerprint": chapter_content_sync_workflow_fingerprint(chapter),
    })
}

pub(crate) fn chapter_content_sync_workflow_run_input(
    chapter: &chapter::Model,
    sync_current_chapter_rag: bool,
) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: chapter_content_sync_workflow_business_key(chapter.project_id, chapter.id),
        workflow_kind: CHAPTER_CONTENT_SYNC_WORKFLOW_KIND.to_string(),
        project_id: Some(chapter.project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter.id),
        chapter_id: Some(chapter.id),
        mode: Some("content_change".to_string()),
        input_fingerprint_json: Some(chapter_content_sync_run_fingerprint(
            chapter,
            sync_current_chapter_rag,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(CHAPTER_CONTENT_SYNC_EXTERNAL_STEP.to_string()),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    }
}

pub(crate) fn chapter_content_sync_checkpoint_input(
    workflow_run_id: i64,
    run_fingerprint_json: Value,
    chapter: &chapter::Model,
    artifacts: &ChapterDerivedCleanupArtifacts,
) -> Result<UpsertWorkflowCheckpointInput, serde_json::Error> {
    Ok(UpsertWorkflowCheckpointInput {
        workflow_run_id,
        step_key: CHAPTER_CONTENT_SYNC_CLEANUP_STEP.to_string(),
        step_order: 10,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        input_fingerprint_sha256: None,
        input_fingerprint_json: Some(run_fingerprint_json),
        checkpoint_json: Some(json!({
            "schema_version": 1,
            "chapter_id": chapter.id,
            "chapter_number": chapter.chapter_number,
        })),
        artifacts_json: Some(serde_json::to_value(artifacts)?),
        generation_run_ids_json: None,
        error_message: None,
        completed: true,
    })
}

pub(crate) fn chapter_content_sync_checkpoint_artifacts(
    run: &workflow_run::Model,
    checkpoint: &workflow_checkpoint::Model,
) -> Result<ChapterDerivedCleanupArtifacts, String> {
    if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
        return Err(format!(
            "章节资料同步 cleanup checkpoint 未完成: {}",
            checkpoint.status
        ));
    }
    if checkpoint.input_fingerprint_json != run.input_fingerprint_json {
        return Err("章节资料同步 cleanup checkpoint 与 workflow 输入不一致".to_string());
    }
    let contract = checkpoint
        .checkpoint_json
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| "章节资料同步 checkpoint_json 合同缺失".to_string())?;
    if contract.get("schema_version").and_then(Value::as_i64) != Some(1)
        || contract.get("chapter_id").and_then(Value::as_i64) != run.chapter_id
    {
        return Err("章节资料同步 checkpoint_json 身份无效".to_string());
    }
    parse_cleanup_artifacts(checkpoint.artifacts_json.as_ref())
}

fn parse_cleanup_artifacts(
    artifacts_json: Option<&Value>,
) -> Result<ChapterDerivedCleanupArtifacts, String> {
    let value = artifacts_json
        .cloned()
        .ok_or_else(|| "章节资料同步 checkpoint 缺少 cleanup artifacts".to_string())?;
    serde_json::from_value(value)
        .map_err(|error| format!("章节资料同步 cleanup artifacts 合同无效: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn chapter() -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::Model {
            id: 7,
            project_id: 3,
            outline_id: None,
            chapter_number: 5,
            title: Some("回声".to_string()),
            content: "正文".to_string(),
            word_count: 2,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn fingerprint_tracks_formal_content_but_not_derived_fields() {
        let mut chapter = chapter();
        let fingerprint = chapter_content_sync_workflow_fingerprint(&chapter);
        assert!(chapter_content_sync_run_input_matches(
            &Some(fingerprint.clone()),
            &chapter
        ));
        chapter.style_tags = Some(json!(["叙事为主"]));
        chapter.updated_at = Utc::now().naive_utc();
        assert_eq!(
            fingerprint,
            chapter_content_sync_workflow_fingerprint(&chapter)
        );

        chapter.content = "新正文".to_string();
        assert_ne!(
            fingerprint,
            chapter_content_sync_workflow_fingerprint(&chapter)
        );
    }

    #[test]
    fn checkpoint_round_trips_cleanup_artifacts() {
        let artifacts = ChapterDerivedCleanupArtifacts {
            stale_profile_ids: vec![1],
            media_file_paths: vec!["projects/3/file.png".to_string()],
            scene_records_changed: true,
            ..Default::default()
        };
        let chapter = chapter();
        let fingerprint = chapter_content_sync_workflow_run_input(&chapter, true)
            .input_fingerprint_json
            .expect("workflow fingerprint should exist");
        let checkpoint =
            chapter_content_sync_checkpoint_input(9, fingerprint, &chapter, &artifacts)
                .expect("checkpoint should serialize");
        let restored = parse_cleanup_artifacts(checkpoint.artifacts_json.as_ref())
            .expect("checkpoint should parse");
        assert_eq!(restored.stale_profile_ids, artifacts.stale_profile_ids);
        assert_eq!(restored.media_file_paths, artifacts.media_file_paths);
        assert_eq!(
            restored.scene_records_changed,
            artifacts.scene_records_changed
        );
    }

    #[test]
    fn repeated_saves_get_distinct_runs_for_the_same_formal_content() {
        let chapter = chapter();
        let first = chapter_content_sync_workflow_run_input(&chapter, true)
            .input_fingerprint_json
            .expect("first workflow fingerprint should exist");
        let second = chapter_content_sync_workflow_run_input(&chapter, true)
            .input_fingerprint_json
            .expect("second workflow fingerprint should exist");

        assert_ne!(first, second);
        assert!(chapter_content_sync_run_input_matches(
            &Some(first),
            &chapter
        ));
        assert!(chapter_content_sync_run_input_matches(
            &Some(second),
            &chapter
        ));

        let malformed = json!({
            "schema_version": 2,
            "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            "source_fingerprint": chapter_content_sync_workflow_fingerprint(&chapter),
        });
        assert!(!chapter_content_sync_run_input_matches(
            &Some(malformed),
            &chapter
        ));
    }

    #[test]
    fn only_content_change_runs_sync_current_chapter_rag() {
        let chapter = chapter();
        let content_change =
            chapter_content_sync_workflow_run_input(&chapter, true).input_fingerprint_json;
        let analysis_cleanup =
            chapter_content_sync_workflow_run_input(&chapter, false).input_fingerprint_json;

        assert!(chapter_content_sync_should_sync_current_rag(
            &content_change
        ));
        assert_eq!(
            chapter_content_sync_current_rag_intent(&content_change),
            Some(true)
        );
        assert!(!chapter_content_sync_should_sync_current_rag(
            &analysis_cleanup
        ));
        assert_eq!(
            chapter_content_sync_current_rag_intent(&analysis_cleanup),
            Some(false)
        );
        assert!(chapter_content_sync_run_input_matches(
            &content_change,
            &chapter
        ));
        assert!(chapter_content_sync_run_input_matches(
            &analysis_cleanup,
            &chapter
        ));
        assert!(!chapter_content_sync_should_sync_current_rag(&Some(
            chapter_content_sync_workflow_fingerprint(&chapter)
        )));
        assert_eq!(
            chapter_content_sync_current_rag_intent(&Some(
                chapter_content_sync_workflow_fingerprint(&chapter)
            )),
            None
        );

        let legacy_v2 = Some(json!({
            "schema_version": 2,
            "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            "sync_revision": Uuid::new_v4(),
            "source_fingerprint": chapter_content_sync_workflow_fingerprint(&chapter),
        }));
        assert!(chapter_content_sync_run_input_matches(&legacy_v2, &chapter));
        assert!(!chapter_content_sync_should_sync_current_rag(&legacy_v2));

        let malformed = Some(json!({
            "schema_version": 3,
            "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            "sync_revision": Uuid::new_v4(),
            "source_fingerprint": chapter_content_sync_workflow_fingerprint(&chapter),
        }));
        assert!(!chapter_content_sync_run_input_matches(
            &malformed, &chapter
        ));
    }

    #[test]
    fn retry_plan_carries_forward_and_deduplicates_pending_cleanup() {
        let mut carried = ChapterDerivedCleanupArtifacts {
            stale_profile_ids: vec![3, 1],
            media_file_paths: vec!["projects/3/old.png".to_string()],
            ..Default::default()
        };
        carried.merge(ChapterDerivedCleanupArtifacts {
            stale_profile_ids: vec![3, 4],
            stale_character_state_ids: vec![8],
            media_file_paths: vec!["projects/3/old.png".to_string()],
            scene_records_changed: true,
            ..Default::default()
        });

        assert_eq!(carried.stale_profile_ids, vec![1, 3, 4]);
        assert_eq!(carried.stale_character_state_ids, vec![8]);
        assert_eq!(carried.media_file_paths, vec!["projects/3/old.png"]);
        assert!(carried.scene_records_changed);
    }
}
