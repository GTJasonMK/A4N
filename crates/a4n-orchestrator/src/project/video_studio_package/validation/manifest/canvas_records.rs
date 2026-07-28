use super::super::*;
use super::contracts::ManifestIds;

pub(super) fn validate_canvas_records(
    package: &VideoStudioProjectPackage,
    ids: &ManifestIds,
) -> Result<(), String> {
    for graph in &package.canvas_graphs {
        if graph.project_id != package.project.id {
            return Err(format!(
                "画布图不属于导入包项目: canvas_graph_id={}",
                graph.id
            ));
        }
        if let Err(err) = validate_canvas_viewport_json(
            graph.viewport_json.as_ref(),
            "canvas_graph.viewport_json",
        ) {
            return Err(format!(
                "视频画布项目包画布视口无效: canvas_graph_id={}, {}",
                graph.id, err
            ));
        }
    }
    let node_ref_ids = CanvasNodeRefIds {
        text_asset_ids: &ids.text_asset_ids,
        generation_spec_ids: &ids.generation_spec_ids,
        prompt_artifact_ids: &ids.prompt_artifact_ids,
        image_ids: &ids.image_ids,
        video_ids: &ids.video_ids,
        media_asset_ids: &ids.media_asset_ids,
        video_segment_ids: &ids.video_segment_ids,
        assembly_timeline_ids: &ids.assembly_timeline_ids,
    };
    for node in &package.canvas_nodes {
        if !ids.graph_ids.contains(&node.graph_id) {
            return Err(format!(
                "画布节点引用了不存在的图: canvas_node_id={}",
                node.id
            ));
        }
        validate_canvas_node_ref(node, &node_ref_ids)?;
    }
    for edge in &package.canvas_edges {
        if !ids.graph_ids.contains(&edge.graph_id) {
            return Err(format!(
                "画布连线引用了不存在的图: canvas_edge_id={}",
                edge.id
            ));
        }
        if !ids.node_ids.contains(&edge.source_node_id)
            || !ids.node_ids.contains(&edge.target_node_id)
        {
            return Err(format!(
                "画布连线引用了不存在的节点: canvas_edge_id={}",
                edge.id
            ));
        }
    }
    validate_canvas_edge_contracts(package)?;
    for asset in &package.canvas_text_assets {
        if asset.project_id != package.project.id {
            return Err(format!(
                "画布文本资产不属于导入包项目: canvas_text_asset_id={}",
                asset.id
            ));
        }
    }
    Ok(())
}
