fn test_generation_asset_fingerprint() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "available_assets_sha256": "test-assets"
    })
}

fn video_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": "boundary_frame_incremental_v1",
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    })
}

fn first_frame_prompt_metadata_with_language(language: &str) -> serde_json::Value {
    json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same"},
        "contract_version": FIRST_FRAME_CHAIN_CONTRACT_VERSION,
        "prompt_output_language": language,
        "generation_asset_fingerprint": test_generation_asset_fingerprint()
    })
}
