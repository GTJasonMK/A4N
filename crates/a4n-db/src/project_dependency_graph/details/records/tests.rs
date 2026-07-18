use super::*;
use serde_json::json;

#[test]
fn video_prompt_asset_kind_preserves_graph_layer_contract() {
    assert_eq!(
        video_prompt_asset_kind("frame_segment_plan"),
        "video_segment_plan"
    );
    assert_eq!(
        video_prompt_asset_kind("frame_segment_plan"),
        "video_segment_plan"
    );
    assert_eq!(
        video_prompt_asset_kind("first_frame_prompt"),
        "video_frame_prompt"
    );
    assert_eq!(
        video_prompt_asset_kind("video_prompt"),
        "chapter_video_prompt"
    );
}

#[test]
fn generated_video_prompt_id_accepts_supported_metadata_shapes() {
    assert_eq!(
        generated_video_prompt_id(Some(&json!({ "prompt_id": 11 }))),
        Some(11)
    );
    assert_eq!(
        generated_video_prompt_id(Some(&json!({ "metadata": { "prompt_id": 12 } }))),
        Some(12)
    );
    assert_eq!(
        generated_video_prompt_id(Some(&json!({ "generation_context": { "prompt_id": 13 } }))),
        Some(13)
    );
    assert_eq!(
        generated_video_prompt_id(Some(&json!({ "prompt_id": 0 }))),
        None
    );
    assert_eq!(generated_video_prompt_id(None), None);
}
