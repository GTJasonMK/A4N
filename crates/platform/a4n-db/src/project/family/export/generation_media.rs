use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{
        chapter_manga_prompt, chapter_shooting_script, chapter_video_prompt, chapter_visual_plan,
    },
    character::character_portrait,
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::types::ProjectFamilyExportError;

project_family_export_load_by_ids!(
    project_family_export_chapter_visual_plans,
    chapter_visual_plan::Entity,
    chapter_visual_plan::Column::ProjectId,
    chapter_visual_plan::Column::SourceChapterId,
    chapter_visual_plan::Column::PlanVersion,
    chapter_visual_plan::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_chapter_shooting_scripts,
    chapter_shooting_script::Entity,
    chapter_shooting_script::Column::ProjectId,
    chapter_shooting_script::Column::SourceChapterId,
    chapter_shooting_script::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_character_portraits,
    character_portrait::Entity,
    character_portrait::Column::ProjectId,
    character_portrait::Column::ProfileId,
    character_portrait::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_generated_images,
    generated_image::Entity,
    generated_image::Column::ProjectId,
    generated_image::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_chapter_manga_prompts,
    chapter_manga_prompt::Entity,
    chapter_manga_prompt::Column::ProjectId,
    chapter_manga_prompt::Column::ChapterId,
    chapter_manga_prompt::Column::PageNumber,
    chapter_manga_prompt::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_chapter_video_prompts,
    chapter_video_prompt::Entity,
    chapter_video_prompt::Column::ProjectId,
    chapter_video_prompt::Column::ChapterId,
    chapter_video_prompt::Column::ShotNumber,
    chapter_video_prompt::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_generated_videos,
    generated_video::Entity,
    generated_video::Column::ProjectId,
    generated_video::Column::ChapterId,
    generated_video::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_generation_specs,
    generation_spec::Entity,
    generation_spec::Column::ProjectId,
    generation_spec::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_generation_spec_text_anchors,
    generation_spec_text_anchor::Entity,
    generation_spec_text_anchor::Column::GenerationSpecId,
);

project_family_export_load_by_ids!(
    project_family_export_generation_spec_entity_refs,
    generation_spec_entity_ref::Entity,
    generation_spec_entity_ref::Column::GenerationSpecId,
);

pub(super) async fn project_family_export_generation_spec_media_refs<C>(
    db: &C,
    spec_ids: &[i64],
    media_asset_ids: &[i64],
) -> Result<Vec<generation_spec_media_ref::Model>, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    if spec_ids.is_empty() && media_asset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = generation_spec_media_ref::Entity::find();
    if !spec_ids.is_empty() {
        query = query
            .filter(generation_spec_media_ref::Column::GenerationSpecId.is_in(spec_ids.to_vec()));
    }
    if !media_asset_ids.is_empty() {
        query = query.filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(media_asset_ids.to_vec()),
        );
    }
    query
        .all(db)
        .await
        .map_err(ProjectFamilyExportError::Database)
}

project_family_export_load_by_ids!(
    project_family_export_prompt_artifacts,
    prompt_artifact::Entity,
    prompt_artifact::Column::GenerationSpecId,
    prompt_artifact::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_media_assets,
    media_asset::Entity,
    media_asset::Column::ProjectId,
    media_asset::Column::Id,
);

project_family_export_load_by_ids!(
    project_family_export_media_provenance,
    media_provenance::Entity,
    media_provenance::Column::MediaAssetId,
);

project_family_export_load_by_ids!(
    project_family_export_media_bindings,
    media_binding::Entity,
    media_binding::Column::MediaAssetId,
    media_binding::Column::Id,
);
