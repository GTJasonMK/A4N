use std::collections::HashMap;

use a4n_core::models::generation::prompt_artifact;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::shared::remap_derived_required_id;

pub(in crate::project::duplicate) async fn copy_derived_prompt_artifacts<C>(
    db: &C,
    old_spec_ids: &[i64],
    spec_id_map: &HashMap<i64, i64>,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let prompts = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .order_by_asc(prompt_artifact::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut prompt_id_map = HashMap::new();
    for prompt in prompts {
        let generation_spec_id = remap_derived_required_id(
            spec_id_map,
            prompt.generation_spec_id,
            "prompt_artifact.generation_spec_id",
        )?;
        let copied = prompt_artifact::ActiveModel {
            generation_spec_id: Set(generation_spec_id),
            prompt_kind: Set(prompt.prompt_kind),
            prompt_text: Set(prompt.prompt_text),
            renderer_version: Set(prompt.renderer_version),
            edited_by_user: Set(prompt.edited_by_user),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        prompt_id_map.insert(prompt.id, copied.id);
    }
    Ok(prompt_id_map)
}
