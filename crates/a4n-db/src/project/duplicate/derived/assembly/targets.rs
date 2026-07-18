use std::collections::HashMap;

use a4n_core::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

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
    for (new_spec_id, old_segment_id) in updates {
        generation_spec::ActiveModel {
            id: Set(*new_spec_id),
            target_entity_id: Set(Some(remap_derived_required_id(
                segment_id_map,
                *old_segment_id,
                "generation_spec.target_entity_id",
            )?)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
