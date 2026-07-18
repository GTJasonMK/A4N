#[test]
fn canvas_image_input_uses_connected_config_node() {
    let prompt_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "画布图片提示词"
        }),
    );
    let image_node = test_node(3, "GenerateImageNode", json!({"image_config_id": 7}));
    let document = test_graph_document(
        vec![prompt_node, image_node.clone()],
        vec![test_typed_edge(1, 3, "text", "text", "context")],
    );

    let input = build_canvas_image_input(20, 10, &image_node, &document).unwrap();

    assert_eq!(input.config_id, Some(7));
    assert_eq!(input.prompt, "画布图片提示词");
}

#[test]
fn canvas_image_input_uses_connected_config_node_without_inline_config() {
    let prompt_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "画布图片提示词"
        }),
    );
    let image_node = test_node(3, "GenerateImageNode", json!({"image_config_id": 7}));
    let document = test_graph_document(
        vec![prompt_node, image_node.clone()],
        vec![test_typed_edge(1, 3, "text", "text", "context")],
    );

    let input = build_canvas_image_input(20, 10, &image_node, &document).unwrap();

    assert_eq!(input.config_id, Some(7));
}

#[test]
fn canvas_image_input_ignores_non_image_config_key() {
    let prompt_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "画布图片提示词"
        }),
    );
    let image_node = test_node(3, "GenerateImageNode", json!({"video_config_id": 7}));
    let document = test_graph_document(
        vec![prompt_node, image_node.clone()],
        vec![test_typed_edge(1, 3, "text", "text", "context")],
    );

    let input = build_canvas_image_input(20, 10, &image_node, &document).unwrap();

    assert_eq!(input.config_id, None);
}

#[test]
fn canvas_operation_nodes_ignore_inline_config_id() {
    let image_prompt_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "画布图片提示词"
        }),
    );
    let image_node = test_node(
        2,
        "GenerateImageNode",
        json!({
            "config_id": 5
        }),
    );
    let image_document = test_graph_document(
        vec![image_prompt_node, image_node.clone()],
        vec![test_typed_edge(1, 2, "text", "text", "context")],
    );
    let image_input = build_canvas_image_input(20, 10, &image_node, &image_document).unwrap();
    assert_eq!(image_input.config_id, None);

    let video_prompt_node = test_node(
        3,
        "TextInputNode",
        json!({
            "content": "画布视频提示词"
        }),
    );
    let video_node = test_node(
        4,
        "GenerateVideoNode",
        json!({
            "config_id": 6
        }),
    );
    let video_document = test_graph_document(
        vec![video_prompt_node, video_node.clone()],
        vec![test_typed_edge(3, 4, "text", "text", "context")],
    );
    let video_input = build_canvas_video_input(30, 20, 10, &video_node, &video_document).unwrap();
    assert_eq!(video_input.config_id, None);

    let text_node = test_node(
        5,
        "TextInputNode",
        json!({
            "content": "需要改写的文本"
        }),
    );
    let llm_node = test_node(
        6,
        "LlmTextNode",
        json!({
            "config_id": 7
        }),
    );
    let llm_document = test_graph_document(
        vec![text_node, llm_node.clone()],
        vec![test_typed_edge(5, 6, "text", "input", "context")],
    );
    let llm_input = build_canvas_llm_transform_input(&llm_node, &llm_document).unwrap();
    assert_eq!(llm_input.config_id, None);
}
