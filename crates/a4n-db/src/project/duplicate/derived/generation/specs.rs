use std::collections::HashMap;

use a4n_core::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::shared::remap_derived_optional_id;

use super::super::types::{CopiedDerivedGenerationSpecs, DuplicatedDerivedSourceSnapshot};

pub(in crate::project::duplicate::derived) async fn copy_derived_generation_specs<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    now: NaiveDateTime,
) -> Result<CopiedDerivedGenerationSpecs, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut spec_id_map = HashMap::new();
    let mut segment_target_updates = Vec::new();
    for spec in &specs {
        let source_snapshot_chapter_id = remap_derived_optional_id(
            &duplicated_snapshot.chapter_ids,
            spec.source_snapshot_chapter_id,
            "generation_spec.source_snapshot_chapter_id",
        )?;
        let target_entity_id = if spec.target_entity_kind == "chapter" {
            remap_derived_optional_id(
                &duplicated_snapshot.chapter_ids,
                spec.target_entity_id,
                "generation_spec.target_entity_id",
            )?
        } else if spec.target_entity_kind == "segment" {
            None
        } else {
            spec.target_entity_id
        };
        let copied = generation_spec::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(root_project_id),
            source_context_kind: Set(Some("snapshot_backed".to_string())),
            source_snapshot_id: Set(Some(duplicated_snapshot.snapshot.id)),
            source_chapter_id: Set(spec.source_chapter_id),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(duplicated_snapshot.snapshot.source_version_hash.clone()),
            params_json: Set(spec.params_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        if spec.target_entity_kind == "segment"
            && let Some(old_segment_id) = spec.target_entity_id
        {
            segment_target_updates.push((copied.id, old_segment_id));
        }
        spec_id_map.insert(spec.id, copied.id);
    }
    Ok(CopiedDerivedGenerationSpecs {
        spec_ids: specs.iter().map(|spec| spec.id).collect(),
        spec_id_map,
        segment_target_updates,
    })
}
