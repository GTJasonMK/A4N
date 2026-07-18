#[test]
fn validate_postprocess_operation_accepts_only_supported_operations() {
    assert_eq!(validate_postprocess_operation(" insert "), Ok("insert"));
    assert_eq!(validate_postprocess_operation("delete"), Ok("delete"));
    assert_eq!(validate_postprocess_operation("replace"), Ok("replace"));
    assert!(validate_postprocess_operation(" ").is_err());
    assert!(validate_postprocess_operation("split").is_err());
}

#[test]
fn parse_postprocess_edit_metadata_requires_valid_version_chain_fields() {
    let mut valid = video_with_metadata(
        21,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "replace",
                "source_video_id": 13
            }
        })),
    );
    valid.video_type = "assembly_postprocess_edit".to_string();

    let parsed = parse_postprocess_edit_metadata(&valid).expect("valid metadata should parse");
    assert_eq!(parsed.push_video_id, 11);
    assert_eq!(parsed.base_video_id, 12);

    let mut missing_push = video_with_metadata(
        22,
        Some(json!({
            "postprocess_edit": {
                "base_video_id": 12,
                "operation": "delete"
            }
        })),
    );
    missing_push.video_type = "assembly_postprocess_edit".to_string();
    let err = parse_postprocess_edit_metadata(&missing_push)
        .expect_err("missing push_video_id should fail");
    assert!(err.contains("push_video_id"));
    assert!(err.contains("必须是正整数"));

    let mut bad_operation = video_with_metadata(
        23,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "merge"
            }
        })),
    );
    bad_operation.video_type = "assembly_postprocess_edit".to_string();
    let err =
        parse_postprocess_edit_metadata(&bad_operation).expect_err("unknown operation should fail");
    assert!(err.contains("后处理操作无效"));

    let mut bad_source = video_with_metadata(
        24,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "insert",
                "source_video_id": 0
            }
        })),
    );
    bad_source.video_type = "assembly_postprocess_edit".to_string();
    let err = parse_postprocess_edit_metadata(&bad_source)
        .expect_err("invalid source_video_id should fail");
    assert!(err.contains("source_video_id"));
    assert!(err.contains("必须为空或正整数"));
}

#[test]
fn postprocess_edit_range_validates_insert_and_delete_boundaries() {
    let mut base = video_with_metadata(1, Some(json!({ "shot_number": 1 })));
    base.duration_secs = 10.0;

    let insert = ApplyAssemblyPostprocessEditInput {
        operation: "insert".to_string(),
        at_secs: Some(9.5),
        start_secs: None,
        end_secs: None,
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert_eq!(
        postprocess_edit_range(&base, &insert, "insert", None)
            .expect("insert at finite time should be valid"),
        (9.5, 9.5, None)
    );

    let delete = ApplyAssemblyPostprocessEditInput {
        operation: "delete".to_string(),
        at_secs: None,
        start_secs: Some(2.0),
        end_secs: Some(4.0),
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert_eq!(
        postprocess_edit_range(&base, &delete, "delete", None)
            .expect("forward delete range should be valid"),
        (2.0, 4.0, None)
    );

    let bad_insert = ApplyAssemblyPostprocessEditInput {
        operation: "insert".to_string(),
        at_secs: Some(10.5),
        start_secs: None,
        end_secs: None,
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert!(
        postprocess_edit_range(&base, &bad_insert, "insert", None)
            .expect_err("insert beyond duration should fail")
            .contains("超出当前片段范围")
    );

    let bad_delete = ApplyAssemblyPostprocessEditInput {
        operation: "delete".to_string(),
        at_secs: None,
        start_secs: Some(4.0),
        end_secs: Some(2.0),
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert!(
        postprocess_edit_range(&base, &bad_delete, "delete", None)
            .expect_err("backward delete range should fail")
            .contains("时间范围无效")
    );
}
