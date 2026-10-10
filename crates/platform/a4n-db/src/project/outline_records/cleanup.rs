use std::collections::{BTreeMap, BTreeSet};

use a4n_db_models::models::chapter::{chapter, chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::{CaseStatement, Expr},
};

use super::{DeleteLatestOutlineRecordsInput, OutlineDeleteTarget, ProjectOutlineRecordError};
use crate::generation::delete_generation_run_targets_for;

pub async fn delete_latest_part_outline_records(
    db: &impl crate::DbExecutor,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<(), ProjectOutlineRecordError> {
    validate_delete_input(&input)?;
    let txn = db.begin().await?;
    super::mutation::lock_project_for_write(&txn, input.project_id, "删除分部大纲").await?;

    let mut query = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let parts = query.all(&txn).await?;
    crate::outline::validate_part_outline_records(&parts)?;
    ensure_latest_targets_match(
        parts
            .iter()
            .rev()
            .take(input.targets.len())
            .map(|part| (part.id, part.updated_at))
            .collect(),
        &input.targets,
        "删除旧分部大纲确认",
    )?;

    let part_ids = input
        .targets
        .iter()
        .map(|target| target.id)
        .collect::<Vec<_>>();
    let mut outline_query = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .filter(chapter_outline::Column::PartId.is_in(part_ids.iter().copied()));
    if txn.get_database_backend() != DbBackend::Sqlite {
        outline_query = outline_query.lock_exclusive();
    }
    let outlines = outline_query.all(&txn).await?;
    if !outlines.is_empty() {
        let mut updated_at_by_id = CaseStatement::new();
        let mut outline_ids = Vec::with_capacity(outlines.len());
        for outline in &outlines {
            updated_at_by_id = updated_at_by_id.case(
                chapter_outline::Column::Id.eq(outline.id),
                super::mutation::next_updated_at(outline.updated_at),
            );
            outline_ids.push(outline.id);
        }
        chapter_outline::Entity::update_many()
            .col_expr(
                chapter_outline::Column::PartId,
                Expr::value(Option::<i64>::None),
            )
            .col_expr(
                chapter_outline::Column::UpdatedAt,
                updated_at_by_id
                    .finally(Expr::col(chapter_outline::Column::UpdatedAt))
                    .into(),
            )
            .filter(chapter_outline::Column::Id.is_in(outline_ids))
            .exec(&txn)
            .await?;
    }

    delete_generation_run_targets_for(&txn, "part_outline", &part_ids).await?;
    part_outline::Entity::delete_many()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .filter(part_outline::Column::Id.is_in(part_ids))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(())
}

pub async fn delete_latest_chapter_outline_records(
    db: &impl crate::DbExecutor,
    input: DeleteLatestOutlineRecordsInput,
) -> Result<(), ProjectOutlineRecordError> {
    validate_delete_input(&input)?;
    let txn = db.begin().await?;
    super::mutation::lock_project_for_write(&txn, input.project_id, "删除章节大纲").await?;

    let mut query = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let outlines = query.all(&txn).await?;
    validate_existing_chapter_outlines(&outlines)?;
    ensure_latest_targets_match(
        outlines
            .iter()
            .rev()
            .take(input.targets.len())
            .map(|outline| (outline.id, outline.updated_at))
            .collect(),
        &input.targets,
        "删除旧章节大纲确认",
    )?;

    let outline_ids = input
        .targets
        .iter()
        .map(|target| target.id)
        .collect::<Vec<_>>();
    chapter::Entity::update_many()
        .col_expr(chapter::Column::OutlineId, Expr::value(Option::<i64>::None))
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .filter(chapter::Column::OutlineId.is_in(outline_ids.iter().copied()))
        .exec(&txn)
        .await?;
    delete_generation_run_targets_for(&txn, "chapter_outline", &outline_ids).await?;
    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .filter(chapter_outline::Column::Id.is_in(outline_ids))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(())
}

fn validate_delete_input(
    input: &DeleteLatestOutlineRecordsInput,
) -> Result<(), ProjectOutlineRecordError> {
    if input.project_id <= 0 {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "项目 ID 必须大于 0".to_string(),
        ));
    }
    if input.targets.is_empty() {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "至少选择一条大纲资料删除".to_string(),
        ));
    }
    if input.targets.len() > 1024 {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "单次最多删除 1024 条大纲资料".to_string(),
        ));
    }
    let mut ids = BTreeSet::new();
    for target in &input.targets {
        if target.id <= 0 {
            return Err(ProjectOutlineRecordError::InvalidArgument(
                "大纲资料 ID 必须大于 0".to_string(),
            ));
        }
        if target.expected_updated_at.trim().is_empty() {
            return Err(ProjectOutlineRecordError::InvalidArgument(
                "缺少大纲资料版本，请刷新后重试".to_string(),
            ));
        }
        if !ids.insert(target.id) {
            return Err(ProjectOutlineRecordError::InvalidArgument(format!(
                "删除目标重复: {}",
                target.id
            )));
        }
    }
    Ok(())
}

fn ensure_latest_targets_match(
    latest: Vec<(i64, NaiveDateTime)>,
    targets: &[OutlineDeleteTarget],
    operation: &str,
) -> Result<(), ProjectOutlineRecordError> {
    if latest.len() != targets.len() {
        return Err(ProjectOutlineRecordError::OutlineSourceChanged {
            operation: operation.to_string(),
        });
    }
    let expected = targets
        .iter()
        .map(|target| (target.id, target.expected_updated_at.trim()))
        .collect::<BTreeMap<_, _>>();
    if latest.iter().any(|(id, updated_at)| {
        let actual_updated_at = updated_at.to_string();
        expected
            .get(id)
            .is_none_or(|expected_updated_at| **expected_updated_at != actual_updated_at)
    }) {
        return Err(ProjectOutlineRecordError::OutlineSourceChanged {
            operation: operation.to_string(),
        });
    }
    Ok(())
}

fn validate_existing_chapter_outlines(
    outlines: &[chapter_outline::Model],
) -> Result<(), ProjectOutlineRecordError> {
    let mut chapter_numbers = BTreeSet::new();
    for outline in outlines {
        if outline.chapter_number <= 0 || !chapter_numbers.insert(outline.chapter_number) {
            return Err(ProjectOutlineRecordError::InvalidArgument(format!(
                "第 {} 章的大纲状态无效，请先清理后再删除",
                outline.chapter_number
            )));
        }
    }
    Ok(())
}
