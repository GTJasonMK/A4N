use a4n_db_models::models::{
    chapter::chapter_visual_plan,
    project::{project_source_snapshot, source_snapshot_chapter},
};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value;
use std::collections::HashMap;

use super::data::build_source_snapshot_data;
use crate::project::{
    ProjectMutationError,
    source_snapshot::{
        DerivedChapterScope,
        hashing::{hash_json, hash_text},
        outlines::freeze_outlines,
        profiles::freeze_profiles,
    },
};

struct PendingSnapshotAnchor {
    source_chapter_id: i64,
    source_snapshot_chapter_id: i64,
    source_id: i64,
    chapter_number: i32,
    content: String,
    source_version_hash: String,
}

pub(crate) async fn create_source_snapshot_in_record<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
    now: NaiveDateTime,
) -> Result<project_source_snapshot::Model, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let data = build_source_snapshot_data(db, root_project_id, scope).await?;

    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root_project_id),
        source_version_hash: Set(data.source_version_hash),
        blueprint_version_id: Set(data.latest_blueprint_version_id),
        chapter_scope_json: Set(data.chapter_scope_json.clone()),
        chapter_hashes_json: Set(Value::Array(data.chapter_hashes_json.clone())),
        profile_hashes_json: Set(data.profile_hashes_json.clone()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)?;

    let chapter_ids = data
        .chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let latest_plan_version_by_chapter = chapter_visual_plan::Entity::find()
        .select_only()
        .column(chapter_visual_plan::Column::SourceChapterId)
        .column(chapter_visual_plan::Column::PlanVersion)
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.is_in(chapter_ids.iter().copied()))
        .distinct_on([chapter_visual_plan::Column::SourceChapterId])
        .order_by_asc(chapter_visual_plan::Column::SourceChapterId)
        .order_by_desc(chapter_visual_plan::Column::PlanVersion)
        .order_by_desc(chapter_visual_plan::Column::Id)
        .into_tuple::<(i64, i32)>()
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)?
        .into_iter()
        .fold(
            HashMap::new(),
            |mut versions, (source_chapter_id, version)| {
                versions.entry(source_chapter_id).or_insert(version);
                versions
            },
        );

    let snapshot_chapter_rows = data
        .chapters
        .iter()
        .map(|chapter| source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title.clone()),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary.clone()),
            analysis_data_json: Set(chapter.analysis_data.clone()),
            content_hash: Set(hash_text(&chapter.content)),
            analysis_hash: Set(chapter.analysis_data.as_ref().map(hash_json)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted_snapshot_chapters = if snapshot_chapter_rows.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::insert_many(snapshot_chapter_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(ProjectMutationError::Database)?
    };
    if inserted_snapshot_chapters.len() != data.chapters.len() {
        return Err(ProjectMutationError::Internal(format!(
            "源快照章节批量写入返回行数不一致: inserted={}, expected={}",
            inserted_snapshot_chapters.len(),
            data.chapters.len()
        )));
    }
    let mut snapshot_chapters_by_source_id =
        HashMap::with_capacity(inserted_snapshot_chapters.len());
    for snapshot_chapter in inserted_snapshot_chapters {
        if snapshot_chapters_by_source_id
            .insert(snapshot_chapter.source_chapter_id, snapshot_chapter)
            .is_some()
        {
            return Err(ProjectMutationError::Internal(
                "源快照章节 source_chapter_id 重复".to_string(),
            ));
        }
    }

    let mut pending_anchors = Vec::with_capacity(data.chapters.len());
    for chapter in data.chapters {
        let mut snapshot_chapter = snapshot_chapters_by_source_id
            .remove(&chapter.id)
            .ok_or_else(|| {
                ProjectMutationError::Internal(format!(
                    "源快照章节批量写入后缺少 source_chapter_id={}",
                    chapter.id
                ))
            })?;
        if let Some(live_plan) = data.visual_plans.get(&chapter.id) {
            let source_revision_key = format!(
                "snapshot:{}:chapter:{}:{}",
                snapshot.id, snapshot_chapter.id, snapshot_chapter.content_hash
            );
            let snapshot_plan_version = latest_plan_version_by_chapter
                .get(&chapter.id)
                .copied()
                .map_or(Ok(1), |version| {
                    version.checked_add(1).ok_or_else(|| {
                        ProjectMutationError::Internal("章节视觉编排版本超出 i32 范围".to_string())
                    })
                })?;
            let snapshot_plan = chapter_visual_plan::ActiveModel {
                root_project_id: Set(live_plan.root_project_id),
                source_chapter_id: Set(live_plan.source_chapter_id),
                source_snapshot_id: Set(Some(snapshot.id)),
                source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
                source_revision_key: Set(source_revision_key.clone()),
                source_content_sha256: Set(live_plan.source_content_sha256.clone()),
                contract_version: Set(live_plan.contract_version.clone()),
                plan_version: Set(snapshot_plan_version),
                revision: Set(live_plan.revision),
                status: Set(live_plan.status.clone()),
                authoring_mode: Set(live_plan.authoring_mode.clone()),
                plan_sha256: Set(live_plan.plan_sha256.clone()),
                plan_json: Set(live_plan.plan_json.clone()),
                author_user_id: Set(live_plan.author_user_id),
                confirmed_at: Set(live_plan.confirmed_at),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(ProjectMutationError::Database)?;
            let mut snapshot_chapter_update: source_snapshot_chapter::ActiveModel =
                snapshot_chapter.clone().into();
            snapshot_chapter_update.visual_plan_id = Set(Some(snapshot_plan.id));
            snapshot_chapter_update.visual_plan_version = Set(Some(snapshot_plan.plan_version));
            snapshot_chapter_update.visual_plan_sha256 =
                Set(Some(snapshot_plan.plan_sha256.clone()));
            snapshot_chapter = snapshot_chapter_update
                .update(db)
                .await
                .map_err(ProjectMutationError::Database)?;
            if let Some(live_script) = data.shooting_scripts.get(&chapter.id) {
                let mut script: a4n_domain::shooting_script::ChapterShootingScriptV1 =
                    serde_json::from_value(live_script.script_json.clone()).map_err(|error| {
                        ProjectMutationError::Internal(format!("解析源拍摄剧本失败: {error}"))
                    })?;
                script.visual_plan_source =
                    Some(a4n_domain::shooting_script::ShootingVisualPlanSourceV1 {
                        plan_id: snapshot_plan.id,
                        plan_version: snapshot_plan.plan_version,
                        plan_sha256: snapshot_plan.plan_sha256.clone(),
                    });
                crate::shooting_script::insert_chapter_shooting_script(
                    db,
                    crate::shooting_script::InsertChapterShootingScriptInput {
                        source: crate::project::ResolvedChapterSource {
                            requested_project_id: root_project_id,
                            root_project_id,
                            source_chapter_id: chapter.id,
                            source_snapshot_id: Some(snapshot.id),
                            source_snapshot_chapter_id: Some(snapshot_chapter.id),
                            chapter_number: snapshot_chapter.chapter_number,
                            title: snapshot_chapter.title.clone(),
                            summary: snapshot_chapter.summary.clone(),
                            analysis_data_json: snapshot_chapter.analysis_data_json.clone(),
                            content: snapshot_chapter.content.clone(),
                            content_sha256: snapshot_chapter.content_hash.clone(),
                            source_revision_key,
                        },
                        input_fingerprint_sha256: live_script.input_fingerprint_sha256.clone(),
                        visual_plan: crate::shooting_script::ChapterVisualPlanIdentity {
                            id: snapshot_plan.id,
                            version: snapshot_plan.plan_version,
                            sha256: snapshot_plan.plan_sha256,
                        },
                        script,
                        workflow_run_id: None,
                    },
                )
                .await?;
            }
        }
        pending_anchors.push(PendingSnapshotAnchor {
            source_chapter_id: snapshot_chapter.source_chapter_id,
            source_snapshot_chapter_id: snapshot_chapter.id,
            source_id: snapshot_chapter.id,
            chapter_number: snapshot_chapter.chapter_number,
            content: snapshot_chapter.content,
            source_version_hash: snapshot_chapter.content_hash,
        });
    }

    if !pending_anchors.is_empty() {
        let inputs = pending_anchors
            .iter()
            .map(|pending| crate::text_anchor::TextAnchorContentInsertInput {
                project_id: root_project_id,
                source_chapter_id: Some(pending.source_chapter_id),
                source_snapshot_id: Some(snapshot.id),
                source_snapshot_chapter_id: Some(pending.source_snapshot_chapter_id),
                source_id: pending.source_id,
                chapter_number: pending.chapter_number,
                content: &pending.content,
                source_version_hash: &pending.source_version_hash,
            })
            .collect::<Vec<_>>();
        crate::text_anchor::insert_content_anchor_records_batch(db, &inputs).await?;
    }

    freeze_outlines(db, snapshot.id, root_project_id, scope).await?;
    freeze_profiles(db, snapshot.id, root_project_id, data.profiles).await?;

    Ok(snapshot)
}
