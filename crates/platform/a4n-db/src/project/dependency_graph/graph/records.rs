mod family;
mod generation;
mod root;

use a4n_error::AppError;
use sea_orm::{ConnectionTrait, FromQueryResult};

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct VideoPromptGraphRow {
    pub(in crate::project::dependency_graph) prompt_type: String,
    pub(in crate::project::dependency_graph) generated_video_id: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ConversationGraphRow {
    pub(in crate::project::dependency_graph) message_count: i32,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct BlueprintGraphRow {
    pub(in crate::project::dependency_graph) current_version: String,
    pub(in crate::project::dependency_graph) one_sentence_summary: Option<String>,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct BlueprintVersionGraphRow {
    pub(in crate::project::dependency_graph) version: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct PartOutlineGraphRow {
    pub(in crate::project::dependency_graph) start_chapter: i32,
    pub(in crate::project::dependency_graph) end_chapter: i32,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ChapterOutlineGraphRow {
    pub(in crate::project::dependency_graph) chapter_number: i32,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ChapterGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) content: String,
    pub(in crate::project::dependency_graph) word_count: i32,
    pub(in crate::project::dependency_graph) has_analysis: bool,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct VisualPlanGraphRow {
    pub(in crate::project::dependency_graph) source_chapter_id: i64,
    pub(in crate::project::dependency_graph) source_snapshot_id: Option<i64>,
    pub(in crate::project::dependency_graph) source_content_sha256: String,
    pub(in crate::project::dependency_graph) status: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct MangaPromptGraphRow {
    pub(in crate::project::dependency_graph) chapter_id: i64,
    pub(in crate::project::dependency_graph) generated_image_id: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ComicTextSourceGraphRow {
    pub(in crate::project::dependency_graph) chapter_id: i64,
    pub(in crate::project::dependency_graph) canvas_text_asset_id: i64,
    pub(in crate::project::dependency_graph) text_asset_version: i32,
    pub(in crate::project::dependency_graph) start_section_index: i32,
    pub(in crate::project::dependency_graph) end_section_index: i32,
    pub(in crate::project::dependency_graph) source_title: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct GeneratedImageGraphRow {
    pub(in crate::project::dependency_graph) image_type: String,
    pub(in crate::project::dependency_graph) metadata_json: Option<serde_json::Value>,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct GeneratedVideoGraphRow {
    pub(in crate::project::dependency_graph) status: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ShootingScriptGraphRow {
    pub(in crate::project::dependency_graph) source_chapter_id: i64,
    pub(in crate::project::dependency_graph) source_snapshot_id: Option<i64>,
    pub(in crate::project::dependency_graph) source_content_sha256: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct CharacterProfileGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) name: String,
    pub(in crate::project::dependency_graph) base_attributes: serde_json::Value,
    pub(in crate::project::dependency_graph) appearance: Option<String>,
    pub(in crate::project::dependency_graph) last_synced_chapter: i32,
}

impl CharacterProfileGraphRow {
    pub(in crate::project::dependency_graph) fn first_seen_chapter(&self) -> Option<i32> {
        a4n_db_models::models::character::protagonist_profile::first_seen_chapter_from_attributes(
            &self.base_attributes,
        )
    }
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct ObjectProfileGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) name: String,
    pub(in crate::project::dependency_graph) visual_description: String,
    pub(in crate::project::dependency_graph) first_seen_chapter: i32,
    pub(in crate::project::dependency_graph) last_seen_chapter: i32,
    pub(in crate::project::dependency_graph) importance: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct SceneProfileGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) name: String,
    pub(in crate::project::dependency_graph) visual_description: String,
    pub(in crate::project::dependency_graph) first_seen_chapter: i32,
    pub(in crate::project::dependency_graph) last_seen_chapter: i32,
    pub(in crate::project::dependency_graph) importance: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct PortraitGraphRow {
    pub(in crate::project::dependency_graph) profile_id: i64,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct GenerationSpecGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) target_kind: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct PromptArtifactGraphRow {
    pub(in crate::project::dependency_graph) prompt_kind: String,
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project::dependency_graph) struct MediaAssetGraphRow {
    pub(in crate::project::dependency_graph) id: i64,
    pub(in crate::project::dependency_graph) media_kind: String,
}

use self::{
    family::{FamilyAssetGraphRecords, load_family_asset_graph_records},
    generation::{GenerationAssetGraphRecords, load_generation_asset_graph_records},
    root::{RootAssetGraphRecords, load_root_asset_graph_records},
};

pub(super) struct AssetGraphRecords {
    pub(super) conversations: Vec<ConversationGraphRow>,
    pub(super) blueprints: Vec<BlueprintGraphRow>,
    pub(super) blueprint_versions: Vec<BlueprintVersionGraphRow>,
    pub(super) part_outlines: Vec<PartOutlineGraphRow>,
    pub(super) chapter_outlines: Vec<ChapterOutlineGraphRow>,
    pub(super) chapters: Vec<ChapterGraphRow>,
    pub(super) visual_plans: Vec<VisualPlanGraphRow>,
    pub(super) shooting_scripts: Vec<ShootingScriptGraphRow>,
    pub(super) character_profiles: Vec<CharacterProfileGraphRow>,
    pub(super) object_profiles: Vec<ObjectProfileGraphRow>,
    pub(super) scene_profiles: Vec<SceneProfileGraphRow>,
    pub(super) portraits: Vec<PortraitGraphRow>,
    pub(super) manga_prompts: Vec<MangaPromptGraphRow>,
    pub(super) comic_text_sources: Vec<ComicTextSourceGraphRow>,
    pub(super) video_prompts: Vec<VideoPromptGraphRow>,
    pub(super) generated_images: Vec<GeneratedImageGraphRow>,
    pub(super) generated_videos: Vec<GeneratedVideoGraphRow>,
    pub(super) generation_specs: Vec<GenerationSpecGraphRow>,
    pub(super) prompt_artifacts: Vec<PromptArtifactGraphRow>,
    pub(super) media_assets: Vec<MediaAssetGraphRow>,
    pub(super) media_provenance_count: u64,
}

impl AssetGraphRecords {
    pub(super) async fn load(
        db: &impl ConnectionTrait,
        root_project_id: i64,
        family_project_ids: &[i64],
    ) -> Result<Self, AppError> {
        // The three projections use disjoint tables and inputs. Overlap pool waits, then keep
        // root -> family -> generation error precedence when extracting results.
        let (root_result, family_result, generation_result) = futures::join!(
            load_root_asset_graph_records(db, root_project_id),
            load_family_asset_graph_records(db, family_project_ids),
            load_generation_asset_graph_records(db, family_project_ids),
        );
        let RootAssetGraphRecords {
            conversations,
            blueprints,
            blueprint_versions,
            part_outlines,
            chapter_outlines,
            chapters,
            visual_plans,
            shooting_scripts,
            character_profiles,
            object_profiles,
            scene_profiles,
        } = root_result?;
        let FamilyAssetGraphRecords {
            portraits,
            manga_prompts,
            comic_text_sources,
            video_prompts,
            generated_images,
            generated_videos,
        } = family_result?;
        let GenerationAssetGraphRecords {
            generation_specs,
            prompt_artifacts,
            media_assets,
            media_provenance_count,
        } = generation_result?;

        Ok(Self {
            conversations,
            blueprints,
            blueprint_versions,
            part_outlines,
            chapter_outlines,
            chapters,
            visual_plans,
            shooting_scripts,
            character_profiles,
            object_profiles,
            scene_profiles,
            portraits,
            manga_prompts,
            comic_text_sources,
            video_prompts,
            generated_images,
            generated_videos,
            generation_specs,
            prompt_artifacts,
            media_assets,
            media_provenance_count,
        })
    }
}
