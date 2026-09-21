use super::super::FreeformCopyMaps;
use crate::project::duplicate::ProjectDuplicateError;
use a4n_domain::{
    GeneratedMediaReference, canvas_cache_key, parse_canvas_cache_key,
    parse_generated_media_reference,
};

pub(in crate::project::duplicate::freeform) fn remap_freeform_source_external_path(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, ProjectDuplicateError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let Some(reference) = parse_generated_media_reference(raw).ok().flatten() else {
        return Ok(Some(raw.to_string()));
    };
    let ids = match reference {
        GeneratedMediaReference::Video(_) => &maps.videos.ids,
        GeneratedMediaReference::Image(_) => &maps.images.ids,
    };
    let id = reference.id();
    let new_id = ids
        .get(&id)
        .copied()
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("{reference} 缺少目标映射"),
            operation: "复制媒体来源路径".to_string(),
        })?;
    Ok(Some(reference.with_id(new_id).to_string()))
}

pub(in crate::project::duplicate::freeform) fn remap_freeform_canvas_cache_key(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, ProjectDuplicateError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if let Some((graph_id, node_id)) = parse_canvas_cache_key(raw) {
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
        return Ok(Some(canvas_cache_key(new_graph_id, new_node_id)));
    }
    Ok(Some(raw.to_string()))
}
