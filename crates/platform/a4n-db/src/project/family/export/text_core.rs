use super::types::{ProjectFamilyExportError, ProjectFamilyTextCoreRows};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, part_outline},
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint},
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(super) async fn project_family_export_text_core_rows<C>(
    db: &C,
    root_project_id: i64,
) -> Result<ProjectFamilyTextCoreRows, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let (
        blueprint,
        blueprint_versions,
        blueprint_characters,
        blueprint_relationships,
        part_outlines,
        chapter_outlines,
        chapters,
        character_profiles,
        object_profiles,
        scene_profiles,
        scene_occurrences,
        scene_transitions,
        scene_entity_states,
    ) = futures::join!(
        async {
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(root_project_id))
                .one(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            blueprint_version::Entity::find()
                .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
                .order_by_asc(blueprint_version::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(root_project_id))
                .order_by_asc(blueprint_character::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(root_project_id))
                .order_by_asc(blueprint_relationship::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            part_outline::Entity::find()
                .filter(part_outline::Column::ProjectId.eq(root_project_id))
                .order_by_asc(part_outline::Column::PartNumber)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            chapter_outline::Entity::find()
                .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
                .order_by_asc(chapter_outline::Column::ChapterNumber)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            chapter::Entity::find()
                .filter(chapter::Column::ProjectId.eq(root_project_id))
                .order_by_asc(chapter::Column::ChapterNumber)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
                .order_by_asc(protagonist_profile::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            object_profile::Entity::find()
                .filter(object_profile::Column::ProjectId.eq(root_project_id))
                .order_by_asc(object_profile::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            scene_profile::Entity::find()
                .filter(scene_profile::Column::ProjectId.eq(root_project_id))
                .order_by_asc(scene_profile::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            scene_occurrence::Entity::find()
                .filter(scene_occurrence::Column::ProjectId.eq(root_project_id))
                .order_by_asc(scene_occurrence::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            scene_transition::Entity::find()
                .filter(scene_transition::Column::ProjectId.eq(root_project_id))
                .order_by_asc(scene_transition::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
        async {
            scene_entity_state::Entity::find()
                .filter(scene_entity_state::Column::ProjectId.eq(root_project_id))
                .order_by_asc(scene_entity_state::Column::Id)
                .all(db)
                .await
                .map_err(ProjectFamilyExportError::Database)
        },
    );

    Ok(ProjectFamilyTextCoreRows {
        fact_feedback: Some(
            crate::fact_feedback::load_feedback_archive(db, root_project_id).await?,
        ),
        blueprint: blueprint?,
        blueprint_versions: blueprint_versions?,
        blueprint_characters: blueprint_characters?,
        blueprint_relationships: blueprint_relationships?,
        part_outlines: part_outlines?,
        chapter_outlines: chapter_outlines?,
        chapters: chapters?,
        character_profiles: character_profiles?,
        object_profiles: object_profiles?,
        scene_profiles: scene_profiles?,
        scene_occurrences: scene_occurrences?,
        scene_transitions: scene_transitions?,
        scene_entity_states: scene_entity_states?,
    })
}
