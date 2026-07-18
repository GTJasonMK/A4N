#[test]
fn canvas_operation_nodes_reject_inline_upstream_resource_inputs() {
    let image_node = test_node(
        1,
        "GenerateImageNode",
        json!({
            "prompt": "隐藏图片提示词"
        }),
    );
    let image_document = test_graph_document(vec![image_node.clone()], Vec::new());
    let err = build_canvas_image_input(20, 10, &image_node, &image_document)
        .expect_err("GenerateImageNode must require a connected prompt source");
    assert!(err.contains("缺少 prompt 输入"));

    let video_node = test_node(
        2,
        "GenerateVideoNode",
        json!({
            "prompt": "隐藏视频提示词",
            "first_frame_image_id": 91,
            "reference_image_ids": [92]
        }),
    );
    let video_document = test_graph_document(vec![video_node.clone()], Vec::new());
    let err = build_canvas_video_input(30, 20, 10, &video_node, &video_document)
        .expect_err("GenerateVideoNode must require a connected prompt source");
    assert!(err.contains("缺少 prompt 输入"));

    let trim_node = test_node(
        3,
        "TrimVideoNode",
        json!({
            "video_id": 91,
            "end_secs": 1.0
        }),
    );
    let trim_document = test_graph_document(vec![trim_node.clone()], Vec::new());
    let err = build_canvas_trim_input(&trim_node, &trim_document)
        .expect_err("TrimVideoNode must require a connected video source");
    assert!(err.contains("缺少源视频"));

    let extract_node = test_node(
        4,
        "ExtractFrameNode",
        json!({
            "video_id": 91
        }),
    );
    let extract_document = test_graph_document(vec![extract_node.clone()], Vec::new());
    let err = build_canvas_extract_frame_input(&extract_node, &extract_document)
        .expect_err("ExtractFrameNode must require a connected video source");
    assert!(err.contains("缺少源视频"));

    let export_node = test_node(
        5,
        "VideoOutputNode",
        json!({
            "video_id": 91,
            "generated_video_id": 92,
            "assembly_timeline_id": 77,
            "timeline_id": 78
        }),
    );
    let export_document = test_graph_document(vec![export_node.clone()], Vec::new());
    let err = build_canvas_export_video_input(&export_node, &export_document)
        .expect_err("VideoOutputNode must require connected export input");
    assert!(err.contains("缺少可输出的视频输入"));

    let llm_node = test_node(
        6,
        "LlmTextNode",
        json!({
            "input": "隐藏改写文本"
        }),
    );
    let llm_document = test_graph_document(vec![llm_node.clone()], Vec::new());
    let err = build_canvas_llm_transform_input(&llm_node, &llm_document)
        .expect_err("LlmTextNode must require a connected text source");
    assert!(err.contains("缺少输入文本"));
}

#[test]
fn canvas_operation_inputs_require_current_run_checkpoint_for_upstream_operations() {
    let render_node = test_node(
        1,
        "RenderTextNode",
        json!({
            "prompt": "旧的隐藏渲染结果",
            "content": "不应作为当前输出",
            "template": "模板 {{context}}"
        }),
    );
    let image_node = test_node(2, "GenerateImageNode", json!({}));
    let document = test_graph_document(
        vec![render_node, image_node.clone()],
        vec![test_typed_edge(1, 2, "text", "text", "context")],
    );

    let err = block_on_input(build_canvas_image_input_for_run(
        None,
        Some(99),
        20,
        10,
        &image_node,
        &document,
    ))
    .expect_err("upstream operation without current-run checkpoint must fail");

    assert!(err.contains("当前 workflow checkpoint"));
}
