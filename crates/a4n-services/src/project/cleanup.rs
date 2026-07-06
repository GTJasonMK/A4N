use sea_orm::*;

use a4n_core::error::AppError;

use crate::generation::delete_generation_run_targets_for;
use crate::video::mark_canvas_generated_video_refs_missing;

#[derive(Debug, Clone, Default)]
pub(super) struct OptimizationCleanupResult {
    pub deleted_sessions: u64,
    pub deleted_session_messages: u64,
    pub deleted_turns: u64,
    pub deleted_events: u64,
    pub deleted_patch_sets: u64,
    pub deleted_patches: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct GenerationMediaCleanupResult {
    pub deleted_generation_specs: u64,
    pub deleted_prompt_artifacts: u64,
    pub deleted_media_assets: u64,
    pub deleted_media_provenance: u64,
    pub deleted_media_bindings: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum GeneratedImageDeleteScope {
    Project,
    Chapters,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TextAnchorDeleteScope {
    Project,
    CurrentProjectContent,
}

#[derive(Debug, Clone, Default)]
pub(super) struct TextAnchorCleanupResult {
    pub deleted_text_anchors: u64,
    pub deleted_profile_evidence: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ChapterGenerationCleanupResult {
    pub deleted_manga_prompts: u64,
    pub deleted_video_prompts: u64,
    pub deleted_chapter_scenes: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ConversationCleanupResult {
    pub deleted_conversations: u64,
    pub deleted_messages: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct TaskCleanupResult {
    pub deleted_tasks: u64,
    pub deleted_dependencies: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct BlueprintCleanupResult {
    pub deleted_current_blueprints: u64,
    pub deleted_blueprint_characters: u64,
    pub deleted_blueprint_relationships: u64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ProfileSceneCleanupResult {
    pub deleted_object_profiles: u64,
    pub deleted_scene_profiles: u64,
    pub deleted_scene_occurrences: u64,
    pub deleted_scene_transitions: u64,
    pub deleted_scene_entity_states: u64,
}

pub(super) async fn delete_optimization_tree<C>(
    db: &C,
    project_id: i64,
) -> Result<OptimizationCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::ai::{
        optimization_patch, optimization_patch_set, optimization_session,
        optimization_session_message, optimization_turn, optimization_turn_event,
    };

    let session_ids = optimization_session::Entity::find()
        .select_only()
        .column(optimization_session::Column::Id)
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .into_query();
    let turn_ids = optimization_turn::Entity::find()
        .select_only()
        .column(optimization_turn::Column::Id)
        .filter(optimization_turn::Column::SessionId.in_subquery(session_ids.clone()))
        .into_query();
    let patch_set_ids = optimization_patch_set::Entity::find()
        .select_only()
        .column(optimization_patch_set::Column::Id)
        .filter(optimization_patch_set::Column::SessionId.in_subquery(session_ids.clone()))
        .into_query();

    let deleted_patches = optimization_patch::Entity::delete_many()
        .filter(optimization_patch::Column::PatchSetId.in_subquery(patch_set_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_patch_sets = optimization_patch_set::Entity::delete_many()
        .filter(optimization_patch_set::Column::Id.in_subquery(patch_set_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_events = optimization_turn_event::Entity::delete_many()
        .filter(optimization_turn_event::Column::TurnId.in_subquery(turn_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_turns = optimization_turn::Entity::delete_many()
        .filter(optimization_turn::Column::Id.in_subquery(turn_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_session_messages = optimization_session_message::Entity::delete_many()
        .filter(optimization_session_message::Column::SessionId.in_subquery(session_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_sessions = optimization_session::Entity::delete_many()
        .filter(optimization_session::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(OptimizationCleanupResult {
        deleted_sessions,
        deleted_session_messages,
        deleted_turns,
        deleted_events,
        deleted_patch_sets,
        deleted_patches,
    })
}

pub(super) async fn delete_generation_media_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<GenerationMediaCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    };
    use a4n_core::models::media::{media_asset, media_binding, media_provenance};

    let generation_spec_ids = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .into_query();
    let media_asset_ids = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .into_query();
    let deleted_media_bindings = media_binding::Entity::delete_many()
        .filter(media_binding::Column::MediaAssetId.in_subquery(media_asset_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_media_provenance = media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.in_subquery(media_asset_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId
                .in_subquery(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::MediaAssetId.in_subquery(media_asset_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    let deleted_media_assets = media_asset::Entity::delete_many()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_prompt_artifacts = prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.in_subquery(generation_spec_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .in_subquery(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId
                .in_subquery(generation_spec_ids.clone()),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    let deleted_generation_specs = generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(GenerationMediaCleanupResult {
        deleted_generation_specs,
        deleted_prompt_artifacts,
        deleted_media_assets,
        deleted_media_provenance,
        deleted_media_bindings,
    })
}

pub(super) async fn delete_chapter_generation_outputs<C>(
    db: &C,
    project_id: i64,
    image_scope: GeneratedImageDeleteScope,
) -> Result<ChapterGenerationCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::ai::{generated_image, generated_video};
    use a4n_core::models::chapter::{
        chapter, chapter_manga_prompt, chapter_scene, chapter_video_prompt,
    };

    let chapter_ids = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .into_query();
    let manga_prompt_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let video_prompt_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generated_video_ids = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let image_ids_query = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id);
    let image_ids_query = match image_scope {
        GeneratedImageDeleteScope::Project => {
            image_ids_query.filter(generated_image::Column::ProjectId.eq(project_id))
        }
        GeneratedImageDeleteScope::Chapters => image_ids_query
            .filter(generated_image::Column::ChapterId.in_subquery(chapter_ids.clone())),
    };
    let generated_image_ids = image_ids_query
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;

    delete_generation_run_targets_for(db, "chapter_manga_prompt", &manga_prompt_ids).await?;
    delete_generation_run_targets_for(db, "chapter_video_prompt", &video_prompt_ids).await?;
    delete_generation_run_targets_for(db, "generated_video", &generated_video_ids).await?;
    delete_generation_run_targets_for(db, "generated_image", &generated_image_ids).await?;
    for video_id in &generated_video_ids {
        mark_canvas_generated_video_refs_missing(db, project_id, *video_id).await?;
    }

    let deleted_manga_prompts = chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_video_prompts = chapter_video_prompt::Entity::delete_many()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_chapter_scenes = chapter_scene::Entity::delete_many()
        .filter(chapter_scene::Column::ChapterId.in_subquery(chapter_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_generated_videos = generated_video::Entity::delete_many()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    let image_delete = generated_image::Entity::delete_many();
    let image_delete = match image_scope {
        GeneratedImageDeleteScope::Project => {
            image_delete.filter(generated_image::Column::ProjectId.eq(project_id))
        }
        GeneratedImageDeleteScope::Chapters => {
            image_delete.filter(generated_image::Column::ChapterId.in_subquery(chapter_ids))
        }
    };
    let deleted_generated_images = image_delete
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(ChapterGenerationCleanupResult {
        deleted_manga_prompts,
        deleted_video_prompts,
        deleted_chapter_scenes,
        deleted_generated_images,
        deleted_generated_videos,
    })
}

pub(super) async fn delete_project_conversations<C>(
    db: &C,
    project_id: i64,
) -> Result<ConversationCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::user::{conversation, message};

    let conversation_ids = conversation::Entity::find()
        .select_only()
        .column(conversation::Column::Id)
        .filter(conversation::Column::ProjectId.eq(project_id))
        .into_query();
    let deleted_messages = message::Entity::delete_many()
        .filter(message::Column::ConversationId.in_subquery(conversation_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_conversations = conversation::Entity::delete_many()
        .filter(conversation::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(ConversationCleanupResult {
        deleted_conversations,
        deleted_messages,
    })
}

pub(super) async fn delete_project_tasks<C>(
    db: &C,
    project_id: i64,
) -> Result<TaskCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::task::{task_dependency, task_queue};

    let task_ids = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .into_query();
    let deleted_dependencies = task_dependency::Entity::delete_many()
        .filter(
            Condition::any()
                .add(task_dependency::Column::TaskId.in_subquery(task_ids.clone()))
                .add(task_dependency::Column::DependsOnTaskId.in_subquery(task_ids.clone())),
        )
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_tasks = task_queue::Entity::delete_many()
        .filter(task_queue::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(TaskCleanupResult {
        deleted_tasks,
        deleted_dependencies,
    })
}

pub(super) async fn delete_project_blueprints<C>(
    db: &C,
    project_id: i64,
    delete_versions: bool,
) -> Result<BlueprintCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
    };

    let deleted_blueprint_characters = blueprint_character::Entity::delete_many()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_blueprint_relationships = blueprint_relationship::Entity::delete_many()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    if delete_versions {
        blueprint_version::Entity::delete_many()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .exec(db)
            .await
            .map_err(AppError::Database)?;
    }
    let deleted_current_blueprints = novel_blueprint::Entity::delete_many()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(BlueprintCleanupResult {
        deleted_current_blueprints,
        deleted_blueprint_characters,
        deleted_blueprint_relationships,
    })
}

pub(super) async fn delete_project_profile_scene_data<C>(
    db: &C,
    project_id: i64,
) -> Result<ProfileSceneCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::asset::{object_profile, profile_primary_material};
    use a4n_core::models::scene::{
        scene_entity_state, scene_occurrence, scene_profile, scene_transition,
    };

    let scene_profile_ids = scene_profile::Entity::find()
        .select_only()
        .column(scene_profile::Column::Id)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let object_profile_ids = object_profile::Entity::find()
        .select_only()
        .column(object_profile::Column::Id)
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    delete_generation_run_targets_for(db, "scene_profile", &scene_profile_ids).await?;
    delete_generation_run_targets_for(db, "object_profile", &object_profile_ids).await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    let deleted_scene_entity_states = scene_entity_state::Entity::delete_many()
        .filter(scene_entity_state::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_scene_transitions = scene_transition::Entity::delete_many()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_scene_occurrences = scene_occurrence::Entity::delete_many()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_scene_profiles = scene_profile::Entity::delete_many()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_object_profiles = object_profile::Entity::delete_many()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(ProfileSceneCleanupResult {
        deleted_object_profiles,
        deleted_scene_profiles,
        deleted_scene_occurrences,
        deleted_scene_transitions,
        deleted_scene_entity_states,
    })
}

pub(super) async fn delete_project_text_anchors<C>(
    db: &C,
    project_id: i64,
    scope: TextAnchorDeleteScope,
) -> Result<TextAnchorCleanupResult, AppError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::project::{profile_evidence, text_anchor};

    let mut anchor_query = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .filter(text_anchor::Column::ProjectId.eq(project_id));
    if matches!(scope, TextAnchorDeleteScope::CurrentProjectContent) {
        anchor_query = anchor_query.filter(text_anchor::Column::SourceSnapshotId.is_null());
    }
    let text_anchor_ids = anchor_query.into_query();

    let deleted_profile_evidence = profile_evidence::Entity::delete_many()
        .filter(profile_evidence::Column::TextAnchorId.in_subquery(text_anchor_ids.clone()))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;
    let deleted_text_anchors = text_anchor::Entity::delete_many()
        .filter(text_anchor::Column::Id.in_subquery(text_anchor_ids))
        .exec(db)
        .await
        .map_err(AppError::Database)?
        .rows_affected;

    Ok(TextAnchorCleanupResult {
        deleted_text_anchors,
        deleted_profile_evidence,
    })
}
