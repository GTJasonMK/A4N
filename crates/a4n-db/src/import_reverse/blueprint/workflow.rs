use chrono::Utc;
use sea_orm::{ConnectionTrait, TransactionTrait};

use super::{
    apply_reverse_blueprint_record, resolve_reverse_blueprint_apply_record,
    reverse_blueprint_apply_plan_record,
};
use crate::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseBlueprintApplyResult,
};

pub async fn apply_reverse_blueprint_records<C>(
    db: &C,
    project_id: i64,
    draft: ReverseBlueprintApplyDraft,
) -> Result<ReverseBlueprintApplyResult, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let apply_plan = reverse_blueprint_apply_plan_record(db, project_id).await?;
    let now = Utc::now().naive_utc();
    let record = resolve_reverse_blueprint_apply_record(&draft, &apply_plan, now)?;
    apply_reverse_blueprint_record(db, project_id, &record).await?;

    let character_count = draft.characters.len();
    crate::project::sync_blueprint_characters(db, project_id, draft.characters, now).await?;
    let relationship_count = draft.relationships.len();
    crate::project::sync_blueprint_relationships(db, project_id, draft.relationships, now).await?;

    Ok(ReverseBlueprintApplyResult {
        character_count,
        relationship_count,
    })
}

pub async fn apply_reverse_blueprint_records_transaction<T>(
    db: &T,
    project_id: i64,
    draft: ReverseBlueprintApplyDraft,
) -> Result<ReverseBlueprintApplyResult, ImportReverseDbError>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let result = apply_reverse_blueprint_records(&txn, project_id, draft).await?;
    txn.commit().await?;
    Ok(result)
}
