mod contract;
mod document;
mod order;

pub(super) use contract::validate_canvas_record_edge_contract;
pub use document::validate_canvas_record_document_edge_contract;
pub(super) use order::{
    canvas_record_edge_order_index, validate_canvas_record_contiguous_order_indexes,
};

#[cfg(test)]
mod tests;
