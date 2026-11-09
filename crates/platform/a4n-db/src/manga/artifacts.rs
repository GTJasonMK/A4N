use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_db_models::models::chapter::manga_pipeline_artifact::{self, Column as ArtifactColumn};
use a4n_domain::manga::{
    MangaPagePromptArtifactMetadata, MangaPagePromptRecord, MangaPipelineArtifactDraft,
    MangaPipelineArtifactRecord, MangaPipelineArtifactStage,
};
use a4n_domain::source_text_hash;
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::workflows::{invalidate_deleted_manga_checkpoints, lock_manga_manual_write_scope};
use super::{
    MangaPromptRecordError, manga_record_database, manga_video_error,
    records::update_manga_page_prompt_layout_expected, require_positive_manga_id,
    validate_manga_page_prompt_layout,
};

#[derive(Debug, Clone)]
pub struct MangaPipelineArtifactProjectionReset {
    pub page_prompt_id: i64,
    pub page_number: u32,
    pub expected_layout: serde_json::Value,
    pub expected_generated_image_id: Option<i64>,
    pub layout: serde_json::Value,
    pub image_prompt: String,
    pub clear_generated_image: bool,
}

#[derive(Debug, Clone)]
pub struct MangaPipelineArtifactProjectionUpdate {
    pub page_prompt: MangaPagePromptRecord,
    pub layout: serde_json::Value,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone)]
pub struct MangaPipelineArtifactDeletionPlan {
    pub artifacts: Vec<MangaPipelineArtifactRecord>,
    pub fingerprint: String,
}

pub async fn manga_pipeline_artifacts_by_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPipelineArtifactRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    manga_pipeline_artifact::Entity::find()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .order_by_asc(ArtifactColumn::Stage)
        .order_by_asc(ArtifactColumn::PageNumber)
        .all(db)
        .await
        .map_err(manga_record_database("读取漫画阶段产物失败"))?
        .into_iter()
        .map(manga_pipeline_artifact_record)
        .collect()
}

#[derive(Debug, FromQueryResult)]
struct MangaPagePromptArtifactRow {
    stage: String,
    page_number: i32,
    stale: bool,
    page_slots_payload: Option<serde_json::Value>,
}

/// Reads artifact metadata for prompt-list projection without materializing
/// page payloads that are only needed by the full artifact detail read.
pub async fn manga_pipeline_artifact_metadata_by_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPagePromptArtifactMetadata>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let rows = manga_pipeline_artifact::Entity::find()
        .select_only()
        .column(ArtifactColumn::Stage)
        .column(ArtifactColumn::PageNumber)
        .column(ArtifactColumn::Stale)
        .column_as(
            Expr::cust(
                "CASE WHEN stage = 'page_slots' AND stale = FALSE THEN payload ELSE NULL END",
            ),
            "page_slots_payload",
        )
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .order_by_asc(ArtifactColumn::Stage)
        .order_by_asc(ArtifactColumn::PageNumber)
        .into_model::<MangaPagePromptArtifactRow>()
        .all(db)
        .await
        .map_err(manga_record_database("读取漫画阶段产物索引失败"))?;

    rows.into_iter()
        .map(|row| {
            let (stage, page_number) = manga_pipeline_artifact_scope(&row.stage, row.page_number)?;
            Ok(MangaPagePromptArtifactMetadata {
                stage,
                page_number,
                stale: row.stale,
                page_slots_payload: row.page_slots_payload,
            })
        })
        .collect()
}

/// Read chapter-level artifacts for several chapters with one database round trip.
pub async fn manga_pipeline_artifacts_by_chapters(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_ids: &[i64],
    stage: MangaPipelineArtifactStage,
) -> Result<HashMap<i64, MangaPipelineArtifactRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    for chapter_id in chapter_ids {
        require_positive_manga_id(*chapter_id, "chapter_id")?;
    }
    if chapter_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let page_number = artifact_scope(stage, None)?;
    let records = manga_pipeline_artifact::Entity::find()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.is_in(chapter_ids.to_vec()))
        .filter(ArtifactColumn::Stage.eq(stage.key()))
        .filter(ArtifactColumn::PageNumber.eq(page_number))
        .all(db)
        .await
        .map_err(manga_record_database("批量读取漫画阶段产物失败"))?
        .into_iter()
        .map(manga_pipeline_artifact_record)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records
        .into_iter()
        .map(|record| (record.chapter_id, record))
        .collect())
}

pub async fn manga_pipeline_artifact(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Result<Option<MangaPipelineArtifactRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let page_number = artifact_scope(stage, page_number)?;
    let record = manga_pipeline_artifact::Entity::find()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .filter(ArtifactColumn::Stage.eq(stage.key()))
        .filter(ArtifactColumn::PageNumber.eq(page_number))
        .one(db)
        .await
        .map_err(manga_record_database("读取漫画阶段产物失败"))?;
    record.map(manga_pipeline_artifact_record).transpose()
}

pub async fn upsert_manga_pipeline_artifact(
    db: &impl crate::DbExecutor,
    draft: MangaPipelineArtifactDraft,
    now: NaiveDateTime,
) -> Result<MangaPipelineArtifactRecord, MangaPromptRecordError> {
    let (page_number, output_fingerprint) = prepare_manga_pipeline_artifact_draft(&draft)?;
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画阶段产物保存事务失败"))?;
    let updated = upsert_manga_pipeline_artifact_in_transaction(
        &txn,
        draft,
        page_number,
        output_fingerprint,
        now,
    )
    .await?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画阶段产物事务失败"))?;
    Ok(updated)
}

pub async fn create_manga_pipeline_artifact(
    db: &impl crate::DbExecutor,
    draft: MangaPipelineArtifactDraft,
    now: NaiveDateTime,
) -> Result<MangaPipelineArtifactRecord, MangaPromptRecordError> {
    let (page_number, output_fingerprint) = prepare_manga_pipeline_artifact_draft(&draft)?;
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画阶段产物创建事务失败"))?;
    lock_manga_pipeline_artifact_chapter(&txn, draft.project_id, draft.chapter_id).await?;
    if artifact_by_scope(
        &txn,
        draft.project_id,
        draft.chapter_id,
        draft.stage,
        page_number,
    )
    .await?
    .is_some()
    {
        let scope = draft
            .page_number
            .map(|page| format!("第 {page} 页"))
            .unwrap_or_else(|| "本章节".to_string());
        return Err(MangaPromptRecordError::Conflict(format!(
            "{scope}的{}产物已存在，请先删除后再生成",
            draft.stage.key()
        )));
    }

    manga_pipeline_artifact::Entity::insert(manga_pipeline_artifact::ActiveModel {
        project_id: Set(draft.project_id),
        chapter_id: Set(draft.chapter_id),
        stage: Set(draft.stage.key().to_string()),
        page_number: Set(page_number),
        input_fingerprint: Set(draft.input_fingerprint),
        output_fingerprint: Set(output_fingerprint),
        payload: Set(draft.payload),
        stale: Set(draft.stale),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(&txn)
    .await
    .map_err(manga_record_database("创建漫画阶段产物失败"))?;
    mark_downstream_stale(
        &txn,
        draft.project_id,
        draft.chapter_id,
        draft.stage,
        page_number,
    )
    .await?;
    let created = artifact_by_scope(
        &txn,
        draft.project_id,
        draft.chapter_id,
        draft.stage,
        page_number,
    )
    .await?
    .ok_or_else(|| MangaPromptRecordError::NotFound("漫画阶段产物创建后未找到".into()))?;
    let created = manga_pipeline_artifact_record(created)?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画阶段产物创建事务失败"))?;
    Ok(created)
}

pub async fn save_manga_pipeline_artifact_with_projection(
    db: &impl crate::DbExecutor,
    draft: MangaPipelineArtifactDraft,
    projection: Option<MangaPipelineArtifactProjectionUpdate>,
    now: NaiveDateTime,
) -> Result<MangaPipelineArtifactRecord, MangaPromptRecordError> {
    let (page_number, output_fingerprint) = prepare_manga_pipeline_artifact_draft(&draft)?;
    if let Some(projection) = projection.as_ref() {
        let projected_page_number = u32::try_from(projection.page_prompt.page_number)
            .map_err(|_| MangaPromptRecordError::InvalidArgument("漫画页码无效".into()))?;
        if !draft.stage.page_scoped()
            || projection.page_prompt.project_id != draft.project_id
            || projection.page_prompt.chapter_id != draft.chapter_id
            || draft.page_number != Some(projected_page_number)
        {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画阶段产物与页面投影范围不一致".into(),
            ));
        }
        if projection.expected_updated_at.trim().is_empty() {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画页面投影缺少预期版本，请刷新后重试".into(),
            ));
        }
    } else if draft.stage.page_scoped() {
        return Err(MangaPromptRecordError::InvalidArgument(
            "保存页面阶段产物必须提供页面提示词版本".into(),
        ));
    }

    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画阶段产物保存事务失败"))?;
    lock_manga_manual_write_scope(&txn, draft.project_id, draft.chapter_id).await?;

    let updated = upsert_manga_pipeline_artifact_in_transaction(
        &txn,
        draft,
        page_number,
        output_fingerprint,
        now,
    )
    .await?;
    if let Some(projection) = projection {
        update_manga_page_prompt_layout_expected(
            &txn,
            projection.page_prompt,
            projection.layout,
            Some(&projection.expected_updated_at),
        )
        .await?;
    }
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画阶段产物事务失败"))?;
    Ok(updated)
}

fn prepare_manga_pipeline_artifact_draft(
    draft: &MangaPipelineArtifactDraft,
) -> Result<(i32, String), MangaPromptRecordError> {
    require_positive_manga_id(draft.project_id, "project_id")?;
    require_positive_manga_id(draft.chapter_id, "chapter_id")?;
    let page_number = artifact_scope(draft.stage, draft.page_number)?;
    if draft.input_fingerprint.trim().is_empty() || draft.payload.is_null() {
        return Err(MangaPromptRecordError::InvalidArgument(
            "阶段输入指纹和产物内容不能为空".into(),
        ));
    }
    let output_fingerprint =
        source_text_hash(&serde_json::to_string(&draft.payload).map_err(|error| {
            MangaPromptRecordError::InvalidArgument(format!("阶段产物无法编码：{error}"))
        })?);
    Ok((page_number, output_fingerprint))
}

async fn upsert_manga_pipeline_artifact_in_transaction<C: ConnectionTrait>(
    db: &C,
    draft: MangaPipelineArtifactDraft,
    page_number: i32,
    output_fingerprint: String,
    now: NaiveDateTime,
) -> Result<MangaPipelineArtifactRecord, MangaPromptRecordError> {
    lock_manga_pipeline_artifact_chapter(db, draft.project_id, draft.chapter_id).await?;
    let existing = artifact_by_scope(
        db,
        draft.project_id,
        draft.chapter_id,
        draft.stage,
        page_number,
    )
    .await?;
    let changed = existing.as_ref().is_none_or(|record| {
        record.input_fingerprint != draft.input_fingerprint
            || record.output_fingerprint != output_fingerprint
    });
    manga_pipeline_artifact::Entity::insert(manga_pipeline_artifact::ActiveModel {
        project_id: Set(draft.project_id),
        chapter_id: Set(draft.chapter_id),
        stage: Set(draft.stage.key().to_string()),
        page_number: Set(page_number),
        input_fingerprint: Set(draft.input_fingerprint),
        output_fingerprint: Set(output_fingerprint),
        payload: Set(draft.payload),
        stale: Set(draft.stale),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            ArtifactColumn::ProjectId,
            ArtifactColumn::ChapterId,
            ArtifactColumn::Stage,
            ArtifactColumn::PageNumber,
        ])
        .update_columns([
            ArtifactColumn::InputFingerprint,
            ArtifactColumn::OutputFingerprint,
            ArtifactColumn::Payload,
            ArtifactColumn::Stale,
            ArtifactColumn::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec(db)
    .await
    .map_err(manga_record_database("保存漫画阶段产物失败"))?;
    if changed {
        mark_downstream_stale(
            db,
            draft.project_id,
            draft.chapter_id,
            draft.stage,
            page_number,
        )
        .await?;
    }
    let updated = artifact_by_scope(
        db,
        draft.project_id,
        draft.chapter_id,
        draft.stage,
        page_number,
    )
    .await?
    .ok_or_else(|| MangaPromptRecordError::NotFound("漫画阶段产物保存后未找到".into()))?;
    manga_pipeline_artifact_record(updated)
}

pub async fn manga_pipeline_artifact_deletion_plan(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Result<MangaPipelineArtifactDeletionPlan, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let page_number = artifact_scope(stage, page_number)?;
    manga_pipeline_artifact_deletion_plan_in_connection(
        db,
        project_id,
        chapter_id,
        stage,
        page_number,
    )
    .await
}

pub async fn delete_manga_pipeline_artifact(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
    expected_impact_fingerprint: &str,
    projection_resets: &[MangaPipelineArtifactProjectionReset],
) -> Result<Vec<MangaPipelineArtifactRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let page_number = artifact_scope(stage, page_number)?;
    if expected_impact_fingerprint.len() != 64
        || !expected_impact_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(MangaPromptRecordError::InvalidArgument(
            "漫画阶段产物删除范围无效，请重新预览".into(),
        ));
    }
    for reset in projection_resets {
        let page_number = i32::try_from(reset.page_number)
            .map_err(|_| MangaPromptRecordError::InvalidArgument("漫画页面页码超出范围".into()))?;
        validate_manga_page_prompt_layout(page_number, Some(&reset.layout))
            .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
        let layout: a4n_domain::manga::MangaPageGenerationLayoutV4 =
            serde_json::from_value(reset.layout.clone())
                .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
        let mut cleared = layout.clone();
        cleared.clear_deleted_stage(stage);
        if layout != cleared
            || reset.image_prompt
                != layout
                    .pipeline
                    .drawing_prompt
                    .as_ref()
                    .map(|prompt| prompt.image_prompt.as_str())
                    .unwrap_or_default()
        {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画页面未完整清理所删除阶段及其下游投影".into(),
            ));
        }
    }
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画阶段产物删除事务失败"))?;
    let run_ids = lock_manga_manual_write_scope(&txn, project_id, chapter_id).await?;
    lock_manga_pipeline_artifact_chapter(&txn, project_id, chapter_id).await?;
    let plan = manga_pipeline_artifact_deletion_plan_in_connection(
        &txn,
        project_id,
        chapter_id,
        stage,
        page_number,
    )
    .await?;
    if plan.fingerprint != expected_impact_fingerprint {
        return Err(MangaPromptRecordError::InvalidArgument(
            "漫画阶段产物范围已变化，请重新预览后再删除".into(),
        ));
    }
    let artifacts = plan.artifacts;
    let actual_ids = artifacts
        .iter()
        .map(|artifact| artifact.id)
        .collect::<Vec<_>>();
    let mut reset_page_numbers = projection_resets
        .iter()
        .map(|reset| reset.page_number)
        .collect::<Vec<_>>();
    reset_page_numbers.sort_unstable();
    let mut page_query = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_manga_prompt::Column::PromptType
                .eq(chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE),
        );
    if stage.page_scoped() {
        page_query = page_query.filter(chapter_manga_prompt::Column::PageNumber.eq(page_number));
    }
    let mut expected_prompt_ids = page_query
        .into_tuple::<i64>()
        .all(&txn)
        .await
        .map_err(manga_record_database("读取漫画页面删除范围失败"))?;
    expected_prompt_ids.sort_unstable();
    let mut reset_prompt_ids = projection_resets
        .iter()
        .map(|reset| reset.page_prompt_id)
        .collect::<Vec<_>>();
    reset_prompt_ids.sort_unstable();
    if reset_page_numbers
        .windows(2)
        .any(|pages| pages[0] == pages[1])
        || reset_prompt_ids != expected_prompt_ids
        || projection_resets.iter().any(|reset| {
            reset.page_prompt_id <= 0
                || reset.page_number == 0
                || (stage.page_scoped()
                    && Some(reset.page_number) != u32::try_from(page_number).ok())
                || reset.clear_generated_image
                    != (stage == MangaPipelineArtifactStage::Image
                        || MangaPipelineArtifactStage::Image.depends_on(stage))
        })
    {
        return Err(MangaPromptRecordError::InvalidArgument(
            "漫画页面投影清理范围无效，请重新预览".into(),
        ));
    }
    let mut generated_image_paths = Vec::new();
    for reset in projection_resets {
        let reset_page_number = i32::try_from(reset.page_number)
            .map_err(|_| MangaPromptRecordError::InvalidArgument("漫画页面页码超出范围".into()))?;
        let prompt = chapter_manga_prompt::Entity::find_by_id(reset.page_prompt_id)
            .lock_exclusive()
            .one(&txn)
            .await
            .map_err(manga_record_database("读取漫画页面投影失败"))?
            .ok_or_else(|| MangaPromptRecordError::NotFound("漫画页面不存在".into()))?;
        if prompt.project_id != project_id
            || prompt.chapter_id != chapter_id
            || prompt.page_number != reset_page_number
            || prompt.prompt_type != chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE
            || prompt.layout.as_ref() != Some(&reset.expected_layout)
            || prompt.generated_image_id != reset.expected_generated_image_id
        {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画页面投影已变化，请重新预览后再删除".into(),
            ));
        }
        if reset.clear_generated_image {
            generated_image_paths.extend(
                crate::video::delete_manga_page_image_records_in(
                    &txn,
                    project_id,
                    chapter_id,
                    reset_page_number,
                    None,
                )
                .await
                .map_err(manga_video_error)?,
            );
        }
        let clear_storyboard = stage == MangaPipelineArtifactStage::Storyboard
            || MangaPipelineArtifactStage::Storyboard.depends_on(stage);
        chapter_manga_prompt::ActiveModel {
            id: Set(prompt.id),
            layout: Set(Some(reset.layout.clone())),
            image_prompt: Set(reset.image_prompt.clone()),
            dialogue_text: Set(if clear_storyboard {
                None
            } else {
                prompt.dialogue_text
            }),
            characters_in_frame: Set(if clear_storyboard {
                None
            } else {
                prompt.characters_in_frame
            }),
            generated_image_id: Set(if reset.clear_generated_image {
                None
            } else {
                prompt.generated_image_id
            }),
            status: Set(
                if reset.clear_generated_image || prompt.generated_image_id.is_none() {
                    "pending".to_string()
                } else {
                    prompt.status
                },
            ),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .update(&txn)
        .await
        .map_err(manga_record_database("清理漫画页面投影失败"))?;
    }
    // Chapter artifacts can also own media whose page projection was never published.
    if stage == MangaPipelineArtifactStage::Image
        || MangaPipelineArtifactStage::Image.depends_on(stage)
    {
        generated_image_paths.extend(
            if stage.page_scoped() {
                crate::video::delete_manga_page_image_records_in(
                    &txn,
                    project_id,
                    chapter_id,
                    page_number,
                    None,
                )
                .await
            } else {
                crate::video::delete_manga_chapter_image_records_in(
                    &txn, project_id, chapter_id, None,
                )
                .await
            }
            .map_err(manga_video_error)?,
        );
    }
    let deleted = manga_pipeline_artifact::Entity::delete_many()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .filter(ArtifactColumn::Id.is_in(actual_ids))
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除漫画阶段产物失败"))?;
    if deleted.rows_affected != artifacts.len() as u64 {
        return Err(MangaPromptRecordError::InvalidArgument(
            "漫画阶段产物范围已变化，请重新预览后再删除".into(),
        ));
    }
    invalidate_deleted_manga_checkpoints(
        &txn,
        &run_ids,
        chapter_id,
        Some((
            stage,
            if stage.page_scoped() {
                Some(page_number as u32)
            } else {
                None
            },
        )),
    )
    .await?;
    if !generated_image_paths.is_empty() {
        let cleanup_hash = crate::generation::canonical_json_sha256(&generated_image_paths)
            .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
        crate::media::schedule_unreferenced_media_cleanup(
            &txn,
            project_id,
            &generated_image_paths,
            format!("manga-artifact-delete:{project_id}:{chapter_id}:{cleanup_hash}"),
        )
        .await
        .map_err(manga_record_database("安排漫画阶段成图文件清理失败"))?;
    }
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画阶段产物删除事务失败"))?;
    Ok(artifacts)
}

async fn manga_pipeline_artifact_deletion_plan_in_connection<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: i32,
) -> Result<MangaPipelineArtifactDeletionPlan, MangaPromptRecordError> {
    let dependent_stages = MangaPipelineArtifactStage::all()
        .iter()
        .copied()
        .filter(|candidate| *candidate == stage || candidate.depends_on(stage))
        .map(|candidate| candidate.key())
        .collect::<Vec<_>>();
    let query = manga_pipeline_artifact::Entity::find()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .filter(ArtifactColumn::Stage.is_in(dependent_stages));
    let query = if stage.page_scoped() {
        query.filter(ArtifactColumn::PageNumber.eq(page_number))
    } else {
        query
    };
    let records = query
        .order_by_asc(ArtifactColumn::Stage)
        .order_by_asc(ArtifactColumn::PageNumber)
        .all(db)
        .await
        .map_err(manga_record_database("读取漫画阶段产物删除范围失败"))?
        .into_iter()
        .map(manga_pipeline_artifact_record)
        .collect::<Result<Vec<_>, _>>()?;
    let expected_page_number = if stage.page_scoped() {
        Some(
            u32::try_from(page_number)
                .map_err(|_| MangaPromptRecordError::InvalidArgument("漫画页码无效".into()))?,
        )
    } else {
        None
    };
    let mut pages = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_manga_prompt::Column::PromptType
                .eq(chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE),
        );
    if stage.page_scoped() {
        pages = pages.filter(chapter_manga_prompt::Column::PageNumber.eq(page_number));
    }
    let pages = pages
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .lock_exclusive()
        .all(db)
        .await
        .map_err(manga_record_database("读取漫画删除投影版本失败"))?;
    let has_artifact = records
        .iter()
        .any(|artifact| artifact.stage == stage && artifact.page_number == expected_page_number);
    // Direct page-image generation binds the image without publishing a pipeline artifact.
    if !has_artifact
        && !(stage == MangaPipelineArtifactStage::Image
            && pages.iter().any(|page| page.generated_image_id.is_some()))
    {
        return Err(MangaPromptRecordError::NotFound(
            "漫画阶段产物不存在".to_string(),
        ));
    }
    let fingerprint = crate::generation::canonical_json_sha256(&(
        project_id,
        chapter_id,
        stage,
        page_number,
        &records,
        pages,
    ))
    .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
    Ok(MangaPipelineArtifactDeletionPlan {
        artifacts: records,
        fingerprint,
    })
}

async fn lock_manga_pipeline_artifact_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), MangaPromptRecordError> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_id.hash(&mut hasher);
    chapter_id.hash(&mut hasher);
    let lock_key = hasher.finish() as i64;
    let _ = db
        .query_one(sea_orm::Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock($1)",
            [lock_key.into()],
        ))
        .await
        .map_err(manga_record_database("锁定漫画阶段产物范围失败"))?;
    Ok(())
}

async fn artifact_by_scope<C: sea_orm::ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    stage: MangaPipelineArtifactStage,
    page_number: i32,
) -> Result<Option<manga_pipeline_artifact::Model>, MangaPromptRecordError> {
    manga_pipeline_artifact::Entity::find()
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .filter(ArtifactColumn::Stage.eq(stage.key()))
        .filter(ArtifactColumn::PageNumber.eq(page_number))
        .one(db)
        .await
        .map_err(manga_record_database("读取漫画阶段产物失败"))
}

async fn mark_downstream_stale<C: sea_orm::ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    upstream: MangaPipelineArtifactStage,
    page_number: i32,
) -> Result<(), MangaPromptRecordError> {
    let stages = MangaPipelineArtifactStage::all()
        .iter()
        .copied()
        .filter(|stage| stage.depends_on(upstream))
        .map(|stage| stage.key())
        .collect::<Vec<_>>();
    if stages.is_empty() {
        return Ok(());
    }
    let mut update = manga_pipeline_artifact::Entity::update_many()
        .col_expr(ArtifactColumn::Stale, Expr::value(true))
        .filter(ArtifactColumn::ProjectId.eq(project_id))
        .filter(ArtifactColumn::ChapterId.eq(chapter_id))
        .filter(ArtifactColumn::Stage.is_in(stages));
    if upstream.page_scoped() {
        update = update.filter(ArtifactColumn::PageNumber.eq(page_number));
    }
    update
        .exec(db)
        .await
        .map_err(manga_record_database("标记漫画下游阶段产物过期失败"))?;
    Ok(())
}

fn artifact_scope(
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Result<i32, MangaPromptRecordError> {
    match (stage.page_scoped(), page_number) {
        (true, Some(page_number)) if page_number > 0 => i32::try_from(page_number)
            .map_err(|_| MangaPromptRecordError::InvalidArgument("漫画页码超出范围".into())),
        (false, None) => Ok(0),
        (true, _) => Err(MangaPromptRecordError::InvalidArgument(
            "页级阶段必须提供正整数页码".into(),
        )),
        (false, Some(_)) => Err(MangaPromptRecordError::InvalidArgument(
            "章节级阶段不能指定页码".into(),
        )),
    }
}

fn manga_pipeline_artifact_record(
    record: manga_pipeline_artifact::Model,
) -> Result<MangaPipelineArtifactRecord, MangaPromptRecordError> {
    let (stage, page_number) = manga_pipeline_artifact_scope(&record.stage, record.page_number)?;
    Ok(MangaPipelineArtifactRecord {
        id: record.id,
        project_id: record.project_id,
        chapter_id: record.chapter_id,
        stage,
        page_number,
        input_fingerprint: record.input_fingerprint,
        output_fingerprint: record.output_fingerprint,
        payload: record.payload,
        stale: record.stale,
        created_at: record.created_at.to_string(),
        updated_at: record.updated_at.to_string(),
    })
}

fn manga_pipeline_artifact_scope(
    stage: &str,
    page_number: i32,
) -> Result<(MangaPipelineArtifactStage, Option<u32>), MangaPromptRecordError> {
    let stage = MangaPipelineArtifactStage::from_key(stage).ok_or_else(|| {
        MangaPromptRecordError::InvalidArgument(format!("数据库中的漫画阶段无效：{stage}"))
    })?;
    let page_number = if page_number == 0 {
        None
    } else {
        Some(u32::try_from(page_number).map_err(|_| {
            MangaPromptRecordError::InvalidArgument("数据库中的漫画页码无效".into())
        })?)
    };
    Ok((stage, page_number))
}
