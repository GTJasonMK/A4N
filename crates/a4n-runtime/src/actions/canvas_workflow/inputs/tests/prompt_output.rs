#[test]
fn canvas_probe_observation_cache_is_not_a_runtime_input() {
    let probe_node = test_node(
        1,
        "ImageProbeNode",
        json!({
            "media_asset_id": 91,
            "observed_data_type": "image"
        }),
    );
    let output_node = test_node(2, "ImageOutputNode", json!({}));
    let document = test_graph_document(
        vec![probe_node, output_node.clone()],
        vec![test_typed_edge(1, 2, "image", "image", "context")],
    );

    let err =
        build_canvas_output_value_for_test(&output_node, &document, "image", "ImageOutputNode")
            .expect_err("unrun probe observation cache must not be accepted as runtime input");

    assert!(err.contains("不能直接读取未运行的上游操作节点"));
}

#[test]
fn canvas_render_prompt_renders_template_context() {
    let text_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "雨夜追逐"
        }),
    );
    let render_node = test_node(
        2,
        "RenderTextNode",
        json!({
            "template": "模板 {{context}}",
            "output_kind": "video_prompt"
        }),
    );
    let document = test_graph_document(
        vec![text_node, render_node.clone()],
        vec![test_typed_edge(1, 2, "text", "input", "context")],
    );

    let rendered = build_canvas_prompt_artifact_input(&render_node, &document).unwrap();

    assert_eq!(rendered.prompt_kind, "video_prompt");
    assert_eq!(rendered.prompt_text, "模板 雨夜追逐");
}

#[test]
fn canvas_export_video_input_reads_timeline_and_video_sources() {
    let assemble_node = test_node(1, "AssembleVideoNode", json!({}));
    let mut video_node = test_node(2, "VideoInputNode", json!({}));
    video_node.ref_type = Some("generated_video".to_string());
    video_node.ref_id = Some(91);
    let export_node = test_node(3, "VideoOutputNode", json!({}));
    let document = test_graph_document(
        vec![assemble_node, video_node, export_node.clone()],
        vec![test_typed_edge(2, 3, "video", "video", "final_video")],
    );

    let input = build_canvas_export_video_input(&export_node, &document).unwrap();

    assert_eq!(input.video_id, Some(91));
    assert_eq!(input.source_node_ids, vec![2]);
}

#[test]
fn canvas_llm_transform_input_collects_text_and_connected_config() {
    let text_node = test_node(
        1,
        "TextInputNode",
        json!({
            "content": "雨夜里，角色推开仓库门。"
        }),
    );
    let transform_node = test_node(
        3,
        "LlmTextNode",
        json!({
            "instruction": "改写为三条镜头脚本",
            "output_kind": "shot_script",
            "llm_config_id": 12,
            "temperature": 0.3,
            "max_tokens": 800
        }),
    );
    let document = test_graph_document(
        vec![text_node, transform_node.clone()],
        vec![test_typed_edge(1, 3, "text", "input", "context")],
    );

    let input = build_canvas_llm_transform_input(&transform_node, &document)
        .expect("llm transform input should be built");

    assert_eq!(input.config_id, Some(12));
    assert_eq!(input.output_kind, "shot_script");
    assert_eq!(input.temperature, Some(0.3));
    assert_eq!(input.max_tokens, Some(800));
    assert!(input.user_prompt.contains("改写为三条镜头脚本"));
    assert!(input.user_prompt.contains("雨夜里，角色推开仓库门。"));
    assert_eq!(input.source_node_ids, vec![1]);
}
