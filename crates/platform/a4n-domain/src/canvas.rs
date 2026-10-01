mod references;
mod transfer;

pub use references::*;
pub use transfer::*;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasTextAssetRecordInput {
    pub id: Option<i64>,
    pub title: String,
    pub text_kind: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasGraphRecord {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub graph_kind: String,
    pub version: i32,
    pub viewport_json: Option<Value>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasNodeRecord {
    pub id: i64,
    pub graph_id: i64,
    pub node_kind: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    pub config_json: Value,
    pub position_json: Value,
    pub state: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasEdgeRecord {
    pub id: i64,
    pub graph_id: i64,
    pub source_node_id: i64,
    pub source_port: String,
    pub target_node_id: i64,
    pub target_port: String,
    pub role: String,
    pub config_json: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasTextAssetRecord {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub text_kind: String,
    pub content: String,
    pub content_hash: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasPromptArtifactRecord {
    pub id: i64,
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasGraphRecordDocument {
    pub graph: CanvasGraphRecord,
    pub nodes: Vec<CanvasNodeRecord>,
    pub edges: Vec<CanvasEdgeRecord>,
    pub text_assets: Vec<CanvasTextAssetRecord>,
}

#[derive(Debug, Clone)]
pub struct CanvasPromptArtifactUpsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
}

#[derive(Debug, Clone)]
pub struct FreeformCanvasGenerationSpecUpsertInput {
    pub project_id: i64,
    pub target_entity_id: i64,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub status: String,
    pub params_json: Value,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblySourceVideoInput {
    pub video_id: i64,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblyContractSyncInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub assembly_video_id: i64,
    pub source_videos: Vec<CanvasAssemblySourceVideoInput>,
    pub generation_spec_id: Option<i64>,
}

/// 画布成片预览缓存键的唯一格式。
pub fn canvas_cache_key(graph_id: i64, node_id: i64) -> String {
    format!("canvas_graph:{graph_id}:node:{node_id}")
}

/// 解析画布成片预览缓存键。
pub fn parse_canvas_cache_key(value: &str) -> Option<(i64, i64)> {
    let mut parts = value.split(':');
    match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some("canvas_graph"), Some(graph_id), Some("node"), Some(node_id), None) => {
            Some((graph_id.parse().ok()?, node_id.parse().ok()?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CanvasNodeRefKind, canvas_cache_key, canvas_transfer_runtime_identity_key,
        clear_canvas_transfer_runtime_identity, parse_canvas_cache_key,
        parse_generated_media_reference,
    };
    use serde_json::json;

    #[test]
    fn canvas_node_ref_kind_accepts_only_canonical_values() {
        assert_eq!(
            "generated_image".parse::<CanvasNodeRefKind>().unwrap(),
            CanvasNodeRefKind::GeneratedImage
        );
        assert_eq!(
            "generated_video".parse::<CanvasNodeRefKind>().unwrap(),
            CanvasNodeRefKind::GeneratedVideo
        );
        assert!(
            "legacy_generated_image"
                .parse::<CanvasNodeRefKind>()
                .is_err()
        );
        assert!(" generated_video".parse::<CanvasNodeRefKind>().is_err());
    }

    #[test]
    fn canvas_cache_key_has_one_canonical_shape() {
        assert_eq!(canvas_cache_key(1, 2), "canvas_graph:1:node:2");
        assert_eq!(
            parse_canvas_cache_key("canvas_graph:1:node:2"),
            Some((1, 2))
        );
        assert_eq!(parse_canvas_cache_key("canvas_graph:1:node:2:extra"), None);
        assert_eq!(parse_canvas_cache_key("custom-cache-key"), None);
        assert_eq!(
            parse_generated_media_reference("generated_video:7").unwrap(),
            Some(super::GeneratedMediaReference::Video(7))
        );
        assert!(parse_generated_media_reference("generated_image:no-id").is_err());
    }

    #[test]
    fn canvas_transfer_runtime_identity_is_removed_recursively() {
        let mut value = json!({
            "config_id": 1,
            "canvas": {"workflow_run_id": 2, "node_id": 3},
            "metadata": [{"provider_job_id": "job", "generated_video_id": 4}]
        });

        assert_eq!(
            canvas_transfer_runtime_identity_key(&value),
            Some("config_id")
        );
        clear_canvas_transfer_runtime_identity(&mut value);

        assert_eq!(canvas_transfer_runtime_identity_key(&value), None);
        assert_eq!(value["canvas"]["node_id"], json!(3));
        assert_eq!(value["metadata"][0]["generated_video_id"], json!(4));
    }
}
