mod persistence;
mod references;
mod types;
mod validation;

pub use persistence::{
    get_or_create_canvas_graph_record, save_canvas_graph_record, upsert_canvas_text_asset_record,
};
pub use types::{
    CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
    CanvasNodeRecordInput, CanvasTextAssetRecordInput,
};
pub use validation::{
    validate_canvas_graph_record_input, validate_canvas_record_document_edge_contract,
    validate_canvas_record_document_is_dag, validate_canvas_record_viewport_json,
};

#[cfg(test)]
use a4n_domain::CanvasNodeRefKind;
#[cfg(test)]
use persistence::GRAPH_KIND_VIDEO_STUDIO;
#[cfg(test)]
use validation::validate_canvas_record_node_ref_shape;

#[cfg(test)]
mod tests;
