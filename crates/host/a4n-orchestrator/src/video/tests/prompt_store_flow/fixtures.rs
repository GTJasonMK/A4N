fn test_generation_asset_fingerprint() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "available_assets_sha256": "test-assets"
    })
}

fn stable_prompt_metadata(mut metadata: serde_json::Value, shot_number: i32) -> serde_json::Value {
    let object = metadata
        .as_object_mut()
        .expect("prompt fixture metadata must be an object");
    object
        .entry("prompt_output_language")
        .or_insert_with(|| serde_json::json!("zh_cn"));
    object
        .entry("video_execution_identity")
        .or_insert_with(|| serde_json::json!({
            "segment_plan_prompt_id": 100,
            "source_shot_id": format!("shot-{shot_number}"),
            "segment_index_in_shot": 1,
            "segment_sha256": format!("segment-{shot_number}"),
            "segment_recipe_fingerprint": format!("recipe-{shot_number}")
        }));
    metadata
}

fn video_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    stable_prompt_metadata(json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": "boundary_frame_incremental_v1",
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    }), 1)
}

fn first_frame_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    stable_prompt_metadata(json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    }), 1)
}
