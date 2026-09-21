use a4n_db_models::models::{
    generation::{generation_spec, generation_spec_entity_ref},
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;

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

pub async fn generation_specs_for_source_chapters<C>(
    db: &C,
    derived_project_id: i64,
    source_chapter_ids: &[i64],
) -> Result<HashMap<i64, Vec<generation_spec::Model>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(derived_project_id, "derived_project_id")?;
    for source_chapter_id in source_chapter_ids {
        validate_positive_id(*source_chapter_id, "source_chapter_id")?;
    }
    if source_chapter_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(derived_project_id))
        .filter(generation_spec::Column::SourceChapterId.is_in(source_chapter_ids.iter().copied()))
        .all(db)
        .await
        .map_err(SourceSyncDbError::Database)?;
    let mut specs_by_chapter = HashMap::<i64, Vec<generation_spec::Model>>::new();
    for spec in specs {
        if let Some(source_chapter_id) = spec.source_chapter_id {
            specs_by_chapter
                .entry(source_chapter_id)
                .or_default()
                .push(spec);
        }
    }
    Ok(specs_by_chapter)
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
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    let spec_ids = refs
        .iter()
        .map(|item| item.generation_spec_id)
        .collect::<Vec<_>>();
    let specs_by_id = generation_spec::Entity::find()
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
        .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();

    let mut records = Vec::new();
    for item in refs {
        let Some(spec) = specs_by_id.get(&item.generation_spec_id).cloned() else {
            continue;
        };
        records.push(SourceSyncProfileGenerationSpecRefRecord {
            spec,
            entity_role: item.role,
        });
    }
    Ok(records)
}

pub async fn generation_specs_for_profile_refs<C>(
    db: &C,
    run: &source_sync_run::Model,
    profile_refs: &[(String, i64)],
) -> Result<HashMap<(String, i64), Vec<SourceSyncProfileGenerationSpecRefRecord>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    if profile_refs.is_empty() {
        return Ok(HashMap::new());
    }

    let normalized_refs = profile_refs
        .iter()
        .map(|(profile_kind, source_profile_id)| {
            Ok((required_text(profile_kind, "profile_kind")?.to_string(), {
                validate_positive_id(*source_profile_id, "source_profile_id")?;
                *source_profile_id
            }))
        })
        .collect::<Result<Vec<_>, SourceSyncDbError>>()?;
    let mut filter = Condition::any();
    for (profile_kind, source_profile_id) in &normalized_refs {
        filter = filter.add(
            Condition::all()
                .add(generation_spec_entity_ref::Column::EntityKind.eq(profile_kind))
                .add(generation_spec_entity_ref::Column::EntityId.eq(*source_profile_id)),
        );
    }
    let refs = generation_spec_entity_ref::Entity::find()
        .filter(filter)
        .all(db)
        .await?;
    if refs.is_empty() {
        return Ok(HashMap::new());
    }
    let spec_ids = refs
        .iter()
        .map(|item| item.generation_spec_id)
        .collect::<Vec<_>>();
    let specs_by_id = generation_spec::Entity::find()
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
        .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();

    let mut records_by_ref =
        HashMap::<(String, i64), Vec<SourceSyncProfileGenerationSpecRefRecord>>::new();
    for item in refs {
        let Some(spec) = specs_by_id.get(&item.generation_spec_id).cloned() else {
            continue;
        };
        records_by_ref
            .entry((item.entity_kind, item.entity_id))
            .or_default()
            .push(SourceSyncProfileGenerationSpecRefRecord {
                spec,
                entity_role: item.role,
            });
    }
    Ok(records_by_ref)
}
