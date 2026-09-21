use std::collections::HashMap;

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_required_id};

pub(super) fn remap_derived_media_binding_target(
    source_project_id: i64,
    target_kind: &str,
    target_id: i64,
    segment_id_map: &HashMap<i64, i64>,
    timeline_id_map: &HashMap<i64, i64>,
    video_id_map: Option<&HashMap<i64, i64>>,
) -> Result<i64, ProjectDuplicateError> {
    match target_kind {
        "video_segment" => {
            remap_derived_required_id(segment_id_map, target_id, "media_binding.target_id")
        }
        "assembly_timeline" => {
            remap_derived_required_id(timeline_id_map, target_id, "media_binding.target_id")
        }
        "generated_video" => remap_derived_required_id(
            video_id_map.ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("source_project_id={source_project_id} 缺少 generated_video 映射"),
                operation: "复制 generated_video 媒体绑定".to_string(),
            })?,
            target_id,
            "media_binding.target_id",
        ),
        "character_profile" | "object_profile" | "scene_profile" | "chapter" => Ok(target_id),
        other => Err(ProjectDuplicateError::InvalidArgument(format!(
            "不支持复制的媒体绑定目标类型: source_project_id={source_project_id}, target_kind={other}"
        ))),
    }
}
