use a4n_db_models::models::chapter::{chapter, chapter_outline, chapter_outline_provenance};
use a4n_domain::chapter::ChapterSourceSnapshot;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbBackend, EntityTrait, QueryFilter, QuerySelect, Set,
};
use std::collections::HashMap;

use super::contracts::{
    ProjectOutlineRecordError, RetrospectiveChapterOutlineAction,
    RetrospectiveChapterOutlineUpsertInput, RetrospectiveChapterOutlineUpsertOutcome,
    chapter_source_snapshot_matches, chapter_source_snapshot_targets,
};

use chapter_outline_provenance::SOURCE_KIND_IMPORT_ANALYSIS;
/// 章节分析回顾性章纲的唯一来源标识。
use chapter_outline_provenance::SOURCE_KIND_RETROSPECTIVE_ANALYSIS;

/// 批量读取章纲自动来源，避免列表页对每一条章纲发起单独查询。
pub async fn chapter_outline_source_kinds_for_ids<C>(
    db: &C,
    outline_ids: &[i64],
) -> Result<HashMap<i64, String>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    if outline_ids.is_empty() {
        return Ok(HashMap::new());
    }
    chapter_outline_provenance::Entity::find()
        .filter(chapter_outline_provenance::Column::OutlineId.is_in(outline_ids.to_vec()))
        .all(db)
        .await
        .map(|records| {
            records
                .into_iter()
                .map(|record| (record.outline_id, record.source_kind))
                .collect()
        })
        .map_err(ProjectOutlineRecordError::Database)
}

/// 带章节正文版本校验的回顾性章纲写入。
///
/// 该入口用于章节分析完成后的派生同步。它会在同一事务内锁定来源章节，
/// 再比较章节身份、标题、正文哈希和更新时间；任何一项变化都拒绝写入。
pub async fn upsert_retrospective_chapter_outline_checked(
    db: &impl crate::DbExecutor,
    input: RetrospectiveChapterOutlineUpsertInput,
    source: ChapterSourceSnapshot,
) -> Result<RetrospectiveChapterOutlineUpsertOutcome, ProjectOutlineRecordError> {
    upsert_automatic_chapter_outline_checked(db, input, source, SOURCE_KIND_RETROSPECTIVE_ANALYSIS)
        .await
}

pub(crate) async fn upsert_automatic_chapter_outline_checked(
    db: &impl crate::DbExecutor,
    input: RetrospectiveChapterOutlineUpsertInput,
    source: ChapterSourceSnapshot,
    source_kind: &str,
) -> Result<RetrospectiveChapterOutlineUpsertOutcome, ProjectOutlineRecordError> {
    if !chapter_source_snapshot_targets(&source, input.project_id, input.chapter_number) {
        return Err(ProjectOutlineRecordError::InvalidArgument(
            "回顾性章纲来源章节与写入目标不一致".to_string(),
        ));
    }
    let txn = db.begin().await?;
    let project = crate::project::project_write_identity_for_update(&txn, input.project_id)
        .await?
        .ok_or_else(|| {
            ProjectOutlineRecordError::InvalidArgument(format!("项目不存在: {}", input.project_id))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(ProjectOutlineRecordError::InvalidState {
            current: project.lifecycle_gate,
            operation: "保存回顾性章节大纲".to_string(),
        });
    }
    lock_and_validate_chapter_source(&txn, &source).await?;
    let result = upsert_automatic_chapter_outline_in(&txn, input, source_kind).await?;
    txn.commit().await?;
    Ok(result)
}

async fn upsert_automatic_chapter_outline_in<C>(
    db: &C,
    input: RetrospectiveChapterOutlineUpsertInput,
    source_kind: &str,
) -> Result<RetrospectiveChapterOutlineUpsertOutcome, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(input.chapter_number))
        .one(db)
        .await?;

    let Some(existing) = existing else {
        let now = Utc::now().naive_utc();
        let inserted = chapter_outline::ActiveModel {
            project_id: Set(input.project_id),
            part_id: Set(None),
            chapter_number: Set(input.chapter_number),
            title: Set(Some(input.title)),
            summary: Set(Some(input.summary)),
            beats: Set(None),
            key_scenes: Set(input.key_scenes),
            pacing_score: Set(None),
            foreshadowing_setup: Set(input.foreshadowing_setup),
            foreshadowing_payoff: Set(input.foreshadowing_payoff),
            characters_involved: Set(input.characters_involved),
            word_count_target: Set(Some(input.word_count_target)),
            status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;

        chapter_outline_provenance::ActiveModel {
            outline_id: Set(inserted.id),
            source_kind: Set(source_kind.to_string()),
            source_fingerprint_json: Set(input.source_fingerprint_json),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await?;

        link_chapter_to_outline_if_needed(db, input.project_id, input.chapter_number, inserted.id)
            .await?;
        return Ok(RetrospectiveChapterOutlineUpsertOutcome {
            outline: inserted,
            action: RetrospectiveChapterOutlineAction::Created,
        });
    };

    // 即使章纲由用户拥有，也允许在写入边界修复历史上的脏软关联；
    // helper 不会覆盖指向同项目同章节的有效关联。
    link_chapter_to_outline_if_needed(db, input.project_id, input.chapter_number, existing.id)
        .await?;
    let provenance = chapter_outline_provenance::Entity::find_by_id(existing.id)
        .one(db)
        .await?;
    let Some(provenance) = provenance else {
        return Ok(RetrospectiveChapterOutlineUpsertOutcome {
            outline: existing,
            action: RetrospectiveChapterOutlineAction::Preserved,
        });
    };
    if provenance.source_kind != source_kind {
        return Ok(RetrospectiveChapterOutlineUpsertOutcome {
            outline: existing,
            action: RetrospectiveChapterOutlineAction::Preserved,
        });
    }
    if provenance.source_fingerprint_json == input.source_fingerprint_json {
        return Ok(RetrospectiveChapterOutlineUpsertOutcome {
            outline: existing,
            action: RetrospectiveChapterOutlineAction::Unchanged,
        });
    }

    let now = std::cmp::max(
        Utc::now().naive_utc(),
        existing.updated_at + chrono::Duration::microseconds(1),
    );
    let mut active: chapter_outline::ActiveModel = existing.into();
    // part_id 由用户/正式大纲流程管理，回顾性刷新不能丢失用户的分部归属。
    active.title = Set(Some(input.title));
    active.summary = Set(Some(input.summary));
    active.beats = Set(None);
    active.key_scenes = Set(input.key_scenes);
    active.pacing_score = Set(None);
    active.foreshadowing_setup = Set(input.foreshadowing_setup);
    active.foreshadowing_payoff = Set(input.foreshadowing_payoff);
    active.characters_involved = Set(input.characters_involved);
    active.word_count_target = Set(Some(input.word_count_target));
    active.status = Set(chapter_outline::STATUS_COMPLETED.to_string());
    active.updated_at = Set(now);
    let updated = active.update(db).await?;

    let mut provenance_active: chapter_outline_provenance::ActiveModel = provenance.into();
    provenance_active.source_fingerprint_json = Set(input.source_fingerprint_json);
    provenance_active.updated_at = Set(now);
    provenance_active.update(db).await?;

    Ok(RetrospectiveChapterOutlineUpsertOutcome {
        outline: updated,
        action: RetrospectiveChapterOutlineAction::Updated,
    })
}

fn chapter_source_query(
    source: &ChapterSourceSnapshot,
    backend: DbBackend,
) -> sea_orm::Select<chapter::Entity> {
    let mut query = chapter::Entity::find_by_id(source.chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .filter(chapter::Column::ProjectId.eq(source.project_id));
    if backend != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query
}

async fn lock_and_validate_chapter_source<C>(
    db: &C,
    source: &ChapterSourceSnapshot,
) -> Result<chapter::Model, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let actual = chapter_source_query(source, db.get_database_backend())
        .one(db)
        .await?;
    let Some(actual) = actual else {
        return Err(ProjectOutlineRecordError::ChapterSourceChanged {
            chapter_id: source.chapter_id,
        });
    };
    if !chapter_source_snapshot_matches(source, &actual) {
        return Err(ProjectOutlineRecordError::ChapterSourceChanged {
            chapter_id: source.chapter_id,
        });
    }
    Ok(actual)
}

/// 用户编辑或正式大纲流程写入后，清除自动来源标记。
///
/// 正式写入调用方应在同一事务内完成来源清理和业务更新，避免留下半次接管。
pub(crate) async fn clear_chapter_outline_provenance<C>(
    db: &C,
    outline_id: i64,
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    chapter_outline_provenance::Entity::delete_by_id(outline_id)
        .exec(db)
        .await
        .map(|_| ())
}

pub(crate) async fn clear_chapter_outline_provenance_many<C>(
    db: &C,
    outline_ids: &[i64],
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    if outline_ids.is_empty() {
        return Ok(());
    }
    chapter_outline_provenance::Entity::delete_many()
        .filter(chapter_outline_provenance::Column::OutlineId.is_in(outline_ids.to_vec()))
        .exec(db)
        .await
        .map(|_| ())
}

/// 正文变化时删除仍由分析拥有的旧章纲；用户接管后的章纲没有 provenance，始终保留。
pub async fn delete_automatic_chapter_outlines_for_source_change<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<i64>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .all(db)
        .await?;
    if outlines.is_empty() {
        return Ok(Vec::new());
    }
    let source_kinds = chapter_outline_provenance::Entity::find()
        .filter(
            chapter_outline_provenance::Column::OutlineId
                .is_in(outlines.iter().map(|outline| outline.id)),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|provenance| (provenance.outline_id, provenance.source_kind))
        .collect::<HashMap<_, _>>();
    let mut automatic_ids = Vec::new();
    for outline in outlines {
        if source_kinds.get(&outline.id).is_some_and(|source_kind| {
            matches!(
                source_kind.as_str(),
                SOURCE_KIND_RETROSPECTIVE_ANALYSIS | SOURCE_KIND_IMPORT_ANALYSIS
            )
        }) {
            automatic_ids.push(outline.id);
        }
    }
    if automatic_ids.is_empty() {
        return Ok(automatic_ids);
    }

    chapter::Entity::update_many()
        .col_expr(chapter::Column::OutlineId, Expr::value(Option::<i64>::None))
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .filter(chapter::Column::OutlineId.is_in(automatic_ids.clone()))
        .exec(db)
        .await?;
    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::Id.is_in(automatic_ids.clone()))
        .exec(db)
        .await?;
    Ok(automatic_ids)
}

/// 在不覆盖有效用户关联的前提下，修复章节到章纲的软关联。
///
/// 历史数据可能把 `chapter.outline_id` 留成已删除、跨项目或不同章节的
/// ID。只有这些无效关联（或 NULL）才会被替换；指向同一项目同一章节的
/// 现有关联视为用户拥有并保留。更新时再次带上旧值条件，避免并发用户
/// 编辑被自动回链覆盖。
pub(in crate::project::outline_records) async fn link_chapter_to_outline_if_needed<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    outline_id: i64,
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let Some((target_project_id, target_chapter_number)) =
        chapter_outline::Entity::find_by_id(outline_id)
            .select_only()
            .column(chapter_outline::Column::ProjectId)
            .column(chapter_outline::Column::ChapterNumber)
            .into_tuple::<(i64, i32)>()
            .one(db)
            .await?
    else {
        return Ok(());
    };
    if target_project_id != project_id || target_chapter_number != chapter_number {
        return Ok(());
    }

    let Some((chapter_id, current_outline_id)) = chapter::Entity::find_current()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::OutlineId)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .into_tuple::<(i64, Option<i64>)>()
        .one(db)
        .await?
    else {
        return Ok(());
    };

    let should_link = match current_outline_id {
        None => true,
        Some(current_outline_id) => !chapter_outline::Entity::find_by_id(current_outline_id)
            .select_only()
            .column(chapter_outline::Column::ProjectId)
            .column(chapter_outline::Column::ChapterNumber)
            .into_tuple::<(i64, i32)>()
            .one(db)
            .await?
            .is_some_and(|(current_project_id, current_chapter_number)| {
                current_project_id == project_id && current_chapter_number == chapter_number
            }),
    };
    if !should_link {
        return Ok(());
    }

    let mut update = chapter::Entity::update_many()
        .col_expr(chapter::Column::OutlineId, Expr::value(Some(outline_id)))
        .filter(chapter::Column::Id.eq(chapter_id));
    if let Some(current_outline_id) = current_outline_id {
        update = update.filter(chapter::Column::OutlineId.eq(current_outline_id));
    } else {
        update = update.filter(chapter::Column::OutlineId.is_null());
    }
    update.exec(db).await.map(|_| ())
}

/// 按项目和章节号查找章纲，并在需要时修复正文章节的软关联。
/// 返回修复后数据库中实际保留的 outline_id；没有对应章纲或章节时返回 None。
pub(crate) async fn link_chapter_to_project_outline_if_needed<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<i64>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .select_only()
        .column(chapter_outline::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?;
    if let Some(outline_id) = outline {
        link_chapter_to_outline_if_needed(db, project_id, chapter_number, outline_id).await?;
    } else if let Some((chapter_id, outline_id)) = chapter::Entity::find_current()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::OutlineId)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .into_tuple::<(i64, Option<i64>)>()
        .one(db)
        .await?
    {
        clear_invalid_chapter_outline_link(db, chapter_id, project_id, chapter_number, outline_id)
            .await?;
    }

    Ok(chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .select_only()
        .column(chapter::Column::OutlineId)
        .into_tuple::<Option<i64>>()
        .one(db)
        .await?
        .flatten())
}

/// 清理历史上遗留的已删除、跨项目或跨章节软关联。
async fn clear_invalid_chapter_outline_link<C>(
    db: &C,
    chapter_id: i64,
    project_id: i64,
    chapter_number: i32,
    outline_id: Option<i64>,
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let Some(current_outline_id) = outline_id else {
        return Ok(());
    };
    let valid = chapter_outline::Entity::find_by_id(current_outline_id)
        .select_only()
        .column(chapter_outline::Column::ProjectId)
        .column(chapter_outline::Column::ChapterNumber)
        .into_tuple::<(i64, i32)>()
        .one(db)
        .await?
        .is_some_and(|(outline_project_id, outline_chapter_number)| {
            outline_project_id == project_id && outline_chapter_number == chapter_number
        });
    if valid {
        return Ok(());
    }

    // CAS 条件避免覆盖并发正文编辑刚建立的有效关联。
    chapter::Entity::update_many()
        .col_expr(chapter::Column::OutlineId, Expr::value(Option::<i64>::None))
        .filter(chapter::Column::Id.eq(chapter_id))
        .filter(chapter::Column::OutlineId.eq(current_outline_id))
        .exec(db)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod source_contract_tests {
    use super::chapter_source_query;
    use crate::project::outline_records::contracts::{
        chapter_source_snapshot_matches, chapter_source_snapshot_targets,
    };
    use a4n_db_models::models::chapter::chapter;
    use a4n_domain::chapter::ChapterSourceSnapshot;
    use a4n_domain::source_text_hash;
    use chrono::Utc;
    use sea_orm::{DbBackend, QueryTrait};

    fn source() -> ChapterSourceSnapshot {
        let now = Utc::now().naive_utc();
        ChapterSourceSnapshot {
            project_id: 3,
            chapter_id: 7,
            chapter_number: 2,
            title: Some("第二章".to_string()),
            content_sha256: source_text_hash("正文"),
            updated_at: now,
        }
    }

    #[test]
    fn chapter_source_query_locks_postgres_but_not_sqlite() {
        let source = source();
        let postgres_sql = chapter_source_query(&source, DbBackend::Postgres)
            .build(DbBackend::Postgres)
            .to_string();
        assert!(postgres_sql.contains("FOR UPDATE"));

        let sqlite_sql = chapter_source_query(&source, DbBackend::Sqlite)
            .build(DbBackend::Sqlite)
            .to_string();
        assert!(!sqlite_sql.contains("FOR UPDATE"));
    }

    #[test]
    fn source_snapshot_carries_all_write_guard_fields() {
        let now = Utc::now().naive_utc();
        let chapter = chapter::Model {
            id: 7,
            project_id: 3,
            outline_id: None,
            chapter_number: 2,
            title: Some("第二章".to_string()),
            content: "正文".to_string(),
            word_count: 2,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        };
        let snapshot = crate::project::chapter_source_snapshot_from_model(&chapter);
        assert_eq!(snapshot.chapter_id, chapter.id);
        assert_eq!(snapshot.project_id, chapter.project_id);
        assert_eq!(snapshot.chapter_number, chapter.chapter_number);
        assert_eq!(snapshot.title, chapter.title);
        assert_eq!(snapshot.content_sha256, source_text_hash(&chapter.content));
        assert_eq!(snapshot.updated_at, chapter.updated_at);
        assert!(chapter_source_snapshot_matches(&snapshot, &chapter));

        let mut changed = chapter;
        changed.content.push('改');
        assert!(!chapter_source_snapshot_matches(&snapshot, &changed));
        assert!(chapter_source_snapshot_targets(&snapshot, 3, 2));
        assert!(!chapter_source_snapshot_targets(&snapshot, 99, 2));
        assert!(!chapter_source_snapshot_targets(&snapshot, 3, 9));
    }
}
