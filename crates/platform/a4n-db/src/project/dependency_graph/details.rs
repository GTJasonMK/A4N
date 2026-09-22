mod generation;

mod manga;
mod profiles;
mod records;
mod story;
mod video;
mod visual_plan;

pub(super) use self::{
    generation::{
        generation_spec_detail_records, inputs::enrich_lineage_records_with_generation_inputs,
        media_asset_detail_records, prompt_artifact_detail_records,
    },
    manga::{manga_image_detail_records, manga_prompt_detail_records},
    profiles::{
        character_portrait_detail_records, character_profile_detail_records,
        object_profile_detail_records, profile_material_image_detail_records,
        scene_profile_detail_records,
    },
    records::{generic_node_detail_records, structural_source_refs_for_node},
    story::{
        blueprint_detail_records, blueprint_version_detail_records,
        chapter_analysis_detail_records, chapter_content_detail_records,
        chapter_outline_detail_records, part_outline_detail_records,
    },
    video::{
        generated_video_detail_records, video_frame_image_detail_records,
        video_prompt_detail_records,
    },
    visual_plan::{shooting_script_detail_records, visual_plan_detail_records},
};
