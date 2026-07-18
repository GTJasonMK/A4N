use std::collections::HashMap;

use a4n_core::models::generation::{
    generation_spec_entity_ref, generation_spec_media_ref, generation_spec_text_anchor,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::shared::remap_derived_required_id;

macro_rules! copy_derived_generation_spec_ref_rows {
    ($fn_name:ident, $module:ident, $target_field:ident, $target_map:ident, $spec_label:literal, $target_label:literal) => {
        pub(in crate::project::duplicate) async fn $fn_name<C>(
            db: &C,
            old_spec_ids: &[i64],
            spec_id_map: &HashMap<i64, i64>,
            $target_map: &HashMap<i64, i64>,
        ) -> Result<(), ProjectDuplicateError>
        where
            C: ConnectionTrait,
        {
            if old_spec_ids.is_empty() {
                return Ok(());
            }
            let refs = $module::Entity::find()
                .filter($module::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
                .all(db)
                .await
                .map_err(ProjectDuplicateError::Database)?;
            for item in refs {
                $module::ActiveModel {
                    generation_spec_id: Set(remap_derived_required_id(
                        spec_id_map,
                        item.generation_spec_id,
                        $spec_label,
                    )?),
                    $target_field: Set(remap_derived_required_id(
                        $target_map,
                        item.$target_field,
                        $target_label,
                    )?),
                    role: Set(item.role),
                }
                .insert(db)
                .await
                .map_err(ProjectDuplicateError::Database)?;
            }
            Ok(())
        }
    };
}

copy_derived_generation_spec_ref_rows!(
    copy_derived_generation_spec_text_anchor_refs,
    generation_spec_text_anchor,
    text_anchor_id,
    anchor_id_map,
    "generation_spec_text_anchor.generation_spec_id",
    "generation_spec_text_anchor.text_anchor_id"
);

pub(in crate::project::duplicate::derived) async fn copy_derived_generation_spec_entity_refs<C>(
    db: &C,
    old_spec_ids: &[i64],
    spec_id_map: &HashMap<i64, i64>,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(());
    }
    let refs = generation_spec_entity_ref::Entity::find()
        .filter(generation_spec_entity_ref::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for item in refs {
        generation_spec_entity_ref::ActiveModel {
            generation_spec_id: Set(remap_derived_required_id(
                spec_id_map,
                item.generation_spec_id,
                "generation_spec_entity_ref.generation_spec_id",
            )?),
            entity_kind: Set(item.entity_kind),
            entity_id: Set(item.entity_id),
            role: Set(item.role),
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}

copy_derived_generation_spec_ref_rows!(
    copy_derived_generation_spec_media_refs,
    generation_spec_media_ref,
    media_asset_id,
    asset_id_map,
    "generation_spec_media_ref.generation_spec_id",
    "generation_spec_media_ref.media_asset_id"
);
