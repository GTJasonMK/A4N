use super::super::*;

pub(super) fn validate_galgame_media_binding_contract(
    binding: &media_binding::Model,
    asset: &media_asset::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    galgame_scenes: &HashMap<i64, &galgame_scene::Model>,
    galgame_lines: &HashMap<i64, &galgame_line::Model>,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    if !is_valid_galgame_media_binding_pair(&binding.target_kind, &binding.role) {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定目标/角色无效: media_binding_id={}, target_kind={}, role={}",
            binding.id, binding.target_kind, binding.role
        ));
    }
    if let Some(required_media_kind) =
        galgame_media_binding_required_media_kind(&binding.target_kind, &binding.role)
        && asset.media_kind != required_media_kind
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产类型不匹配: media_binding_id={}, media_asset_id={}, target_kind={}, role={}, required_media_kind={}, actual_media_kind={}",
            binding.id,
            asset.id,
            binding.target_kind,
            binding.role,
            required_media_kind,
            asset.media_kind
        ));
    }
    if matches!(
        binding.target_kind.as_str(),
        "galgame_scene" | "galgame_line"
    ) && (asset.status != MEDIA_STATUS_COMPLETED || asset.file_path.is_none())
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产不是可用已完成媒体: media_binding_id={}, media_asset_id={}",
            binding.id, asset.id
        ));
    }
    match binding.target_kind.as_str() {
        "galgame_scene" => {
            let scene = galgame_scenes.get(&binding.target_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标场不存在: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                )
            })?;
            validate_galgame_media_binding_project(
                binding,
                asset,
                scene,
                galgame_scripts,
                snapshot_chapters,
            )?;
        }
        "galgame_line" => {
            let line = galgame_lines.get(&binding.target_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标行不存在: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                )
            })?;
            let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 媒体绑定目标行所属场不存在: media_binding_id={}, target_id={}, scene_id={}",
                    binding.id, binding.target_id, line.scene_id
                )
            })?;
            validate_galgame_media_binding_project(
                binding,
                asset,
                scene,
                galgame_scripts,
                snapshot_chapters,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_galgame_media_binding_project(
    binding: &media_binding::Model,
    asset: &media_asset::Model,
    scene: &galgame_scene::Model,
    galgame_scripts: &HashMap<i64, &galgame_script::Model>,
    snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 媒体绑定目标场引用了不存在的剧本: media_binding_id={}, scene_id={}, script_id={}",
            binding.id, scene.id, scene.script_id
        )
    })?;
    if asset.project_id != script.project_id || asset.root_project_id != script.root_project_id {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产与目标项目不一致: media_binding_id={}, media_asset_id={}, asset_project_id={}, target_project_id={}",
            binding.id, asset.id, asset.project_id, script.project_id
        ));
    }
    let chapter = snapshot_chapters
        .get(&scene.source_snapshot_chapter_id)
        .ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 媒体绑定目标场源快照章节不存在: scene_id={}",
                scene.id
            )
        })?;
    if asset.source_snapshot_id != Some(chapter.snapshot_id) {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产源快照与目标不一致: media_binding_id={}, media_asset_id={}, asset_source_snapshot_id={:?}, target_source_snapshot_id={}",
            binding.id, asset.id, asset.source_snapshot_id, chapter.snapshot_id
        ));
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && source_snapshot_chapter_id != scene.source_snapshot_chapter_id
    {
        return Err(format!(
            "项目族导入包 Galgame 媒体绑定资产源快照章节与目标不一致: media_binding_id={}, media_asset_id={}, asset_source_snapshot_chapter_id={}, target_source_snapshot_chapter_id={}",
            binding.id, asset.id, source_snapshot_chapter_id, scene.source_snapshot_chapter_id
        ));
    }
    Ok(())
}
pub(super) fn validate_media_binding_target_ref(
    binding: &media_binding::Model,
    chapter_ids: &HashSet<i64>,
    character_profile_ids: &HashSet<i64>,
    object_profile_ids: &HashSet<i64>,
    scene_profile_ids: &HashSet<i64>,
    chapter_video_prompt_ids: &HashSet<i64>,
    generated_video_ids: &HashSet<i64>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
    galgame_scene_ids: &HashSet<i64>,
    galgame_line_ids: &HashSet<i64>,
) -> Result<(), String> {
    let exists = match binding.target_kind.as_str() {
        "character_profile" => character_profile_ids.contains(&binding.target_id),
        "object_profile" => object_profile_ids.contains(&binding.target_id),
        "scene_profile" => scene_profile_ids.contains(&binding.target_id),
        "chapter_video_prompt" => chapter_video_prompt_ids.contains(&binding.target_id),
        "generated_video" => generated_video_ids.contains(&binding.target_id),
        "video_segment" => video_segment_ids.contains(&binding.target_id),
        "chapter" => chapter_ids.contains(&binding.target_id),
        "assembly_timeline" => assembly_timeline_ids.contains(&binding.target_id),
        "galgame_scene" => galgame_scene_ids.contains(&binding.target_id),
        "galgame_line" => galgame_line_ids.contains(&binding.target_id),
        _ => return Ok(()),
    };
    ensure_package!(
        exists,
        "项目族导入包媒体绑定目标不存在: media_binding_id={}, target_kind={}, target_id={}",
        binding.id,
        binding.target_kind,
        binding.target_id
    );
    Ok(())
}
