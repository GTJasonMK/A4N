use super::contracts::ManifestIds;

use a4n_domain::parse_canvas_cache_key;

use crate::media::{is_valid_media_binding_role, is_valid_media_binding_target_kind};
use crate::project::video_studio_package::VideoStudioProjectPackage;

pub(super) fn validate_assembly_records(
    package: &VideoStudioProjectPackage,
    ids: &ManifestIds,
) -> Result<(), String> {
    for segment in &package.video_segments {
        if segment.project_id != package.project.id || segment.root_project_id != package.project.id
        {
            return Err(format!(
                "视频片段不属于导入包 freeform 项目: video_segment_id={}",
                segment.id
            ));
        }
        if segment.source_chapter_id.is_some() || segment.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "视频画布片段不能绑定文本章节: video_segment_id={}",
                segment.id
            ));
        }
        if let Some(spec_id) = segment.source_generation_spec_id
            && !ids.generation_spec_ids.contains(&spec_id)
        {
            return Err(format!(
                "视频片段引用了不存在的生成意图: video_segment_id={}, generation_spec_id={spec_id}",
                segment.id
            ));
        }
        if let Some(asset_id) = segment.result_media_asset_id
            && !ids.media_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "视频片段引用了不存在的媒体资产: video_segment_id={}, media_asset_id={asset_id}",
                segment.id
            ));
        }
        if let Some(previous_id) = segment.previous_segment_id
            && !ids.video_segment_ids.contains(&previous_id)
        {
            return Err(format!(
                "视频片段引用了不存在的上一片段: video_segment_id={}, previous_segment_id={previous_id}",
                segment.id
            ));
        }
        if let Some(next_id) = segment.next_segment_id
            && !ids.video_segment_ids.contains(&next_id)
        {
            return Err(format!(
                "视频片段引用了不存在的下一片段: video_segment_id={}, next_segment_id={next_id}",
                segment.id
            ));
        }
    }
    for timeline in &package.assembly_timelines {
        if timeline.project_id != package.project.id {
            return Err(format!(
                "成片时间线不属于导入包项目: assembly_timeline_id={}",
                timeline.id
            ));
        }
        if timeline.source_chapter_id.is_some() || timeline.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "视频画布成片时间线不能绑定文本章节: assembly_timeline_id={}",
                timeline.id
            ));
        }
        if let Some(asset_id) = timeline.preview_media_asset_id
            && !ids.media_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "成片时间线引用了不存在的预览媒体资产: assembly_timeline_id={}, media_asset_id={asset_id}",
                timeline.id
            ));
        }
        if let Some(cache_key) = timeline.preview_cache_key.as_deref()
            && let Some((graph_id, node_id)) = parse_canvas_cache_key(cache_key)
        {
            if !ids.graph_ids.contains(&graph_id) {
                return Err(format!(
                    "成片时间线缓存键引用了不存在的画布图: assembly_timeline_id={}, graph_id={graph_id}",
                    timeline.id
                ));
            }
            if !ids.node_ids.contains(&node_id) {
                return Err(format!(
                    "成片时间线缓存键引用了不存在的画布节点: assembly_timeline_id={}, node_id={node_id}",
                    timeline.id
                ));
            }
        }
    }
    for clip in &package.assembly_clips {
        if !ids.assembly_timeline_ids.contains(&clip.timeline_id) {
            return Err(format!(
                "成片 clip 引用了不存在的时间线: assembly_clip_id={}",
                clip.id
            ));
        }
        if !ids.media_asset_ids.contains(&clip.media_asset_id) {
            return Err(format!(
                "成片 clip 引用了不存在的媒体资产: assembly_clip_id={}, media_asset_id={}",
                clip.id, clip.media_asset_id
            ));
        }
        if let Some(segment_id) = clip.source_segment_id
            && !ids.video_segment_ids.contains(&segment_id)
        {
            return Err(format!(
                "成片 clip 引用了不存在的视频片段: assembly_clip_id={}, source_segment_id={segment_id}",
                clip.id
            ));
        }
    }
    for binding in &package.media_bindings {
        if !ids.media_asset_ids.contains(&binding.media_asset_id) {
            return Err(format!(
                "媒体绑定引用了不存在的媒体资产: media_binding_id={}, media_asset_id={}",
                binding.id, binding.media_asset_id
            ));
        }
        if !is_valid_media_binding_target_kind(&binding.target_kind) {
            return Err(format!(
                "媒体绑定目标类型无效: media_binding_id={}, target_kind={}",
                binding.id, binding.target_kind
            ));
        }
        if !is_valid_media_binding_role(&binding.role) {
            return Err(format!(
                "媒体绑定角色无效: media_binding_id={}, role={}",
                binding.id, binding.role
            ));
        }
        match binding.target_kind.as_str() {
            "generated_video" if !ids.video_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的生成视频: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "assembly_timeline" if !ids.assembly_timeline_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的成片时间线: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "video_segment" if !ids.video_segment_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的视频片段: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "generated_video" | "assembly_timeline" | "video_segment" => {}
            other => {
                return Err(format!(
                    "视频画布项目包暂不支持媒体绑定目标类型: media_binding_id={}, target_kind={other}",
                    binding.id
                ));
            }
        }
    }
    Ok(())
}
