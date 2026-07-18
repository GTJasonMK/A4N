#[test]
fn canvas_assemble_input_collects_connected_video_nodes() {
    let mut video_node = test_node(1, "VideoInputNode", json!({}));
    video_node.ref_type = Some("legacy_generated_video".to_string());
    video_node.ref_id = Some(91);
    let assemble_node = test_node(
        2,
        "AssembleVideoNode",
        json!({
            "title": "组装预览"
        }),
    );
    let document = test_graph_document(
        vec![video_node, assemble_node.clone()],
        vec![test_edge_with_order(1, 2, "assembly_clip", 1)],
    );

    let (video_ids, title, source_node_ids) =
        build_canvas_assemble_input(&assemble_node, &document).unwrap();

    assert_eq!(video_ids, vec![91]);
    assert_eq!(title.as_deref(), Some("组装预览"));
    assert_eq!(source_node_ids, vec![1]);
}

#[test]
fn canvas_assemble_input_sorts_connected_video_nodes_by_explicit_order() {
    let mut first_video_node = test_node(1, "VideoInputNode", json!({}));
    first_video_node.ref_type = Some("legacy_generated_video".to_string());
    first_video_node.ref_id = Some(91);
    let mut second_video_node = test_node(3, "VideoInputNode", json!({}));
    second_video_node.ref_type = Some("legacy_generated_video".to_string());
    second_video_node.ref_id = Some(92);
    let assemble_node = test_node(2, "AssembleVideoNode", json!({}));
    let document = test_graph_document(
        vec![first_video_node, assemble_node.clone(), second_video_node],
        vec![
            test_edge_with_order(1, 2, "assembly_clip", 2),
            test_edge_with_order(3, 2, "assembly_clip", 1),
        ],
    );

    let (video_ids, _, source_node_ids) =
        build_canvas_assemble_input(&assemble_node, &document).unwrap();

    assert_eq!(video_ids, vec![92, 91]);
    assert_eq!(source_node_ids, vec![3, 1]);
}

#[test]
fn canvas_assemble_input_rejects_missing_clip_order() {
    let mut video_node = test_node(1, "VideoInputNode", json!({}));
    video_node.ref_type = Some("legacy_generated_video".to_string());
    video_node.ref_id = Some(91);
    let assemble_node = test_node(2, "AssembleVideoNode", json!({}));
    let document = test_graph_document(
        vec![video_node, assemble_node.clone()],
        vec![test_edge(1, 2, "assembly_clip")],
    );

    let err = build_canvas_assemble_input(&assemble_node, &document)
        .expect_err("AssembleVideoNode clip edges must have an explicit order");

    assert!(err.contains("必须设置 order_index"));
}

#[test]
fn canvas_assemble_input_rejects_config_video_ids() {
    let assemble_node = test_node(
        2,
        "AssembleVideoNode",
        json!({
            "video_ids": [91]
        }),
    );
    let document = test_graph_document(vec![assemble_node.clone()], Vec::new());

    let err = build_canvas_assemble_input(&assemble_node, &document)
        .expect_err("AssembleVideoNode must not accept hidden video_ids config");

    assert!(err.contains("不支持 config_json.video_ids"));
}

#[test]
fn canvas_video_source_inputs_reject_hidden_legacy_config_keys() {
    let trim_node = test_node(
        1,
        "TrimVideoNode",
        json!({
            "source_video_id": 91,
            "end_secs": 1.0
        }),
    );
    let document = test_graph_document(vec![trim_node.clone()], Vec::new());

    let err = build_canvas_trim_input(&trim_node, &document)
        .expect_err("source_video_id must not be accepted as a hidden canvas input");
    assert!(err.contains("缺少源视频"));

    let source_node = test_node(
        1,
        "GenerateVideoNode",
        json!({
            "legacy_generated_video_id": 91
        }),
    );
    let extract_node = test_node(2, "ExtractFrameNode", json!({}));
    let document = test_graph_document(
        vec![source_node, extract_node.clone()],
        vec![test_typed_edge(1, 2, "video", "video", "source_video")],
    );

    let err = build_canvas_extract_frame_input(&extract_node, &document)
        .expect_err("legacy_generated_video_id must not be accepted as a hidden canvas output");
    assert!(err.contains("不能直接读取未运行的上游操作节点"));

    let image_source = test_node(3, "ImageInputNode", json!({"generated_image_id": 91}));
    let prompt_node = test_node(4, "TextInputNode", json!({"prompt": "画布视频提示词"}));
    let video_node = test_node(5, "GenerateVideoNode", json!({"input_mode": "first_frame"}));
    let document = test_graph_document(
        vec![image_source, prompt_node, video_node.clone()],
        vec![
            test_typed_edge(3, 5, "image", "first_frame", "first_frame"),
            test_typed_edge(4, 5, "text", "text", "context"),
        ],
    );
    let err = build_canvas_video_input(30, 20, 10, &video_node, &document)
        .expect_err("generated_image_id config must not be accepted as a hidden image output");
    assert!(err.contains("首帧模式缺少 first_frame 输入"));

    let video_source = test_node(6, "VideoInputNode", json!({"generated_video_id": 92}));
    let output_node = test_node(7, "VideoOutputNode", json!({}));
    let document = test_graph_document(
        vec![video_source, output_node.clone()],
        vec![test_typed_edge(6, 7, "video", "video", "final_video")],
    );
    let err = build_canvas_export_video_input(&output_node, &document)
        .expect_err("generated_video_id config must not be accepted as a hidden video output");
    assert!(err.contains("缺少可输出的视频输入"));

    let audio_source = test_node(8, "AudioInputNode", json!({"media_asset_id": 93}));
    let audio_output = test_node(9, "AudioOutputNode", json!({}));
    let document = test_graph_document(
        vec![audio_source, audio_output.clone()],
        vec![test_typed_edge(8, 9, "audio", "audio", "context")],
    );
    let err =
        build_canvas_output_value_for_test(&audio_output, &document, "audio", "AudioOutputNode")
            .expect_err("media_asset_id config must not be accepted as a hidden audio output");
    assert!(err.contains("缺少音频输入"));
}
