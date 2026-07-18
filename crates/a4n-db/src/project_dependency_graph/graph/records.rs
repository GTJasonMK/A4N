mod family;
mod generation;
mod root;

use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        asset::object_profile,
        chapter::{
            chapter, chapter_manga_prompt, chapter_outline, chapter_shooting_script,
            chapter_video_prompt, part_outline,
        },
        character::{character_portrait, protagonist_profile},
        generation::{generation_spec, prompt_artifact},
        media::{media_asset, media_provenance},
        project::{blueprint_version, novel_blueprint},
        scene::scene_profile,
        user::conversation,
    },
};
use sea_orm::ConnectionTrait;

use self::{
    family::{FamilyAssetGraphRecords, load_family_asset_graph_records},
    generation::{GenerationAssetGraphRecords, load_generation_asset_graph_records},
    root::{RootAssetGraphRecords, load_root_asset_graph_records},
};

pub(super) struct AssetGraphRecords {
    pub(super) conversations: Vec<conversation::Model>,
    pub(super) blueprints: Vec<novel_blueprint::Model>,
    pub(super) blueprint_versions: Vec<blueprint_version::Model>,
    pub(super) part_outlines: Vec<part_outline::Model>,
    pub(super) chapter_outlines: Vec<chapter_outline::Model>,
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) shooting_scripts: Vec<chapter_shooting_script::Model>,
    pub(super) character_profiles: Vec<protagonist_profile::Model>,
    pub(super) object_profiles: Vec<object_profile::Model>,
    pub(super) scene_profiles: Vec<scene_profile::Model>,
    pub(super) portraits: Vec<character_portrait::Model>,
    pub(super) manga_prompts: Vec<chapter_manga_prompt::Model>,
    pub(super) video_prompts: Vec<chapter_video_prompt::Model>,
    pub(super) generated_images: Vec<generated_image::Model>,
    pub(super) generated_videos: Vec<generated_video::Model>,
    pub(super) generation_specs: Vec<generation_spec::Model>,
    pub(super) prompt_artifacts: Vec<prompt_artifact::Model>,
    pub(super) media_assets: Vec<media_asset::Model>,
    pub(super) media_provenance: Vec<media_provenance::Model>,
}

impl AssetGraphRecords {
    pub(super) async fn load(
        db: &impl ConnectionTrait,
        root_project_id: i64,
        family_project_ids: Vec<i64>,
    ) -> Result<Self, AppError> {
        let RootAssetGraphRecords {
            conversations,
            blueprints,
            blueprint_versions,
            part_outlines,
            chapter_outlines,
            chapters,
            shooting_scripts,
            character_profiles,
            object_profiles,
            scene_profiles,
        } = load_root_asset_graph_records(db, root_project_id).await?;
        let FamilyAssetGraphRecords {
            portraits,
            manga_prompts,
            video_prompts,
            generated_images,
            generated_videos,
        } = load_family_asset_graph_records(db, &family_project_ids).await?;
        let GenerationAssetGraphRecords {
            generation_specs,
            prompt_artifacts,
            media_assets,
            media_provenance,
        } = load_generation_asset_graph_records(db, &family_project_ids).await?;

        Ok(Self {
            conversations,
            blueprints,
            blueprint_versions,
            part_outlines,
            chapter_outlines,
            chapters,
            shooting_scripts,
            character_profiles,
            object_profiles,
            scene_profiles,
            portraits,
            manga_prompts,
            video_prompts,
            generated_images,
            generated_videos,
            generation_specs,
            prompt_artifacts,
            media_assets,
            media_provenance,
        })
    }
}
