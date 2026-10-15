use std::collections::BTreeSet;

use a4n_db_models::models::{
    ai::generated_image,
    asset::profile_primary_material,
    character::{
        attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
        protagonist_deletion_mark, protagonist_profile, snapshot_reference,
    },
    project::novel_project,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::character::portrait;

use super::records::referenced_profile_ids;
use super::types::{CharacterProfileDeleteError, CharacterProfileDeletionSummary};

pub(crate) async fn delete_profile_cascade_in<C>(
    db: &C,
    profile_id: i64,
) -> Result<CharacterProfileDeletionSummary, CharacterProfileDeleteError>
where
    C: ConnectionTrait,
{
    let mut profile_query = protagonist_profile::Entity::find_by_id(profile_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        profile_query = profile_query.lock_exclusive();
    }
    let profile = profile_query
        .select_only()
        .column(protagonist_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| CharacterProfileDeleteError::NotFound("主角档案不存在".into()))?;
    if db.get_database_backend() != DbBackend::Sqlite {
        novel_project::Entity::find_by_id(profile)
            .select_only()
            .column(novel_project::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
            .one(db)
            .await?;
    }

    if referenced_profile_ids(db, profile, &[profile_id])
        .await?
        .contains(&profile_id)
    {
        return Err(CharacterProfileDeleteError::Referenced(profile_id));
    }
    let state_index_ids = character_state_index::Entity::find()
        .select_only()
        .column(character_state_index::Column::Id)
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let attribute_event_ids = attribute_event::Entity::find()
        .select_only()
        .column(attribute_event::Column::Id)
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut portraits_query = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .order_by_asc(character_portrait::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        portraits_query = portraits_query.lock_exclusive();
    }
    let portraits = portraits_query
        .select_only()
        .column(character_portrait::Column::Id)
        .column(character_portrait::Column::ImagePath)
        .into_tuple::<(i64, String)>()
        .all(db)
        .await?;
    let portrait_ids = portraits
        .iter()
        .map(|(portrait_id, _)| *portrait_id)
        .collect::<Vec<_>>();
    let portrait_paths = portraits
        .iter()
        .map(|(_, image_path)| image_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if !portrait_paths.is_empty() {
        let mut image_query = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(profile))
            .filter(generated_image::Column::FilePath.is_in(portrait_paths.clone()))
            .order_by_asc(generated_image::Column::Id);
        if db.get_database_backend() != DbBackend::Sqlite {
            image_query = image_query.lock_exclusive();
        }
        let image_ids = image_query
            .select_only()
            .column(generated_image::Column::Id)
            .column(generated_image::Column::MetadataJson)
            .into_tuple::<(i64, Option<serde_json::Value>)>()
            .all(db)
            .await?
            .into_iter()
            .filter(|(_, metadata_json)| {
                portrait::generated_image_metadata_matches_any_portrait(
                    metadata_json.as_ref(),
                    &portrait_ids,
                )
            })
            .map(|(image_id, _)| image_id)
            .collect::<Vec<_>>();
        crate::video::clear_generated_image_references_record(db, &image_ids).await?;
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::PrimaryImageId.is_in(image_ids.clone()))
            .exec(db)
            .await?;
        crate::media::delete_generated_image_target_media_bindings(db, &image_ids).await?;
        crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids)
            .await?;
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::Id.is_in(image_ids))
            .exec(db)
            .await?;
    }

    crate::generation::delete_generation_run_targets_for(db, "character_portrait", &portrait_ids)
        .await?;

    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.eq(profile_id))
        .exec(db)
        .await?;

    crate::generation::delete_generation_run_targets_for(db, "protagonist_profile", &[profile_id])
        .await?;
    let deleted = protagonist_profile::Entity::delete_by_id(profile_id)
        .exec(db)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(CharacterProfileDeleteError::NotFound(
            "主角档案不存在".into(),
        ));
    }
    let deletable_portrait_paths =
        crate::media::filter_unreferenced_media_paths(db, &portrait_paths).await?;

    Ok(CharacterProfileDeletionSummary {
        project_id: profile,
        profile_id,
        portrait_paths: deletable_portrait_paths,
        attribute_event_ids,
        state_index_ids,
    })
}

pub(crate) async fn delete_profiles_cascade_in<C>(
    db: &C,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<(), CharacterProfileDeleteError>
where
    C: ConnectionTrait,
{
    if profile_ids.is_empty() {
        return Ok(());
    }

    let requested_profile_ids = profile_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut profile_query = protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Id.is_in(requested_profile_ids.iter().copied()));
    if db.get_database_backend() != DbBackend::Sqlite {
        profile_query = profile_query.lock_exclusive();
    }
    let existing_profile_ids = profile_query
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    if existing_profile_ids != requested_profile_ids {
        return Err(CharacterProfileDeleteError::NotFound(
            "主角档案不存在".into(),
        ));
    }
    if db.get_database_backend() != DbBackend::Sqlite {
        novel_project::Entity::find_by_id(project_id)
            .select_only()
            .column(novel_project::Column::Id)
            .lock_exclusive()
            .into_tuple::<i64>()
            .one(db)
            .await?
            .ok_or_else(|| CharacterProfileDeleteError::NotFound("项目不存在".into()))?;
    }

    let referenced = referenced_profile_ids(db, project_id, profile_ids).await?;
    if let Some(profile_id) = referenced.first() {
        return Err(CharacterProfileDeleteError::Referenced(*profile_id));
    }
    let mut portraits_query = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::Id)
        .column(character_portrait::Column::ProfileId)
        .column(character_portrait::Column::ImagePath)
        .filter(character_portrait::Column::ProfileId.is_in(requested_profile_ids.iter().copied()))
        .order_by_asc(character_portrait::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        portraits_query = portraits_query.lock_exclusive();
    }
    let portraits = portraits_query
        .into_tuple::<(i64, i64, String)>()
        .all(db)
        .await?;
    let portrait_ids = portraits
        .iter()
        .map(|(portrait_id, _, _)| *portrait_id)
        .collect::<Vec<_>>();
    let portrait_paths = portraits
        .iter()
        .map(|(_, _, image_path)| image_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let image_ids = if portrait_paths.is_empty() {
        Vec::new()
    } else {
        let mut image_query = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::FilePath.is_in(portrait_paths.iter().cloned()));
        if db.get_database_backend() != DbBackend::Sqlite {
            image_query = image_query.lock_exclusive();
        }
        image_query
            .select_only()
            .column(generated_image::Column::Id)
            .column(generated_image::Column::MetadataJson)
            .into_tuple::<(i64, Option<serde_json::Value>)>()
            .all(db)
            .await?
            .into_iter()
            .filter(|(_, metadata_json)| {
                portrait::generated_image_metadata_matches_any_portrait(
                    metadata_json.as_ref(),
                    &portrait_ids,
                )
            })
            .map(|(image_id, _)| image_id)
            .collect::<Vec<_>>()
    };

    crate::video::clear_generated_image_references_record(db, &image_ids).await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::PrimaryImageId.is_in(image_ids.iter().copied()))
        .exec(db)
        .await?;
    crate::media::delete_generated_image_target_media_bindings(db, &image_ids).await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_image", &image_ids).await?;
    generated_image::Entity::delete_many()
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .exec(db)
        .await?;

    crate::generation::delete_generation_run_targets_for(db, "character_portrait", &portrait_ids)
        .await?;
    protagonist_behavior_record::Entity::delete_many()
        .filter(
            protagonist_behavior_record::Column::ProfileId
                .is_in(requested_profile_ids.iter().copied()),
        )
        .exec(db)
        .await?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(
            protagonist_deletion_mark::Column::ProfileId
                .is_in(requested_profile_ids.iter().copied()),
        )
        .exec(db)
        .await?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.is_in(requested_profile_ids.iter().copied()))
        .exec(db)
        .await?;
    character_state_index::Entity::delete_many()
        .filter(
            character_state_index::Column::ProfileId.is_in(requested_profile_ids.iter().copied()),
        )
        .exec(db)
        .await?;
    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.is_in(requested_profile_ids.iter().copied()))
        .exec(db)
        .await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.is_in(requested_profile_ids.iter().copied()))
        .exec(db)
        .await?;
    crate::generation::delete_generation_run_targets_for(
        db,
        "protagonist_profile",
        &requested_profile_ids.iter().copied().collect::<Vec<_>>(),
    )
    .await?;
    protagonist_profile::Entity::delete_many()
        .filter(protagonist_profile::Column::Id.is_in(requested_profile_ids.iter().copied()))
        .exec(db)
        .await?;
    crate::media::filter_unreferenced_media_paths(db, &portrait_paths).await?;
    Ok(())
}

pub async fn delete_profile_cascade_transaction<C>(
    db: &C,
    profile_id: i64,
) -> Result<CharacterProfileDeletionSummary, CharacterProfileDeleteError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let project_id = protagonist_profile::Entity::find_by_id(profile_id)
        .select_only()
        .column(protagonist_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| CharacterProfileDeleteError::NotFound("主角档案不存在".into()))?;
    let tx = db
        .begin()
        .await
        .map_err(CharacterProfileDeleteError::Database)?;
    let project = crate::project::project_write_identity_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| CharacterProfileDeleteError::NotFound("项目不存在".into()))?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(CharacterProfileDeleteError::InvalidState {
            current: project.lifecycle_gate,
            operation: "删除角色档案".to_string(),
        });
    }
    let summary = delete_profile_cascade_in(&tx, profile_id).await?;
    if !summary.portrait_paths.is_empty() {
        crate::cleanup_outbox::insert_file_artifact_cleanup_intent(
            &tx,
            format!("character-profile-delete:{profile_id}"),
            crate::cleanup_outbox::FileArtifactCleanupPayload::new(
                summary.project_id,
                summary.portrait_paths.clone(),
            ),
        )
        .await?;
    }
    tx.commit()
        .await
        .map_err(CharacterProfileDeleteError::Database)?;
    Ok(summary)
}
