use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use super::contracts::{
    ManualChapterOutlineSaveInput, ManualPartOutlineSaveInput, ProjectChapterOutlineUpdateError,
    ProjectChapterOutlineUpdateInput, ProjectOutlineRecordError,
};

pub async fn update_chapter_outline_record(
    db: &DatabaseConnection,
    input: ProjectChapterOutlineUpdateInput,
) -> Result<chapter_outline::Model, ProjectChapterOutlineUpdateError> {
    if input.title.is_none()
        && input.summary.is_none()
        && input.beats.is_none()
        && input.key_scenes.is_none()
    {
        return Err(ProjectChapterOutlineUpdateError::InvalidArgument(
            "至少需要更新一个章节大纲字段".to_string(),
        ));
    }
    if input.expected_updated_at.trim().is_empty() {
        return Err(ProjectChapterOutlineUpdateError::InvalidArgument(
            "缺少章节大纲资料版本，请刷新后重试".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let project = crate::project::project_by_id_for_update(&txn, input.project_id)
        .await?
        .ok_or_else(|| {
            ProjectChapterOutlineUpdateError::NotFound(format!("项目不存在: {}", input.project_id))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(ProjectChapterOutlineUpdateError::InvalidState {
            current: project.lifecycle_gate,
            operation: "更新章节大纲".to_string(),
        });
    }

    let mut query = chapter_outline::Entity::find_by_id(input.outline_id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let outline = query.one(&txn).await?.ok_or_else(|| {
        ProjectChapterOutlineUpdateError::NotFound(format!("章节大纲不存在: {}", input.outline_id))
    })?;
    if outline.project_id != input.project_id {
        return Err(ProjectChapterOutlineUpdateError::Forbidden(format!(
            "章节大纲 {} 不属于项目 {}",
            outline.id, input.project_id
        )));
    }
    if outline.updated_at.to_string() != input.expected_updated_at.trim() {
        return Err(ProjectChapterOutlineUpdateError::SourceChanged);
    }

    let updated_at = next_updated_at(outline.updated_at);
    super::provenance::clear_chapter_outline_provenance(&txn, outline.id).await?;
    let mut active: chapter_outline::ActiveModel = outline.into();
    if let Some(title) = input.title {
        active.title = Set(Some(title));
    }
    if let Some(summary) = input.summary {
        active.summary = Set(Some(summary));
    }
    if let Some(beats) = input.beats {
        active.beats = Set(Some(beats));
    }
    if let Some(key_scenes) = input.key_scenes {
        active.key_scenes = Set(Some(key_scenes));
    }
    active.updated_at = Set(updated_at);

    let updated = active.update(&txn).await?;
    txn.commit().await?;
    Ok(updated)
}

pub async fn save_manual_chapter_outline(
    db: &DatabaseConnection,
    input: ManualChapterOutlineSaveInput,
) -> Result<chapter_outline::Model, ProjectOutlineRecordError> {
    let input = normalize_manual_chapter_input(input)?;
    let txn = db.begin().await?;
    lock_project_for_write(&txn, input.project_id, "保存章节大纲").await?;

    let mut query = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .order_by_asc(chapter_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let outlines = query.all(&txn).await?;
    let existing = input
        .outline_id
        .map(|outline_id| {
            outlines
                .iter()
                .find(|outline| outline.id == outline_id)
                .cloned()
                .ok_or_else(|| {
                    ProjectOutlineRecordError::InvalidArgument(format!(
                        "章节大纲 {outline_id} 不属于项目 {}",
                        input.project_id
                    ))
                })
        })
        .transpose()?;
    if let Some(existing) = &existing {
        require_current_version(
            existing.updated_at,
            input.expected_updated_at.as_deref(),
            "保存旧章节大纲表单",
        )?;
    }
    if let Some(conflict) = outlines.iter().find(|outline| {
        outline.chapter_number == input.chapter_number
            && existing
                .as_ref()
                .is_none_or(|current| current.id != outline.id)
    }) {
        return Err(ProjectOutlineRecordError::InvalidArgument(format!(
            "第 {} 章已有章节大纲 {}，请编辑现有记录",
            input.chapter_number, conflict.id
        )));
    }

    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(&txn)
        .await?;
    let part_id = crate::outline::part_id_for_chapter_number(&parts, input.chapter_number)
        .map_err(map_outline_error)?;

    let outline = match existing {
        Some(existing) => {
            if existing.chapter_number != input.chapter_number {
                chapter::Entity::update_many()
                    .col_expr(chapter::Column::OutlineId, Expr::value(Option::<i64>::None))
                    .filter(chapter::Column::ProjectId.eq(input.project_id))
                    .filter(chapter::Column::OutlineId.eq(existing.id))
                    .exec(&txn)
                    .await?;
            }
            let outline_id = existing.id;
            let updated_at = next_updated_at(existing.updated_at);
            let mut active: chapter_outline::ActiveModel = existing.into();
            active.part_id = Set(part_id);
            active.chapter_number = Set(input.chapter_number);
            active.title = Set(input.title);
            active.summary = Set(input.summary);
            active.status = Set(chapter_outline::STATUS_DRAFT.to_string());
            active.updated_at = Set(updated_at);
            super::provenance::clear_chapter_outline_provenance(&txn, outline_id).await?;
            active.update(&txn).await?
        }
        None => {
            let now = Utc::now().naive_utc();
            chapter_outline::ActiveModel {
                project_id: Set(input.project_id),
                part_id: Set(part_id),
                chapter_number: Set(input.chapter_number),
                title: Set(input.title),
                summary: Set(input.summary),
                beats: Set(None),
                key_scenes: Set(None),
                pacing_score: Set(None),
                foreshadowing_setup: Set(None),
                foreshadowing_payoff: Set(None),
                characters_involved: Set(None),
                word_count_target: Set(None),
                status: Set(chapter_outline::STATUS_DRAFT.to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?
        }
    };
    super::provenance::link_chapter_to_outline_if_needed(
        &txn,
        input.project_id,
        input.chapter_number,
        outline.id,
    )
    .await?;
    txn.commit().await?;
    Ok(outline)
}

pub async fn save_manual_part_outline(
    db: &DatabaseConnection,
    input: ManualPartOutlineSaveInput,
) -> Result<part_outline::Model, ProjectOutlineRecordError> {
    let input = normalize_manual_part_input(input)?;
    let txn = db.begin().await?;
    lock_project_for_write(&txn, input.project_id, "保存分部大纲").await?;

    let mut query = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let parts = query.all(&txn).await?;
    let existing = input
        .part_id
        .map(|part_id| {
            parts
                .iter()
                .find(|part| part.id == part_id)
                .cloned()
                .ok_or_else(|| {
                    ProjectOutlineRecordError::InvalidArgument(format!(
                        "分部大纲 {part_id} 不属于项目 {}",
                        input.project_id
                    ))
                })
        })
        .transpose()?;
    if let Some(existing) = &existing {
        require_current_version(
            existing.updated_at,
            input.expected_updated_at.as_deref(),
            "保存旧分部大纲表单",
        )?;
    }
    if let Some(conflict) = parts.iter().find(|part| {
        part.part_number == input.part_number
            && existing
                .as_ref()
                .is_none_or(|current| current.id != part.id)
    }) {
        return Err(ProjectOutlineRecordError::InvalidArgument(format!(
            "第 {} 部已有分部大纲 {}，请编辑现有记录",
            input.part_number, conflict.id
        )));
    }

    let now = Utc::now().naive_utc();
    let updated_at = existing.as_ref().map_or(now, |part| {
        std::cmp::max(now, next_updated_at(part.updated_at))
    });
    let candidate = part_outline::Model {
        id: existing.as_ref().map_or(0, |part| part.id),
        project_id: input.project_id,
        part_number: input.part_number,
        title: input.title.clone().unwrap_or_default(),
        start_chapter: input.start_chapter,
        end_chapter: input.end_chapter,
        summary: input.summary.clone(),
        theme: input.theme.clone(),
        key_events: existing.as_ref().and_then(|part| part.key_events.clone()),
        character_focus: existing
            .as_ref()
            .and_then(|part| part.character_focus.clone()),
        created_at: existing.as_ref().map_or(now, |part| part.created_at),
        updated_at,
    };
    let prospective = parts
        .iter()
        .filter(|part| {
            existing
                .as_ref()
                .is_none_or(|current| current.id != part.id)
        })
        .cloned()
        .chain(std::iter::once(candidate.clone()))
        .collect::<Vec<_>>();
    crate::outline::validate_part_outline_records(&prospective).map_err(map_outline_error)?;

    let saved = match existing {
        Some(existing) => {
            let mut active: part_outline::ActiveModel = existing.into();
            active.part_number = Set(candidate.part_number);
            active.title = Set(candidate.title);
            active.start_chapter = Set(candidate.start_chapter);
            active.end_chapter = Set(candidate.end_chapter);
            active.summary = Set(candidate.summary);
            active.theme = Set(candidate.theme);
            active.updated_at = Set(candidate.updated_at);
            active.update(&txn).await?
        }
        None => {
            part_outline::ActiveModel {
                project_id: Set(candidate.project_id),
                part_number: Set(candidate.part_number),
                title: Set(candidate.title),
                start_chapter: Set(candidate.start_chapter),
                end_chapter: Set(candidate.end_chapter),
                summary: Set(candidate.summary),
                theme: Set(candidate.theme),
                key_events: Set(None),
                character_focus: Set(None),
                created_at: Set(candidate.created_at),
                updated_at: Set(candidate.updated_at),
                ..Default::default()
            }
            .insert(&txn)
            .await?
        }
    };

    let current_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(&txn)
        .await?;
    crate::outline::sync_chapter_outline_part_assignments(
        &txn,
        input.project_id,
        &current_parts,
        Utc::now().naive_utc(),
    )
    .await
    .map_err(map_outline_error)?;
    txn.commit().await?;
    Ok(saved)
}

pub(super) async fn lock_project_for_write<C>(
    db: &C,
    project_id: i64,
    operation: &str,
) -> Result<(), ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    if project_id <= 0 {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "项目 ID 必须大于 0".to_string(),
        ));
    }
    let project = crate::project::project_by_id_for_update(db, project_id)
        .await?
        .ok_or_else(|| {
            ProjectOutlineRecordError::InvalidArgument(format!("项目不存在: {project_id}"))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(ProjectOutlineRecordError::InvalidState {
            current: project.lifecycle_gate,
            operation: operation.to_string(),
        });
    }
    Ok(())
}

fn normalize_manual_chapter_input(
    mut input: ManualChapterOutlineSaveInput,
) -> Result<ManualChapterOutlineSaveInput, ProjectOutlineRecordError> {
    if input.chapter_number <= 0 {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "章节号必须大于 0".to_string(),
        ));
    }
    input.title = normalized_optional_text(input.title);
    input.summary = normalized_optional_text(input.summary);
    if input.title.is_none() && input.summary.is_none() {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "章节标题和章节摘要至少填写一项".to_string(),
        ));
    }
    Ok(input)
}

fn normalize_manual_part_input(
    mut input: ManualPartOutlineSaveInput,
) -> Result<ManualPartOutlineSaveInput, ProjectOutlineRecordError> {
    if input.part_number <= 0 {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "分部编号必须大于 0".to_string(),
        ));
    }
    if input.start_chapter <= 0 || input.end_chapter < input.start_chapter {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "分部章节范围无效".to_string(),
        ));
    }
    input.title = normalized_optional_text(input.title);
    input.summary = normalized_optional_text(input.summary);
    input.theme = normalized_optional_text(input.theme);
    Ok(input)
}

fn normalized_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn require_current_version(
    actual_updated_at: NaiveDateTime,
    expected_updated_at: Option<&str>,
    operation: &str,
) -> Result<(), ProjectOutlineRecordError> {
    let expected = expected_updated_at.unwrap_or_default().trim();
    if expected.is_empty() {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "缺少大纲资料版本，请刷新后重试".to_string(),
        ));
    }
    if actual_updated_at.to_string() != expected {
        return Err(ProjectOutlineRecordError::OutlineSourceChanged {
            operation: operation.to_string(),
        });
    }
    Ok(())
}

pub(super) fn next_updated_at(current: NaiveDateTime) -> NaiveDateTime {
    std::cmp::max(
        Utc::now().naive_utc(),
        current + chrono::Duration::microseconds(1),
    )
}

fn map_outline_error(error: crate::outline::OutlineDbError) -> ProjectOutlineRecordError {
    match error {
        crate::outline::OutlineDbError::Database(error) => {
            ProjectOutlineRecordError::Database(error)
        }
        crate::outline::OutlineDbError::InvalidArgument(message) => {
            ProjectOutlineRecordError::InvalidArgument(message)
        }
        crate::outline::OutlineDbError::InvalidState { current, operation } => {
            ProjectOutlineRecordError::InvalidState { current, operation }
        }
        crate::outline::OutlineDbError::Generation(error) => {
            ProjectOutlineRecordError::InvalidArgument(error.to_string())
        }
    }
}
