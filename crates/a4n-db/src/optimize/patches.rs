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

    for patch in input.patches {
        optimization_patch::ActiveModel {
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
        }
        .insert(db)
        .await
        .map_err(OptimizeDbError::Database)?;
    }

    Ok(Some(patch_set))
}

pub async fn update_optimization_patch_set_status<C>(
    db: &C,
    patch_set: optimization_patch_set::Model,
    patch_set_status: &str,
    patch_decision: &str,
    patch_apply_status: &str,
) -> Result<(), OptimizeDbError>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    let patches = optimization_patch_set_patches(db, patch_set.id).await?;
    for patch in patches {
        let mut active: optimization_patch::ActiveModel = patch.into();
        active.decision = Set(patch_decision.to_string());
        active.apply_status = Set(patch_apply_status.to_string());
        active.updated_at = Set(now);
        active.update(db).await.map_err(OptimizeDbError::Database)?;
    }

    let mut active: optimization_patch_set::ActiveModel = patch_set.into();
    active.status = Set(patch_set_status.to_string());
    active.updated_at = Set(now);
    active.update(db).await.map_err(OptimizeDbError::Database)?;
    Ok(())
}
