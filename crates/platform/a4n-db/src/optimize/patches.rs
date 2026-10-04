use super::*;
use sea_orm::{DatabaseConnection, TransactionTrait};

pub(super) fn optimization_patch_set_record(
    patch_set: optimization_patch_set::Model,
) -> a4n_domain::project::OptimizationPatchSetRecord {
    a4n_domain::project::OptimizationPatchSetRecord {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash,
        status: patch_set.status,
        summary: patch_set.summary,
        created_at: patch_set.created_at,
        updated_at: patch_set.updated_at,
    }
}

fn optimization_patch_record(
    patch: optimization_patch::Model,
) -> a4n_domain::project::OptimizationPatchRecord {
    a4n_domain::project::OptimizationPatchRecord {
        id: patch.id,
        patch_set_id: patch.patch_set_id,
        patch_id: patch.patch_id,
        issue_id: patch.issue_id,
        operation: patch.operation,
        old_text: patch.old_text,
        new_text: patch.new_text,
        reason: patch.reason,
        risk: patch.risk,
        decision: patch.decision,
        apply_status: patch.apply_status,
        created_at: patch.created_at,
        updated_at: patch.updated_at,
    }
}

pub async fn optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
) -> Result<Vec<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .order_by_desc(optimization_patch_set::Column::UpdatedAt)
        .order_by_desc(optimization_patch_set::Column::Id)
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(optimization_patch_set_record)
                .collect()
        })
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_by_turn_id<C>(
    db: &C,
    turn_id: i64,
) -> Result<Option<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::TurnId.eq(turn_id))
        .order_by_desc(optimization_patch_set::Column::Id)
        .one(db)
        .await
        .map(|row| row.map(optimization_patch_set_record))
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_by_id<C>(
    db: &C,
    patch_set_id: i64,
) -> Result<Option<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find_by_id(patch_set_id)
        .one(db)
        .await
        .map(|row| row.map(optimization_patch_set_record))
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_patches<C>(
    db: &C,
    patch_set_id: i64,
) -> Result<Vec<a4n_domain::project::OptimizationPatchRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch::Entity::find()
        .filter(optimization_patch::Column::PatchSetId.eq(patch_set_id))
        .order_by_asc(optimization_patch::Column::Id)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(optimization_patch_record).collect())
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_patches_for_sets<C>(
    db: &C,
    patch_set_ids: &[i64],
) -> Result<Vec<a4n_domain::project::OptimizationPatchRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    if patch_set_ids.is_empty() {
        return Ok(Vec::new());
    }
    optimization_patch::Entity::find()
        .filter(optimization_patch::Column::PatchSetId.is_in(patch_set_ids.to_vec()))
        .order_by_asc(optimization_patch::Column::PatchSetId)
        .order_by_asc(optimization_patch::Column::Id)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(optimization_patch_record).collect())
        .map_err(OptimizeDbError::Database)
}

pub async fn matching_pending_optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
    base_content_hash: &str,
    pending_status: &str,
) -> Result<Vec<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .filter(optimization_patch_set::Column::BaseContentHash.eq(base_content_hash))
        .filter(optimization_patch_set::Column::Status.eq(pending_status))
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(optimization_patch_set_record)
                .collect()
        })
        .map_err(OptimizeDbError::Database)
}

pub async fn pending_optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
    pending_status: &str,
) -> Result<Vec<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .filter(optimization_patch_set::Column::Status.eq(pending_status))
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(optimization_patch_set_record)
                .collect()
        })
        .map_err(OptimizeDbError::Database)
}

pub async fn discard_pending_optimization_patch_sets_and_record_event(
    db: &DatabaseConnection,
    session_id: i64,
) -> Result<(), OptimizeDbError> {
    if session_id <= 0 {
        return Err(OptimizeDbError::InvalidArgument(
            "session_id 必须大于 0".to_string(),
        ));
    }
    let tx = db.begin().await?;
    let patch_sets = pending_optimization_patch_sets(
        &tx,
        session_id,
        a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_PENDING,
    )
    .await?;
    let patch_set_ids = patch_sets
        .iter()
        .map(|patch_set| patch_set.id)
        .collect::<Vec<_>>();
    update_optimization_patch_sets_status(
        &tx,
        &patch_set_ids,
        a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_REJECTED,
        a4n_domain::project::OPTIMIZATION_PATCH_DECISION_REJECTED,
        a4n_domain::project::OPTIMIZATION_PATCH_APPLY_STATUS_PENDING,
    )
    .await?;
    insert_optimization_session_message(
        &tx,
        OptimizationSessionMessageInsertInput {
            session_id,
            role: "agent_event".to_string(),
            content: "用户已丢弃当前待应用局部补丁。".to_string(),
            tool_calls: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn discard_optimization_patch_set_and_record_event(
    db: &DatabaseConnection,
    session_id: i64,
    patch_set_id: i64,
) -> Result<(), OptimizeDbError> {
    if session_id <= 0 || patch_set_id <= 0 {
        return Err(OptimizeDbError::InvalidArgument(
            "session_id 和 patch_set_id 必须大于 0".to_string(),
        ));
    }
    let tx = db.begin().await?;
    let patch_set = optimization_patch_set_by_id(&tx, patch_set_id)
        .await?
        .ok_or_else(|| OptimizeDbError::NotFound(format!("优化补丁集不存在: {patch_set_id}")))?;
    if patch_set.session_id != session_id {
        return Err(OptimizeDbError::Forbidden(
            "优化补丁集不属于当前优化会话".into(),
        ));
    }
    if patch_set.status != a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_PENDING {
        return Err(OptimizeDbError::InvalidState {
            current: patch_set.status,
            operation: "拒绝优化补丁集".into(),
        });
    }
    update_optimization_patch_sets_status(
        &tx,
        &[patch_set_id],
        a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_REJECTED,
        a4n_domain::project::OPTIMIZATION_PATCH_DECISION_REJECTED,
        a4n_domain::project::OPTIMIZATION_PATCH_APPLY_STATUS_PENDING,
    )
    .await?;
    insert_optimization_session_message(
        &tx,
        OptimizationSessionMessageInsertInput {
            session_id,
            role: "agent_event".to_string(),
            content: "用户已拒绝当前正文补丁。".to_string(),
            tool_calls: None,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn create_optimization_patch_set<C>(
    db: &C,
    input: OptimizationPatchSetCreateInput,
) -> Result<Option<a4n_domain::project::OptimizationPatchSetRecord>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    if input.patches.is_empty() {
        return Ok(None);
    }

    let now = chrono::Utc::now().naive_utc();
    let patch_set = optimization_patch_set::ActiveModel {
        turn_id: Set(input.turn_id),
        session_id: Set(input.session_id),
        chapter_id: Set(input.chapter_id),
        base_content_hash: Set(input.base_content_hash),
        status: Set(input.status),
        summary: Set(input.summary),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(OptimizeDbError::Database)?;

    let patches = input
        .patches
        .into_iter()
        .map(|patch| optimization_patch::ActiveModel {
            patch_set_id: Set(patch_set.id),
            patch_id: Set(patch.patch_id),
            issue_id: Set(patch.issue_id),
            operation: Set(patch.operation),
            old_text: Set(patch.old_text),
            new_text: Set(patch.new_text),
            reason: Set(patch.reason),
            risk: Set(patch.risk),
            decision: Set(input.patch_decision.clone()),
            apply_status: Set(input.patch_apply_status.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    optimization_patch::Entity::insert_many(patches)
        .exec(db)
        .await
        .map_err(OptimizeDbError::Database)?;

    Ok(Some(optimization_patch_set_record(patch_set)))
}

pub async fn update_optimization_patch_sets_status<C>(
    db: &C,
    patch_set_ids: &[i64],
    patch_set_status: &str,
    patch_decision: &str,
    patch_apply_status: &str,
) -> Result<(), OptimizeDbError>
where
    C: ConnectionTrait,
{
    if patch_set_ids.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().naive_utc();
    optimization_patch::Entity::update_many()
        .col_expr(
            optimization_patch::Column::Decision,
            sea_orm::sea_query::Expr::value(patch_decision.to_string()),
        )
        .col_expr(
            optimization_patch::Column::ApplyStatus,
            sea_orm::sea_query::Expr::value(patch_apply_status.to_string()),
        )
        .col_expr(
            optimization_patch::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(optimization_patch::Column::PatchSetId.is_in(patch_set_ids.to_vec()))
        .exec(db)
        .await
        .map_err(OptimizeDbError::Database)?;

    let updated = optimization_patch_set::Entity::update_many()
        .col_expr(
            optimization_patch_set::Column::Status,
            sea_orm::sea_query::Expr::value(patch_set_status.to_string()),
        )
        .col_expr(
            optimization_patch_set::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(optimization_patch_set::Column::Id.is_in(patch_set_ids.to_vec()))
        .exec(db)
        .await
        .map_err(OptimizeDbError::Database)?;
    if updated.rows_affected != patch_set_ids.len() as u64 {
        return Err(OptimizeDbError::Database(DbErr::RecordNotFound(
            "批量更新优化补丁集时记录数量不匹配".to_string(),
        )));
    }
    Ok(())
}
