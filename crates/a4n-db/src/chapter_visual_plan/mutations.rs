use a4n_core::models::{
    chapter::chapter_visual_plan,
    project::{novel_project, source_snapshot_chapter},
};
use a4n_core::source_spans::source_spans_for_content;
use a4n_domain::chapter_visual_plan::{CHAPTER_VISUAL_PLAN_V1, ChapterVisualPlanStatus};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use super::{
    codec::{parse_record, serialize_plan},
    contracts::{
        ChapterVisualPlanDbError, ConfirmChapterVisualPlanInput, ConfirmChapterVisualPlanOutcome,
        DiscardChapterVisualPlanDraftInput, ParsedChapterVisualPlan,
        SaveChapterVisualPlanDraftInput,
    },
    queries::{chapter_visual_plan_by_id, validate_source},
};

pub async fn save_chapter_visual_plan_draft(
    db: &sea_orm::DatabaseConnection,
    input: SaveChapterVisualPlanDraftInput,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError> {
    validate_source(&input.source)?;
    if input.author_user_id <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "author_user_id 必须大于 0".into(),
        ));
    }
    let source_span_ids = source_spans_for_content(
        input.source.source_chapter_id,
        input.source.chapter_number,
        &input.source.content,
    )
    .into_iter()
    .map(|span| span.id)
    .collect::<Vec<_>>();
    input
        .plan
        .validate_source_span_coverage(&source_span_ids)
        .map_err(|error| ChapterVisualPlanDbError::InvalidArgument(error.to_string()))?;
    let (plan_json, plan_sha256) = serialize_plan(&input.plan)?;
    let tx = db.begin().await?;
    let existing = chapter_visual_plan::Entity::find()
        .filter(chapter_visual_plan::Column::RootProjectId.eq(input.source.root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.eq(input.source.source_chapter_id))
        .filter(
            chapter_visual_plan::Column::SourceRevisionKey.eq(&input.source.source_revision_key),
        )
        .filter(chapter_visual_plan::Column::SourceContentSha256.eq(&input.source.content_sha256))
        .filter(chapter_visual_plan::Column::Status.eq("draft"))
        .one(&tx)
        .await?;
    let result = if let Some(existing) = existing {
        let expected = input
            .expected
            .ok_or(ChapterVisualPlanDbError::RevisionConflict {
                record_id: existing.id,
                expected_revision: existing.revision,
            })?;
        if expected.record_id != existing.id || expected.revision != existing.revision {
            return Err(ChapterVisualPlanDbError::RevisionConflict {
                record_id: expected.record_id,
                expected_revision: expected.revision,
            });
        }
        if existing.plan_sha256 == plan_sha256
            && existing.authoring_mode == input.authoring_mode.as_str()
        {
            parse_record(existing)?
        } else {
            let next_revision = existing.revision.checked_add(1).ok_or_else(|| {
                ChapterVisualPlanDbError::InvalidState("章节视觉编排 revision 超出 i32 范围".into())
            })?;
            let updated = chapter_visual_plan::Entity::update_many()
                .col_expr(
                    chapter_visual_plan::Column::Revision,
                    Expr::value(next_revision),
                )
                .col_expr(
                    chapter_visual_plan::Column::AuthoringMode,
                    Expr::value(input.authoring_mode.as_str()),
                )
                .col_expr(
                    chapter_visual_plan::Column::PlanSha256,
                    Expr::value(plan_sha256),
                )
                .col_expr(
                    chapter_visual_plan::Column::PlanJson,
                    Expr::value(plan_json),
                )
                .col_expr(
                    chapter_visual_plan::Column::AuthorUserId,
                    Expr::value(input.author_user_id),
                )
                .col_expr(
                    chapter_visual_plan::Column::UpdatedAt,
                    Expr::value(Utc::now().naive_utc()),
                )
                .filter(chapter_visual_plan::Column::Id.eq(existing.id))
                .filter(chapter_visual_plan::Column::Revision.eq(existing.revision))
                .filter(chapter_visual_plan::Column::Status.eq("draft"))
                .exec(&tx)
                .await?;
            if updated.rows_affected != 1 {
                return Err(ChapterVisualPlanDbError::RevisionConflict {
                    record_id: existing.id,
                    expected_revision: existing.revision,
                });
            }
            chapter_visual_plan_by_id(&tx, existing.id).await?
        }
    } else {
        if let Some(expected) = input.expected {
            return Err(ChapterVisualPlanDbError::RevisionConflict {
                record_id: expected.record_id,
                expected_revision: expected.revision,
            });
        }
        if input.source.source_snapshot_id.is_none() {
            chapter_visual_plan::Entity::update_many()
                .col_expr(chapter_visual_plan::Column::Status, Expr::value("stale"))
                .col_expr(
                    chapter_visual_plan::Column::UpdatedAt,
                    Expr::value(Utc::now().naive_utc()),
                )
                .filter(chapter_visual_plan::Column::RootProjectId.eq(input.source.root_project_id))
                .filter(
                    chapter_visual_plan::Column::SourceChapterId.eq(input.source.source_chapter_id),
                )
                .filter(chapter_visual_plan::Column::SourceSnapshotId.is_null())
                .filter(
                    chapter_visual_plan::Column::SourceRevisionKey
                        .ne(&input.source.source_revision_key),
                )
                .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
                .exec(&tx)
                .await?;
        }
        let latest = chapter_visual_plan::Entity::find()
            .filter(chapter_visual_plan::Column::RootProjectId.eq(input.source.root_project_id))
            .filter(chapter_visual_plan::Column::SourceChapterId.eq(input.source.source_chapter_id))
            .order_by_desc(chapter_visual_plan::Column::PlanVersion)
            .one(&tx)
            .await?;
        let plan_version = match latest {
            Some(record) => record.plan_version.checked_add(1).ok_or_else(|| {
                ChapterVisualPlanDbError::InvalidState("章节视觉编排版本超出 i32 范围".into())
            })?,
            None => 1,
        };
        let now = Utc::now().naive_utc();
        let record = chapter_visual_plan::ActiveModel {
            id: NotSet,
            root_project_id: Set(input.source.root_project_id),
            source_chapter_id: Set(input.source.source_chapter_id),
            source_snapshot_id: Set(input.source.source_snapshot_id),
            source_snapshot_chapter_id: Set(input.source.source_snapshot_chapter_id),
            source_revision_key: Set(input.source.source_revision_key),
            source_content_sha256: Set(input.source.content_sha256),
            contract_version: Set(CHAPTER_VISUAL_PLAN_V1.into()),
            plan_version: Set(plan_version),
            revision: Set(1),
            status: Set(ChapterVisualPlanStatus::Draft.as_str().into()),
            authoring_mode: Set(input.authoring_mode.as_str().into()),
            plan_sha256: Set(plan_sha256),
            plan_json: Set(plan_json),
            author_user_id: Set(input.author_user_id),
            confirmed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&tx)
        .await?;
        parse_record(record)?
    };
    tx.commit().await?;
    Ok(result)
}

pub async fn confirm_chapter_visual_plan(
    db: &sea_orm::DatabaseConnection,
    input: ConfirmChapterVisualPlanInput,
) -> Result<ConfirmChapterVisualPlanOutcome, ChapterVisualPlanDbError> {
    if input.record_id <= 0 || input.expected_revision <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "record_id 和 expected_revision 必须大于 0".into(),
        ));
    }
    let tx = db.begin().await?;
    let current = chapter_visual_plan_by_id(&tx, input.record_id).await?;
    if current.record.revision != input.expected_revision {
        return Err(ChapterVisualPlanDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }
    if current.status == ChapterVisualPlanStatus::Confirmed {
        tx.commit().await?;
        return Ok(ConfirmChapterVisualPlanOutcome {
            visual_plan: current,
            invalidated_project_ids: Vec::new(),
        });
    }
    if current.status != ChapterVisualPlanStatus::Draft {
        return Err(ChapterVisualPlanDbError::InvalidState(format!(
            "只有 draft 可以确认，当前状态为 {}",
            current.record.status
        )));
    }
    chapter_visual_plan::Entity::update_many()
        .col_expr(
            chapter_visual_plan::Column::Status,
            Expr::value("superseded"),
        )
        .col_expr(
            chapter_visual_plan::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(chapter_visual_plan::Column::RootProjectId.eq(current.record.root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.eq(current.record.source_chapter_id))
        .filter(
            chapter_visual_plan::Column::SourceRevisionKey.eq(&current.record.source_revision_key),
        )
        .filter(chapter_visual_plan::Column::Status.eq("confirmed"))
        .filter(chapter_visual_plan::Column::Id.ne(current.record.id))
        .exec(&tx)
        .await?;
    let next_revision = current.record.revision.checked_add(1).ok_or_else(|| {
        ChapterVisualPlanDbError::InvalidState("章节视觉编排 revision 超出 i32 范围".into())
    })?;
    let now = Utc::now().naive_utc();
    let updated = chapter_visual_plan::Entity::update_many()
        .col_expr(
            chapter_visual_plan::Column::Status,
            Expr::value("confirmed"),
        )
        .col_expr(
            chapter_visual_plan::Column::Revision,
            Expr::value(next_revision),
        )
        .col_expr(chapter_visual_plan::Column::ConfirmedAt, Expr::value(now))
        .col_expr(chapter_visual_plan::Column::UpdatedAt, Expr::value(now))
        .filter(chapter_visual_plan::Column::Id.eq(current.record.id))
        .filter(chapter_visual_plan::Column::Revision.eq(current.record.revision))
        .filter(chapter_visual_plan::Column::Status.eq("draft"))
        .exec(&tx)
        .await?;
    if updated.rows_affected != 1 {
        return Err(ChapterVisualPlanDbError::RevisionConflict {
            record_id: current.record.id,
            expected_revision: current.record.revision,
        });
    }
    if let Some(snapshot_chapter_id) = current.record.source_snapshot_chapter_id {
        source_snapshot_chapter::Entity::update_many()
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanId,
                Expr::value(current.record.id),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanVersion,
                Expr::value(current.record.plan_version),
            )
            .col_expr(
                source_snapshot_chapter::Column::VisualPlanSha256,
                Expr::value(current.record.plan_sha256.clone()),
            )
            .filter(source_snapshot_chapter::Column::Id.eq(snapshot_chapter_id))
            .exec(&tx)
            .await?;
    }
    let invalidated_project_ids = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .filter(novel_project::Column::RootProjectId.eq(current.record.root_project_id))
        .filter(novel_project::Column::ProjectKind.is_in(["comic", "video", "galgame"]))
        .into_tuple::<i64>()
        .all(&tx)
        .await?;
    crate::cleanup_outbox::insert_media_workflow_v2_reset_intents(
        &tx,
        invalidated_project_ids.iter().map(|project_id| {
            (
                format!(
                    "visual-plan-confirm:{}:version:{}:project:{}",
                    current.record.id, current.record.plan_version, project_id
                ),
                crate::cleanup_outbox::MediaWorkflowV2ResetPayload::new(*project_id),
            )
        }),
    )
    .await?;
    let visual_plan = chapter_visual_plan_by_id(&tx, current.record.id).await?;
    tx.commit().await?;
    Ok(ConfirmChapterVisualPlanOutcome {
        visual_plan,
        invalidated_project_ids,
    })
}

pub async fn discard_chapter_visual_plan_draft(
    db: &sea_orm::DatabaseConnection,
    input: DiscardChapterVisualPlanDraftInput,
) -> Result<(), ChapterVisualPlanDbError> {
    if input.record_id <= 0 || input.expected_revision <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "record_id 和 expected_revision 必须大于 0".into(),
        ));
    }
    let deleted = chapter_visual_plan::Entity::delete_many()
        .filter(chapter_visual_plan::Column::Id.eq(input.record_id))
        .filter(chapter_visual_plan::Column::Revision.eq(input.expected_revision))
        .filter(chapter_visual_plan::Column::Status.eq("draft"))
        .exec(db)
        .await?;
    if deleted.rows_affected != 1 {
        return Err(ChapterVisualPlanDbError::RevisionConflict {
            record_id: input.record_id,
            expected_revision: input.expected_revision,
        });
    }
    Ok(())
}

pub async fn invalidate_live_chapter_visual_plans_for_source_change<C>(
    db: &C,
    root_project_id: i64,
    source_chapter_id: i64,
) -> Result<u64, ChapterVisualPlanDbError>
where
    C: sea_orm::ConnectionTrait,
{
    if root_project_id <= 0 || source_chapter_id <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "root_project_id 和 source_chapter_id 必须大于 0".into(),
        ));
    }
    Ok(chapter_visual_plan::Entity::update_many()
        .col_expr(chapter_visual_plan::Column::Status, Expr::value("stale"))
        .col_expr(
            chapter_visual_plan::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.eq(source_chapter_id))
        .filter(chapter_visual_plan::Column::SourceSnapshotId.is_null())
        .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
        .exec(db)
        .await?
        .rows_affected)
}
