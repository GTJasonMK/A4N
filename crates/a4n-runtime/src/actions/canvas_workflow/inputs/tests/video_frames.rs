#[test]
fn canvas_video_input_ignores_non_video_config_key() {
    let prompt_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "画布视频提示词"
        }),
    );
    let video_node = test_node(3, "GenerateVideoNode", json!({"image_config_id": 7}));
    let document = test_graph_document(
        vec![prompt_node, video_node.clone()],
        vec![test_typed_edge(1, 3, "text", "text", "context")],
    );

    let input = build_canvas_video_input(30, 20, 10, &video_node, &document).unwrap();

    assert_eq!(input.config_id, None);
}

#[test]
fn canvas_video_input_uses_image_node_ref_id() {
    let mut image_node = test_node(1, "ImageInputNode", json!({}));
    image_node.ref_type = Some("generated_image".to_string());
    image_node.ref_id = Some(91);
    let prompt_node = test_node(
        2,
        "TextInputNode",
        json!({
            "prompt": "画布视频提示词"
        }),
    );
    let video_node = test_node(3, "GenerateVideoNode", json!({"input_mode": "first_frame"}));
    let document = test_graph_document(
        vec![image_node, prompt_node, video_node.clone()],
        vec![
            test_typed_edge(1, 3, "image", "first_frame", "first_frame"),
            test_typed_edge(2, 3, "text", "text", "context"),
        ],
    );

    let input = build_canvas_video_input(30, 20, 10, &video_node, &document).unwrap();

    assert_eq!(input.first_frame_image_id, Some(91));
}

#[test]
fn canvas_video_input_rejects_role_only_image_edges() {
    let mut image_node = test_node(1, "ImageInputNode", json!({}));
    image_node.ref_type = Some("generated_image".to_string());
    image_node.ref_id = Some(91);
    let prompt_node = test_node(2, "TextInputNode", json!({"prompt": "画布视频提示词"}));
    let video_node = test_node(3, "GenerateVideoNode", json!({"input_mode": "first_frame"}));
    let document = test_graph_document(
        vec![image_node, prompt_node, video_node.clone()],
        vec![
            test_edge(1, 3, "first_frame"),
            test_typed_edge(2, 3, "text", "text", "context"),
        ],
    );

    let err = build_canvas_video_input(30, 20, 10, &video_node, &document)
        .expect_err("role-only image edge must not be accepted as first_frame input");

    assert!(err.contains("图片输入端口无效"));
}

#[test]
fn canvas_video_input_first_end_mode_requires_both_frame_inputs() {
    let mut first_image = test_node(1, "ImageInputNode", json!({}));
    first_image.ref_type = Some("generated_image".to_string());
    first_image.ref_id = Some(91);
    let prompt_node = test_node(2, "TextInputNode", json!({"prompt": "画布视频提示词"}));
    let video_node = test_node(
        3,
        "GenerateVideoNode",
        json!({"input_mode": "first_end_frame"}),
    );
    let missing_end_document = test_graph_document(
        vec![first_image.clone(), prompt_node.clone(), video_node.clone()],
        vec![
            test_typed_edge(1, 3, "image", "first_frame", "first_frame"),
            test_typed_edge(2, 3, "text", "text", "context"),
        ],
    );

    let err = build_canvas_video_input(30, 20, 10, &video_node, &missing_end_document)
        .expect_err("first_end_frame mode must require tail frame input");
    assert!(err.contains("必须同时连接 first_frame 和 end_frame"));

    let mut end_image = test_node(4, "ImageInputNode", json!({}));
    end_image.ref_type = Some("generated_image".to_string());
    end_image.ref_id = Some(92);
    let document = test_graph_document(
        vec![first_image, prompt_node, video_node.clone(), end_image],
        vec![
            test_typed_edge(1, 3, "image", "first_frame", "first_frame"),
            test_typed_edge(4, 3, "image", "end_frame", "end_frame"),
            test_typed_edge(2, 3, "text", "text", "context"),
        ],
    );

    let input = build_canvas_video_input(30, 20, 10, &video_node, &document).unwrap();
    assert_eq!(input.first_frame_image_id, Some(91));
    assert_eq!(input.last_frame_image_id, Some(92));
    assert!(input.reference_image_ids.is_empty());
}

#[test]
fn canvas_video_input_validates_image_inputs_against_explicit_mode() {
    let mut first_image = test_node(1, "ImageInputNode", json!({}));
    first_image.ref_type = Some("generated_image".to_string());
    first_image.ref_id = Some(91);
    let mut reference_image = test_node(2, "ImageInputNode", json!({}));
    reference_image.ref_type = Some("generated_image".to_string());
    reference_image.ref_id = Some(92);
    let prompt_node = test_node(3, "TextInputNode", json!({"prompt": "画布视频提示词"}));
    let text_only_node = test_node(4, "GenerateVideoNode", json!({"input_mode": "text_only"}));
    let text_only_document = test_graph_document(
        vec![
            first_image.clone(),
            prompt_node.clone(),
            text_only_node.clone(),
        ],
        vec![
            test_typed_edge(1, 4, "image", "first_frame", "first_frame"),
            test_typed_edge(3, 4, "text", "text", "context"),
        ],
    );
    let err = build_canvas_video_input(30, 20, 10, &text_only_node, &text_only_document)
        .expect_err("text_only mode must reject connected images");
    assert!(err.contains("仅提示词模式"));

    let implicit_text_only_node = test_node(6, "GenerateVideoNode", json!({}));
    let implicit_text_only_document = test_graph_document(
        vec![
            first_image.clone(),
            prompt_node.clone(),
            implicit_text_only_node.clone(),
        ],
        vec![
            test_typed_edge(1, 6, "image", "first_frame", "first_frame"),
            test_typed_edge(3, 6, "text", "text", "context"),
        ],
    );
    let err = build_canvas_video_input(
        30,
        20,
        10,
        &implicit_text_only_node,
        &implicit_text_only_document,
    )
    .expect_err("missing input_mode must not infer first_frame from connected images");
    assert!(err.contains("仅提示词模式"));

    let reference_node = test_node(
        5,
        "GenerateVideoNode",
        json!({"input_mode": "reference_images"}),
    );
    let mixed_document = test_graph_document(
        vec![
            first_image,
            reference_image.clone(),
            prompt_node.clone(),
            reference_node.clone(),
        ],
        vec![
            test_typed_edge(1, 5, "image", "first_frame", "first_frame"),
            test_typed_edge(2, 5, "image", "reference_image", "style_reference"),
            test_typed_edge(3, 5, "text", "text", "context"),
        ],
    );
    let err = build_canvas_video_input(30, 20, 10, &reference_node, &mixed_document)
        .expect_err("reference_images mode must reject first frame input");
    assert!(err.contains("参考图模式只能连接 reference_image"));

    let reference_document = test_graph_document(
        vec![reference_image, prompt_node, reference_node.clone()],
        vec![
            test_typed_edge(2, 5, "image", "reference_image", "style_reference"),
            test_typed_edge(3, 5, "text", "text", "context"),
        ],
    );
    let input = build_canvas_video_input(30, 20, 10, &reference_node, &reference_document).unwrap();
    assert_eq!(input.reference_image_ids, vec![92]);
    assert_eq!(input.first_frame_image_id, None);
    assert_eq!(input.last_frame_image_id, None);
}
