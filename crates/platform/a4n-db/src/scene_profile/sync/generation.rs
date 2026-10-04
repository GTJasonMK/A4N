use sea_orm::ConnectionTrait;
use std::collections::BTreeSet;

use a4n_domain::{
    chapter::ChapterSourceSnapshot,
    generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput},
    scene_profile::{SceneProfileEvidenceRecordInput, SceneSyncRecordInput, SceneSyncRecordResult},
    text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind},
};

use super::super::{SceneProfileDbError, SceneProfileDbResult};
use super::replacement::replace_chapter_scene_sync_records;
use super::rules::{
    scene_sync_generation_response_summary, validate_scene_sync_generation_run_scope,
};
use crate::project::chapter_source_snapshot_targets;
use crate::text_anchor::EVIDENCE_ROLE_FIRST_SEEN;

pub async fn replace_chapter_scene_sync_with_generation_run_records(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    source: ChapterSourceSnapshot,
    input: SceneSyncRecordInput,
    profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
    mut generation_run: RecordCompletedGenerationRunInput,
) -> SceneProfileDbResult<SceneSyncRecordResult> {
    validate_scene_sync_generation_run_scope(
        project_id,
        generation_run.project_id,
        generation_run.targets.is_empty(),
    )?;
    if source.chapter_id != chapter_id
        || !chapter_source_snapshot_targets(&source, project_id, chapter_number)
    {
        return Err(super::super::SceneProfileDbError::InvalidArgument(
            "场景同步章节来源身份不一致".to_string(),
        ));
    }

    let txn = db.begin().await?;
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(
        &txn,
        std::slice::from_ref(&source),
    )
    .await?
    {
        return Err(super::super::SceneProfileDbError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: "保存场景同步结果；章节正文已变化，请重新分析".to_string(),
        });
    }
    let result =
        replace_chapter_scene_sync_records(&txn, project_id, chapter_id, chapter_number, input)
            .await?;
    merge_scene_profile_evidence_records(&txn, project_id, &result, profile_evidence).await?;
    generation_run.targets = result
        .profile_ids
        .iter()
        .map(|id| GenerationRunTraceTarget {
            target_kind: "scene_profile_record".to_string(),
            target_table: "scene_profile".to_string(),
            target_id: *id,
        })
        .collect();
    if !generation_run.targets.is_empty() {
        generation_run.response_summary_json =
            scene_sync_generation_response_summary(generation_run.response_summary_json, &result);
        crate::generation::record_completed_generation_run_with_checkpoint_in(
            &txn,
            generation_run,
            crate::generation::GenerationRunCheckpointContext::default(),
        )
        .await?;
    }
    txn.commit().await?;
    Ok(result)
}

async fn merge_scene_profile_evidence_records<C>(
    db: &C,
    project_id: i64,
    result: &SceneSyncRecordResult,
    inputs: Vec<SceneProfileEvidenceRecordInput>,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    let mut bound = BTreeSet::new();
    let mut evidence_batches = Vec::with_capacity(inputs.len());
    for input in inputs {
        if !bound.insert(input.scene_ref.clone()) {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景档案证据存在重复 scene_ref: {}",
                input.scene_ref
            )));
        }
        let profile_id = result
            .profile_ref_ids
            .get(&input.scene_ref)
            .copied()
            .ok_or_else(|| SceneProfileDbError::InvalidState {
                current: format!("场景档案写入结果缺少 scene_ref: {}", input.scene_ref),
                operation: "绑定场景档案证据".to_string(),
            })?;
        let refs = input
            .text_anchor_ids
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|text_anchor_id| TextAnchorEvidenceInput {
                text_anchor_id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            })
            .collect::<Vec<_>>();
        if refs.is_empty() {
            return Err(SceneProfileDbError::InvalidArgument(
                "场景档案 profile_evidence 不能为空".to_string(),
            ));
        }
        evidence_batches.push((profile_id, refs));
    }
    if bound.len() != result.profile_ref_ids.len() {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景档案证据未覆盖全部写入结果".to_string(),
        ));
    }
    crate::text_anchor::merge_profile_evidence_records_batch(
        db,
        project_id,
        None,
        TextAnchorProfileKind::Scene,
        evidence_batches,
    )
    .await?;
    Ok(())
}
