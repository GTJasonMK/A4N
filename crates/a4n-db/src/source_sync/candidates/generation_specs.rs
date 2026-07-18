use a4n_core::models::{
    generation::{generation_spec, generation_spec_entity_ref},
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::contracts::SourceSyncProfileGenerationSpecRefRecord;
use crate::source_sync::{SourceSyncDbError, required_text, validate_positive_id};

pub async fn generation_specs_for_source_chapter<C>(
    db: &C,
    derived_project_id: i64,
    source_chapter_id: i64,
) -> Result<Vec<generation_spec::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(derived_project_id, "derived_project_id")?;
    validate_positive_id(source_chapter_id, "source_chapter_id")?;
    generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(derived_project_id))
        .filter(generation_spec::Column::SourceChapterId.eq(source_chapter_id))
        .all(db)
        .await
        .map_err(SourceSyncDbError::Database)
}

pub async fn generation_specs_for_profile_ref<C>(
    db: &C,
    run: &source_sync_run::Model,
    profile_kind: &str,
    source_profile_id: i64,
) -> Result<Vec<SourceSyncProfileGenerationSpecRefRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    let profile_kind = required_text(profile_kind, "profile_kind")?;
    validate_positive_id(source_profile_id, "source_profile_id")?;

    let refs = generation_spec_entity_ref::Entity::find()
        .filter(generation_spec_entity_ref::Column::EntityKind.eq(profile_kind))
        .filter(generation_spec_entity_ref::Column::EntityId.eq(source_profile_id))
        .all(db)
        .await?;

    let mut records = Vec::new();
    for item in refs {
        let Some(spec) = generation_spec::Entity::find_by_id(item.generation_spec_id)
            .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
            .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
            .one(db)
            .await?
        else {
            continue;
        };
        records.push(SourceSyncProfileGenerationSpecRefRecord {
            spec,
            entity_role: item.role,
        });
    }
    Ok(records)
}
