use a4n_db_models::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use a4n_domain::canvas::{
    CanvasEdgeRecord, CanvasGraphRecord, CanvasNodeRecord, CanvasTextAssetRecord,
};

pub(crate) fn canvas_graph_record(row: canvas_graph::Model) -> CanvasGraphRecord {
    CanvasGraphRecord {
        id: row.id,
        project_id: row.project_id,
        name: row.name,
        graph_kind: row.graph_kind,
        version: row.version,
        viewport_json: row.viewport_json,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(crate) fn canvas_node_record(row: canvas_node::Model) -> CanvasNodeRecord {
    CanvasNodeRecord {
        id: row.id,
        graph_id: row.graph_id,
        node_kind: row.node_kind,
        ref_type: row.ref_type,
        ref_id: row.ref_id,
        config_json: row.config_json,
        position_json: row.position_json,
        state: row.state,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(crate) fn canvas_edge_record(row: canvas_edge::Model) -> CanvasEdgeRecord {
    CanvasEdgeRecord {
        id: row.id,
        graph_id: row.graph_id,
        source_node_id: row.source_node_id,
        source_port: row.source_port,
        target_node_id: row.target_node_id,
        target_port: row.target_port,
        role: row.role,
        config_json: row.config_json,
    }
}

pub(crate) fn canvas_text_asset_record(row: canvas_text_asset::Model) -> CanvasTextAssetRecord {
    CanvasTextAssetRecord {
        id: row.id,
        project_id: row.project_id,
        title: row.title,
        text_kind: row.text_kind,
        content: row.content,
        content_hash: row.content_hash,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_record_preserves_media_reference_and_payload() {
        let now = chrono::Utc::now().naive_utc();
        let model = canvas_node::Model {
            id: 4,
            graph_id: 2,
            node_kind: "VideoInputNode".to_string(),
            ref_type: Some("generated_video".to_string()),
            ref_id: Some(7),
            config_json: json!({"canvas_runtime_result": true}),
            position_json: json!({"x": 1, "y": 2}),
            state: "ready".to_string(),
            created_at: now,
            updated_at: now,
        };

        let record = canvas_node_record(model.clone());
        assert_eq!(
            serde_json::to_value(record).unwrap(),
            serde_json::to_value(model).unwrap()
        );
    }
}
