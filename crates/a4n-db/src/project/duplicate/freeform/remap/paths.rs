use super::super::FreeformCopyMaps;
use crate::project::duplicate::ProjectDuplicateError;

pub(in crate::project::duplicate::freeform) fn remap_freeform_source_external_path(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, ProjectDuplicateError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if let Some(id) = raw
        .strip_prefix("generated_video:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = maps.videos.ids.get(&id).copied().ok_or_else(|| {
            ProjectDuplicateError::InvalidState {
                current: format!("generated_video:{id} 缺少目标映射"),
                operation: "复制媒体来源路径".to_string(),
            }
        })?;
        return Ok(Some(format!("generated_video:{new_id}")));
    }
    if let Some(id) = raw
        .strip_prefix("generated_image:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = maps.images.ids.get(&id).copied().ok_or_else(|| {
            ProjectDuplicateError::InvalidState {
                current: format!("generated_image:{id} 缺少目标映射"),
                operation: "复制媒体来源路径".to_string(),
            }
        })?;
        return Ok(Some(format!("generated_image:{new_id}")));
    }
    Ok(Some(raw.to_string()))
}

pub(in crate::project::duplicate::freeform) fn remap_freeform_canvas_cache_key(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, ProjectDuplicateError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let parts = raw.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        let new_graph_id = maps.graph_ids.get(&graph_id).copied().ok_or_else(|| {
            ProjectDuplicateError::InvalidState {
                current: format!("preview_cache_key graph_id={graph_id} 缺少目标映射"),
                operation: "复制成片时间线缓存键".to_string(),
            }
        })?;
        let new_node_id = maps.node_ids.get(&node_id).copied().ok_or_else(|| {
            ProjectDuplicateError::InvalidState {
                current: format!("preview_cache_key node_id={node_id} 缺少目标映射"),
                operation: "复制成片时间线缓存键".to_string(),
            }
        })?;
        return Ok(Some(format!(
            "canvas_graph:{new_graph_id}:node:{new_node_id}"
        )));
    }
    Ok(Some(raw.to_string()))
}
