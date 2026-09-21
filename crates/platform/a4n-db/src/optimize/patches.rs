use super::*;

pub async fn optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
) -> Result<Vec<optimization_patch_set::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .order_by_desc(optimization_patch_set::Column::UpdatedAt)
        .order_by_desc(optimization_patch_set::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_by_turn_id<C>(
    db: &C,
    turn_id: i64,
) -> Result<Option<optimization_patch_set::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::TurnId.eq(turn_id))
        .order_by_desc(optimization_patch_set::Column::Id)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_by_id<C>(
    db: &C,
    patch_set_id: i64,
) -> Result<Option<optimization_patch_set::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find_by_id(patch_set_id)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_patches<C>(
    db: &C,
    patch_set_id: i64,
) -> Result<Vec<optimization_patch::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch::Entity::find()
        .filter(optimization_patch::Column::PatchSetId.eq(patch_set_id))
        .order_by_asc(optimization_patch::Column::Id)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_patch_set_patches_for_sets<C>(
    db: &C,
    patch_set_ids: &[i64],
) -> Result<Vec<optimization_patch::Model>, OptimizeDbError>
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
        .map_err(OptimizeDbError::Database)
}

pub async fn matching_pending_optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
    base_content_hash: &str,
    pending_status: &str,
) -> Result<Vec<optimization_patch_set::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .filter(optimization_patch_set::Column::BaseContentHash.eq(base_content_hash))
        .filter(optimization_patch_set::Column::Status.eq(pending_status))
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn pending_optimization_patch_sets<C>(
    db: &C,
    session_id: i64,
    pending_status: &str,
) -> Result<Vec<optimization_patch_set::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    optimization_patch_set::Entity::find()
        .filter(optimization_patch_set::Column::SessionId.eq(session_id))
        .filter(optimization_patch_set::Column::Status.eq(pending_status))
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn create_optimization_patch_set<C>(
    db: &C,
    input: OptimizationPatchSetCreateInput,
) -> Result<Option<optimization_patch_set::Model>, OptimizeDbError>
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

    Ok(Some(patch_set))
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
