use super::super::types::{EmbeddingRow, ImageRow, ImportAllConfigsInput, LlmRow, VideoRow};
use super::*;
use serde_json::{Value, json};

fn full_export_json() -> Value {
    json!({
        "version": "1.0",
        "export_time": "2026-05-27T00:00:00Z",
        "export_type": "all",
        "llm_configs": [],
        "embedding_configs": [],
        "image_configs": [],
        "video_configs": [],
        "prompts_modified": [],
        "themes": [],
        "system_settings": []
    })
}
