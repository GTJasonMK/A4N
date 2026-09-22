use crate::project::dependency_graph::AssetLineageRecord;

pub(super) type GenerationTargetKey = (&'static str, i64);

pub(super) fn generation_target_key(record: &AssetLineageRecord) -> Option<GenerationTargetKey> {
    let target_id = record.asset_id?;
    let target_table = generation_target_table(record.asset_kind.as_str())?;
    Some((target_table, target_id))
}

pub(super) fn generation_target_table(asset_kind: &str) -> Option<&'static str> {
    match asset_kind {
        "novel_blueprint" => Some("novel_blueprint"),
        "part_outline" => Some("part_outline"),
        "chapter_outline" => Some("chapter_outline"),
        "chapter_content" => Some("chapter_content"),
        "chapter_analysis" => Some("chapter_analysis"),
        "character_profile_record" => Some("protagonist_profile"),
        "object_profile_record" => Some("object_profile"),
        "scene_profile_record" => Some("scene_profile"),
        "character_portrait" => Some("character_portrait"),
        "chapter_manga_prompt" => Some("chapter_manga_prompt"),
        "manga_page_image" | "profile_material_image" | "video_frame_image" => {
            Some("generated_image")
        }
        "video_frame_prompt" | "chapter_video_prompt" => Some("chapter_video_prompt"),
        "generated_video" => Some("generated_video"),
        "prompt_artifact" => Some("prompt_artifact"),
        "media_asset" => Some("media_asset"),
        _ => None,
    }
}

pub(super) fn has_structural_lineage(record: &AssetLineageRecord) -> bool {
    !record.source_refs.is_empty()
        || !record.prompt_sources.is_empty()
        || !record.media_sources.is_empty()
        || record.request_snapshot.is_some()
}

pub(super) fn should_expect_generation_inputs(record: &AssetLineageRecord) -> bool {
    matches!(
        record.origin.as_str(),
        "llm_generated" | "image_generated" | "video_generated"
    ) && matches!(
        record.asset_kind.as_str(),
        "novel_blueprint"
            | "part_outline"
            | "chapter_outline"
            | "chapter_content"
            | "chapter_analysis"
            | "character_profile_record"
            | "object_profile_record"
            | "scene_profile_record"
            | "character_portrait"
            | "profile_material_image"
            | "manga_page_image"
            | "video_frame_image"
            | "chapter_manga_prompt"
            | "video_frame_prompt"
            | "chapter_video_prompt"
            | "generated_video"
            | "prompt_artifact"
    )
}
