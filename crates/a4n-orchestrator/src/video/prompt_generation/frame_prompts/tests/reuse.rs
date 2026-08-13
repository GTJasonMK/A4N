use super::*;

#[test]
fn segment_plan_reuse_requires_saved_source_hash() {
    let row = segment_plan_row(11, None);
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];
    let projection = shooting_script_projection();

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        duration_secs: 5,
        aspect_ratio: "16:9",
        shooting_script_projection: &projection,
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan without source hash must not be silently reused"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("metadata_json 缺少 source.content_sha256")
    );
}

#[test]
fn segment_plan_reuse_requires_metadata_json() {
    let mut row = segment_plan_row(13, Some("current-source"));
    row.metadata_json = None;
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];
    let projection = shooting_script_projection();

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        duration_secs: 5,
        aspect_ratio: "16:9",
        shooting_script_projection: &projection,
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan without metadata_json must not be silently reused"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("缺少 metadata_json"));
}

#[test]
fn segment_plan_reuse_rejects_source_mismatch() {
    let row = segment_plan_row(12, Some("old-source"));
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];
    let projection = shooting_script_projection();

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        duration_secs: 5,
        aspect_ratio: "16:9",
        shooting_script_projection: &projection,
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan from another source must not be reused"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("来源内容与当前请求不一致"));
}

#[test]
fn image_lookup_context_ignores_generation_asset_fingerprint() {
    let source_metadata = json!({
        "content_sha256": "same-source"
    });
    let mut row = row_for_boundary_frame(45, frame(1, vec![3]));
    row.metadata_json = Some(json!({
        "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
        "boundary_frame": frame(1, vec![3]),
        "source_prompt_type": "frame_segment_plan",
        "source": {
            "content_sha256": "same-source"
        },
        "generation_asset_fingerprint": {
            "schema_version": 1,
            "reference_image_ids": [3],
            "video_library_image_ids": [],
            "available_assets_sha256": "old-assets"
        },
        "prompt_output_language": "zh_cn"
    }));
    let segment = segment(1);
    let recipe = a4n_core::video_execution::default_video_segment_recipes(
        std::slice::from_ref(&segment),
        "测试风格",
        "zh_cn",
    )
    .unwrap()
    .remove(0);
    let execution_identity = video_segment_frame_execution_identity(
        video_execution_chain_identity(
            &shooting_script_projection(),
            std::slice::from_ref(&segment),
            "测试风格",
            5,
            "16:9",
        )
        .unwrap(),
        Some(44),
        Some(&recipe),
        (&segment.source_shot_id, segment.segment_index_in_shot),
    )
    .unwrap();
    row.metadata_json
        .as_mut()
        .expect("frame metadata")
        .as_object_mut()
        .expect("frame metadata object")
        .insert(
            "video_execution_identity".to_string(),
            execution_identity.clone(),
        );

    assert!(frame_row_matches_image_lookup_context(
        &row,
        &FrameImageLookupContext {
            workflow_mode: PromptWorkflowMode::BoundaryFrames,
            prompt_type: "frame_prompt",
            source_metadata: &source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            video_execution_identity: &execution_identity,
            source_prompt_type: "frame_segment_plan",
            prompt_output_language: PromptOutputLanguage::ZhCn,
        },
    ));
}

#[test]
fn video_prompt_reference_ids_are_system_derived_from_generation_and_frames() {
    let segments = vec![segment(1), segment(2)];
    let frame_by_number = HashMap::from([
        (1, frame(1, vec![11])),
        (2, frame(2, vec![12])),
        (3, frame(3, vec![13, 11])),
    ]);
    let frame_image_ids_by_number = HashMap::from([(1, 101), (2, 102), (3, 103)]);

    let enriched_segments = video_prompt_segments_with_reference_ids(
        PromptWorkflowMode::BoundaryFrames,
        &segments,
        &frame_by_number,
        &frame_image_ids_by_number,
        &[99, 12],
    );

    assert_eq!(segments[0].reference_image_ids, vec![3, 9]);
    assert_eq!(
        enriched_segments[0].reference_image_ids,
        vec![101, 102, 11, 12, 99]
    );
    assert_eq!(
        enriched_segments[1].reference_image_ids,
        vec![102, 103, 12, 13, 11, 99]
    );
}

#[test]
fn trims_reusable_frame_prefix_when_previous_image_is_missing_from_prompt_refs() {
    let frames = vec![frame(1, vec![3, 9]), frame(2, vec![3, 9])];
    let image_id_by_frame_number = HashMap::from([(1, 13)]);

    let keep_len =
        reusable_frame_prefix_len_with_previous_images(&frames, &image_id_by_frame_number);

    assert_eq!(keep_len, 1);
}

#[test]
fn reusable_boundary_frame_prefix_stops_before_stale_reference_image() {
    let segment_plan = vec![segment(1), segment(2)];
    let rows = vec![
        row_for_boundary_frame(45, boundary_frame(1, vec![3, 9], vec![1])),
        row_for_boundary_frame(50, boundary_frame(2, vec![3, 9, 13], vec![1, 2])),
    ];
    let allowed = HashSet::from([3, 9]);

    let prefix =
        boundary_frame_prompt_rows_prefix(rows, &segment_plan, &allowed, &HashMap::new()).unwrap();

    assert_eq!(prefix.len(), 1);
    assert_eq!(prefix[0].id, 45);
}
