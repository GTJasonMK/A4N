#[test]
fn validate_assembly_workflow_mode_accepts_known_modes() {
    assert_eq!(
        validate_assembly_workflow_mode("boundary_frames"),
        Ok("boundary_frames")
    );
    assert_eq!(
        validate_assembly_workflow_mode(" first_frame_chain "),
        Ok("first_frame_chain")
    );
    assert_eq!(
        validate_assembly_workflow_mode("reference_media"),
        Ok("reference_media")
    );
    assert_eq!(
        validate_assembly_workflow_mode("chat_video"),
        Ok("chat_video")
    );
}

#[test]
fn validate_assembly_workflow_mode_rejects_blank_or_unknown() {
    assert!(validate_assembly_workflow_mode(" ").is_err());
    assert!(validate_assembly_workflow_mode("unknown").is_err());
}

#[test]
fn assembly_clip_shot_number_requires_positive_integer_metadata() {
    assert_eq!(
        assembly_clip_shot_number(&video_with_metadata(1, Some(json!({ "shot_number": 3 }))))
            .expect("valid shot number"),
        3
    );

    for metadata in [
        None,
        Some(json!({})),
        Some(json!({ "shot_number": 0 })),
        Some(json!({ "shot_number": "3" })),
    ] {
        let err = assembly_clip_shot_number(&video_with_metadata(2, metadata))
            .expect_err("invalid shot number should fail");
        assert!(err.contains("缺少有效 shot_number"));
    }
}

#[test]
fn assembly_clip_sort_order_supports_insert_clips_between_shots() {
    let first = video_with_metadata(1, Some(json!({ "shot_number": 1 })));
    let second = video_with_metadata(2, Some(json!({ "shot_number": 2 })));
    let mut insert = video_with_metadata(
        3,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": 1,
                "insert_after_shot_number": 1
            }
        })),
    );
    insert.video_type = "assembly_insert".to_string();

    let mut ordered = vec![second, insert, first];
    ordered.sort_by(|left, right| {
        assembly_clip_sort_order(left)
            .expect("valid left order")
            .total_cmp(&assembly_clip_sort_order(right).expect("valid right order"))
    });

    assert_eq!(
        ordered
            .into_iter()
            .map(|video| video.id)
            .collect::<Vec<_>>(),
        vec![1, 3, 2]
    );
}

#[test]
fn assembly_clip_sort_order_rejects_malformed_insert_metadata() {
    let mut bad_insert_anchor = video_with_metadata(
        3,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": "1",
                "insert_after_shot_number": 1
            }
        })),
    );
    bad_insert_anchor.video_type = "assembly_insert".to_string();

    let err = assembly_clip_sort_order(&bad_insert_anchor)
        .expect_err("invalid insert_after_video_id should fail explicitly");
    assert!(err.contains("insert_after_video_id"));
    assert!(err.contains("必须为空或正整数"));

    let mut bad_insert_shot = video_with_metadata(
        4,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": 1,
                "insert_after_shot_number": 0
            }
        })),
    );
    bad_insert_shot.video_type = "assembly_insert".to_string();

    let err = assembly_clip_sort_order(&bad_insert_shot)
        .expect_err("invalid insert_after_shot_number should fail explicitly");
    assert!(err.contains("insert_after_shot_number"));
    assert!(err.contains("必须为空或正整数"));
}

#[test]
fn assembly_insert_metadata_with_position_rejects_malformed_base_metadata() {
    let err = assembly_insert_metadata_with_position(Some(json!("legacy")), 500.0, None, None)
        .expect_err("non-object metadata should fail");
    assert!(err.contains("metadata 必须是对象"));

    let err = assembly_insert_metadata_with_position(
        Some(json!({ "assembly_insert": "legacy" })),
        500.0,
        Some(1),
        Some(1),
    )
    .expect_err("non-object assembly_insert should fail");
    assert!(err.contains("assembly_insert metadata 必须是对象"));
}

#[test]
fn next_assembly_insert_sort_order_places_clip_after_anchor() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let (order, shot_number) =
        next_assembly_insert_sort_order(&clips, Some(1)).expect("anchor exists");

    assert!(order > clips[0].sort_order);
    assert!(order < clips[1].sort_order);
    assert_eq!(shot_number, Some(1));
}

#[test]
fn next_assembly_insert_sort_order_places_clip_before_first_without_anchor() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let (order, shot_number) =
        next_assembly_insert_sort_order(&clips, None).expect("start insert should be valid");

    assert!(order < clips[0].sort_order);
    assert_eq!(shot_number, None);
}

#[test]
fn next_assembly_insert_sort_order_rejects_missing_anchor() {
    let clips = vec![assembly_clip(1, 1280, 720, 24)];

    let err =
        next_assembly_insert_sort_order(&clips, Some(999)).expect_err("missing anchor should fail");

    assert!(err.contains("不在当前成片片段中"));
}

#[test]
fn assembly_push_rows_parse_completed_push_metadata_in_order() {
    let mut failed_push = assembly_push_video(99, 3, "pending", vec![9]);
    failed_push.status = "failed".to_string();
    let rows = vec![
        assembly_push_video(12, 2, "rejected", vec![3]),
        failed_push,
        assembly_push_video(11, 1, "accepted", vec![1, 2]),
        assembly_push_video(13, 3, "pending", vec![4]),
    ];

    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    assert_eq!(
        pushes.iter().map(|push| push.video.id).collect::<Vec<_>>(),
        vec![11, 12, 13]
    );
    assert_eq!(active_assembly_push_source_ids(&pushes), vec![1, 2, 4]);
}

#[test]
fn assembly_push_rows_reject_malformed_completed_push_metadata() {
    let mut missing_status = assembly_push_video(100, 4, "pending", vec![4]);
    if let Some(metadata) = missing_status.metadata_json.as_mut()
        && let Some(push) = metadata
            .get_mut("assembly_push")
            .and_then(serde_json::Value::as_object_mut)
    {
        push.remove("status");
    }
    let missing_status_err = assembly_push_rows(&[missing_status], "first_frame_chain")
        .expect_err("missing status must not become pending");
    assert!(missing_status_err.contains("缺少 assembly_push.status"));

    let invalid_status = assembly_push_video(101, 5, "done", vec![5]);
    let invalid_status_err = assembly_push_rows(&[invalid_status], "first_frame_chain")
        .expect_err("invalid status must fail visibly");
    assert!(invalid_status_err.contains("assembly_push.status 无效"));

    let empty_sources = {
        let mut video = video_with_metadata(
            102,
            Some(json!({
                "assembly_push": {
                    "status": "pending",
                    "push_index": 6,
                    "source_video_ids": []
                }
            })),
        );
        video.video_type = "assembly_push".to_string();
        video
    };
    let empty_sources_err = assembly_push_rows(&[empty_sources], "first_frame_chain")
        .expect_err("empty source ids must fail visibly");
    assert!(empty_sources_err.contains("source_video_ids 不能为空"));
}

#[test]
fn ensure_active_pushes_match_clip_prefix_accepts_contiguous_prefix() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
        assembly_clip(3, 1280, 720, 24),
    ];

    ensure_active_pushes_match_clip_prefix(&clips, &[1, 2])
        .expect("active push ids should match the clip prefix");
}

#[test]
fn ensure_active_pushes_match_clip_prefix_rejects_gap_or_overflow() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let gap = ensure_active_pushes_match_clip_prefix(&clips, &[1, 3])
        .expect_err("gap should break pushed prefix continuity");
    assert!(gap.contains("不连续"));

    let overflow = ensure_active_pushes_match_clip_prefix(&clips, &[1, 2, 3])
        .expect_err("pushed source count cannot exceed current clips");
    assert!(overflow.contains("超过当前成片片段数量"));
}

#[test]
fn selected_assembly_push_source_clips_limits_range_by_end_video_id() {
    let pending = vec![
        assembly_clip(3, 1280, 720, 24),
        assembly_clip(4, 1280, 720, 24),
        assembly_clip(5, 1280, 720, 24),
    ];

    let single = selected_assembly_push_source_clips(&pending, Some(3))
        .expect("first pending clip can be pushed alone");
    assert_eq!(
        single.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3]
    );

    let partial = selected_assembly_push_source_clips(&pending, Some(4))
        .expect("range should end at selected pending clip");
    assert_eq!(
        partial.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3, 4]
    );

    let all = selected_assembly_push_source_clips(&pending, None)
        .expect("blank selection pushes all pending clips");
    assert_eq!(
        all.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3, 4, 5]
    );
}

#[test]
fn selected_assembly_push_source_clips_rejects_non_pending_end_video() {
    let pending = vec![
        assembly_clip(3, 1280, 720, 24),
        assembly_clip(4, 1280, 720, 24),
    ];

    let err = selected_assembly_push_source_clips(&pending, Some(2))
        .expect_err("already-pushed or skipped end clip should be rejected");

    assert!(err.contains("不在可推送后续片段中"));
}

#[test]
fn ensure_insert_position_not_inside_pushed_prefix_blocks_start_and_middle() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
        assembly_clip(3, 1280, 720, 24),
    ];
    let rows = vec![assembly_push_video(11, 1, "accepted", vec![1, 2])];
    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    let start_err = ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, None)
        .expect_err("start insert should be blocked after any active push");
    assert!(start_err.contains("开头插入"));

    let middle_err = ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(1))
        .expect_err("middle insert should be blocked inside pushed prefix");
    assert!(middle_err.contains("中间插入"));

    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(2))
        .expect("insert after the pushed prefix tail should be allowed");
    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(3))
        .expect("insert after unpushed clips should be allowed");
}

#[test]
fn ensure_insert_position_not_inside_pushed_prefix_ignores_rejected_pushes() {
    let clips = vec![assembly_clip(1, 1280, 720, 24)];
    let rows = vec![assembly_push_video(11, 1, "rejected", vec![1])];
    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, None)
        .expect("rejected pushes should not lock the assembly prefix");
}
