use std::collections::HashMap;

use a4n_db_models::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_required_id};

pub(in crate::project::duplicate::derived) async fn update_derived_generation_spec_segment_targets<
    C,
>(
    db: &C,
    updates: &[(i64, i64)],
    segment_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if updates.is_empty() {
        return Ok(());
    }
    let mut target_entity_id_case = CaseStatement::new();
    let mut spec_ids = Vec::with_capacity(updates.len());
    for (new_spec_id, old_segment_id) in updates {
        target_entity_id_case = target_entity_id_case.case(
            generation_spec::Column::Id.eq(*new_spec_id),
            Expr::value(Some(remap_derived_required_id(
                segment_id_map,
                *old_segment_id,
                "generation_spec.target_entity_id",
            )?)),
        );
        spec_ids.push(*new_spec_id);
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::TargetEntityId,
            target_entity_id_case
                .finally(Expr::col(generation_spec::Column::TargetEntityId))
                .into(),
        )
        .col_expr(generation_spec::Column::UpdatedAt, Expr::value(now))
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != updates.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成意图目标数量与待回填记录不一致".to_string(),
            operation: "回填派生项目生成意图目标".to_string(),
        });
    }
    Ok(())
}
