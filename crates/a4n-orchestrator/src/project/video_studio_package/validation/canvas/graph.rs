use crate::canvas::{
    CanvasGraphRecordDocument as CanvasGraphDocument,
    validate_canvas_record_document_edge_contract as validate_canvas_document_edge_contract,
    validate_canvas_record_document_is_dag as validate_canvas_document_is_dag,
};

use crate::project::video_studio_package::VideoStudioProjectPackage;

pub(in crate::project::video_studio_package::validation) fn validate_canvas_edge_contracts(
    package: &VideoStudioProjectPackage,
) -> Result<(), String> {
    for graph in &package.canvas_graphs {
        let document = CanvasGraphDocument {
            graph: graph.clone(),
            nodes: package
                .canvas_nodes
                .iter()
                .filter(|node| node.graph_id == graph.id)
                .cloned()
                .collect(),
            edges: package
                .canvas_edges
                .iter()
                .filter(|edge| edge.graph_id == graph.id)
                .cloned()
                .collect(),
            text_assets: package.canvas_text_assets.clone(),
        };
        validate_canvas_document_edge_contract(&document).map_err(|err| {
            format!(
                "视频画布项目包画布连线合同无效: canvas_graph_id={}, {}",
                graph.id, err
            )
        })?;
        validate_canvas_document_is_dag(&document).map_err(|err| {
            format!(
                "视频画布项目包画布结构无效: canvas_graph_id={}, {}",
                graph.id, err
            )
        })?;
    }
    Ok(())
}
