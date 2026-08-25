use std::collections::{BTreeSet, HashMap, HashSet};

use super::contracts::{
    AssetDbError, ObjectProfileEvidenceRecordInput, ObjectProfileRecordInput,
    ObjectProfileUpsertRecordResult,
};
use super::profiles::{ensure_object_profile_project_exists, upsert_object_profiles_in};
use crate::project::ChapterSourceSnapshot;
use crate::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, TextAnchorEvidenceInput, TextAnchorProfileKind,
};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::{Value, json};

pub async fn upsert_object_profiles_with_generation_run_record<C>(
    db: &C,
    project_id: i64,
    inputs: Vec<ObjectProfileRecordInput>,
    source_chapters: Vec<ChapterSourceSnapshot>,
    profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
    mut generation_run: crate::generation::RecordCompletedGenerationRunInput,
) -> Result<ObjectProfileUpsertRecordResult, AssetDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if generation_run.project_id != project_id {
        return Err(AssetDbError::InvalidArgument(
            "物体档案生成运行记录 project_id 与项目不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(AssetDbError::InvalidArgument(
            "物体档案生成运行记录目标由物体服务绑定，调用方不能预填 targets".to_string(),
        ));
    }
    if source_chapters.is_empty()
        || source_chapters
            .iter()
            .any(|source| source.project_id != project_id)
    {
        return Err(AssetDbError::InvalidArgument(
            "物体档案发现缺少有效章节来源快照".to_string(),
        ));
    }

    let txn = db.begin().await?;
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(&txn, &source_chapters)
        .await?
    {
        return Err(AssetDbError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: "保存物体档案发现结果；章节正文已变化，请重新分析".to_string(),
        });
    }
    ensure_object_profile_project_exists(&txn, project_id).await?;
    let result = upsert_object_profiles_in(&txn, project_id, inputs).await?;
    merge_object_profile_evidence_records(&txn, project_id, &result, profile_evidence).await?;
    generation_run.targets = result
        .created
        .iter()
        .chain(result.updated.iter())
        .map(|profile| crate::generation::GenerationRunTraceTarget {
            target_kind: "object_profile_record".to_string(),
            target_table: "object_profile".to_string(),
            target_id: profile.id,
        })
        .collect();
    if !generation_run.targets.is_empty() {
        generation_run.response_summary_json = object_profile_generation_response_summary(
            generation_run.response_summary_json,
            &result,
        );
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

async fn merge_object_profile_evidence_records<C>(
    db: &C,
    project_id: i64,
    result: &ObjectProfileUpsertRecordResult,
    inputs: Vec<ObjectProfileEvidenceRecordInput>,
) -> Result<(), AssetDbError>
where
    C: ConnectionTrait,
{
    let profiles = result
        .created
        .iter()
        .chain(result.updated.iter())
        .map(|profile| (profile.normalized_name.clone(), profile.id))
        .collect::<HashMap<_, _>>();
    let mut bound = HashSet::new();
    let mut evidence_batches = Vec::with_capacity(inputs.len());
    for input in inputs {
        if !bound.insert(input.normalized_name.clone()) {
            return Err(AssetDbError::InvalidArgument(format!(
                "物体档案证据存在重复对象: {}",
                input.normalized_name
            )));
        }
        let profile_id = profiles
            .get(&input.normalized_name)
            .copied()
            .ok_or_else(|| AssetDbError::InvalidState {
                current: format!("物体档案写入结果缺少对象: {}", input.normalized_name),
                operation: "绑定物体档案证据".to_string(),
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
            return Err(AssetDbError::InvalidArgument(
                "物体档案 profile_evidence 不能为空".to_string(),
            ));
        }
        evidence_batches.push((profile_id, refs));
    }
    if bound.len() != profiles.len() {
        return Err(AssetDbError::InvalidArgument(
            "物体档案证据未覆盖全部写入结果".to_string(),
        ));
    }
    crate::text_anchor::merge_profile_evidence_records_batch(
        db,
        project_id,
        None,
        TextAnchorProfileKind::Object,
        evidence_batches,
    )
    .await?;
    Ok(())
}

fn object_profile_generation_response_summary(
    summary: Option<Value>,
    result: &ObjectProfileUpsertRecordResult,
) -> Option<Value> {
    let generated = json!({
        "created_count": result.created.len(),
        "updated_count": result.updated.len(),
        "profile_ids": result
            .created
            .iter()
            .chain(result.updated.iter())
            .map(|profile| profile.id)
            .collect::<Vec<_>>(),
    });
    match summary {
        Some(Value::Object(mut map)) => {
            if let Some(generated) = generated.as_object() {
                for (key, value) in generated {
                    map.insert(key.clone(), value.clone());
                }
            }
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "object_profiles": generated,
        })),
        None => Some(generated),
    }
}
