use super::*;
use serde_json::{Value, json};

fn node(ref_type: Option<&str>, ref_id: Option<i64>) -> CanvasNodeRecordInput {
    node_with_kind("ImageInputNode", "node-1", ref_type, ref_id, json!({}))
}

fn node_with_kind(
    node_kind: &str,
    client_id: &str,
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: Value,
) -> CanvasNodeRecordInput {
    CanvasNodeRecordInput {
        client_id: client_id.to_string(),
        id: None,
        node_kind: node_kind.to_string(),
        ref_type: ref_type.map(str::to_string),
        ref_id,
        config_json: Some(config_json),
        position_json: json!({"x": 0, "y": 0}),
        state: None,
    }
}

fn graph_input(
    nodes: Vec<CanvasNodeRecordInput>,
    edges: Vec<CanvasEdgeRecordInput>,
) -> CanvasGraphRecordInput {
    CanvasGraphRecordInput {
        id: None,
        name: Some("测试画布".to_string()),
        graph_kind: Some(GRAPH_KIND_VIDEO_STUDIO.to_string()),
        viewport_json: None,
        nodes,
        edges,
    }
}

fn typed_edge(
    source: &str,
    target: &str,
    source_port: &str,
    target_port: &str,
    role: &str,
) -> CanvasEdgeRecordInput {
    CanvasEdgeRecordInput {
        source_client_id: source.to_string(),
        target_client_id: target.to_string(),
        source_node_id: None,
        target_node_id: None,
        source_port: source_port.to_string(),
        target_port: target_port.to_string(),
        role: role.to_string(),
        config_json: None,
    }
}

fn assemble_clip_edge(source: &str, target: &str, order_index: i64) -> CanvasEdgeRecordInput {
    CanvasEdgeRecordInput {
        config_json: Some(json!({
            "order_index": order_index
        })),
        ..typed_edge(source, target, "video", "clip", "assembly_clip")
    }
}

#[test]
fn canvas_ref_shape_rejects_ref_id_without_ref_type() {
    let err = validate_canvas_record_node_ref_shape(&node(None, Some(1)))
        .expect_err("ref_id without ref_type must fail");

    assert!(err.to_string().contains("ref_type 为空"));
}

#[test]
fn canvas_ref_shape_rejects_unknown_ref_type() {
    let err = validate_canvas_record_node_ref_shape(&node(Some("old_media"), Some(1)))
        .expect_err("unknown ref_type must fail");

    assert!(err.to_string().contains("未知画布节点引用类型"));
}

#[test]
fn canvas_ref_shape_requires_id_for_project_owned_refs() {
    let err = validate_canvas_record_node_ref_shape(&node(Some("legacy_generated_image"), None))
        .expect_err("project-owned ref must have an id");

    assert!(err.to_string().contains("必须设置 ref_id"));
}

#[test]
fn canvas_ref_shape_accepts_inline_without_id_and_config_with_positive_id() {
    let inline = validate_canvas_record_node_ref_shape(&node(Some("inline"), None))
        .expect("inline without ref_id should be valid");
    let config = validate_canvas_record_node_ref_shape(&node(Some("config"), Some(12)))
        .expect("config can carry a positive config id");

    assert_eq!(inline, Some(CanvasRecordNodeRefKind::Inline));
    assert_eq!(config, Some(CanvasRecordNodeRefKind::Config));
}

#[test]
fn canvas_config_node_edges_are_not_part_of_four_type_contract() {
    let config = node_with_kind(
        "ConfigNode",
        "cfg",
        None,
        None,
        json!({
            "config_type": "image",
            "config_id": 7
        }),
    );
    let image = node_with_kind("GenerateImageNode", "image", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![config, image],
        vec![typed_edge(
            "cfg",
            "image",
            "image_config",
            "config",
            "config",
        )],
    ))
    .expect_err("ConfigNode edges are no longer accepted");

    assert!(err.to_string().contains("端口不匹配"));
}

#[test]
fn canvas_edge_contract_rejects_legacy_output_input_fallback() {
    let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
    let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![text, render],
        vec![typed_edge("text", "render", "output", "input", "context")],
    ))
    .expect_err("generic output/input edge must not be accepted");

    assert!(err.to_string().contains("端口不匹配"));
}

#[test]
fn canvas_edge_contract_accepts_documented_text_ports() {
    let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
    let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));
    validate_canvas_graph_record_input(&graph_input(
        vec![text, render],
        vec![typed_edge("text", "render", "text", "input", "context")],
    ))
    .expect("documented text -> render text edge should be accepted");
}

#[test]
fn canvas_edge_contract_accepts_text_producers_for_generation_nodes() {
    for source_kind in ["TextInputNode", "RenderTextNode", "LlmTextNode"] {
        for target_kind in ["GenerateImageNode", "GenerateVideoNode"] {
            let source = node_with_kind(source_kind, "source", Some("inline"), None, json!({}));
            let target = node_with_kind(target_kind, "target", None, None, json!({}));

            validate_canvas_graph_record_input(&graph_input(
                vec![source, target],
                vec![typed_edge("source", "target", "text", "text", "context")],
            ))
            .unwrap_or_else(|err| {
                panic!("{source_kind}.text -> {target_kind}.text should be accepted: {err}")
            });
        }
    }
}

#[test]
fn canvas_edge_contract_accepts_probe_passthrough_nodes() {
    let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
    let text_probe = node_with_kind("TextProbeNode", "text_probe", None, None, json!({}));
    let image = node_with_kind(
        "ImageInputNode",
        "image",
        Some("media_asset"),
        Some(1),
        json!({}),
    );
    let image_probe = node_with_kind("ImageProbeNode", "image_probe", None, None, json!({}));
    let video = node_with_kind(
        "VideoInputNode",
        "video",
        Some("media_asset"),
        Some(2),
        json!({}),
    );
    let video_probe = node_with_kind("VideoProbeNode", "video_probe", None, None, json!({}));
    let audio = node_with_kind(
        "AudioInputNode",
        "audio",
        Some("media_asset"),
        Some(3),
        json!({}),
    );
    let audio_probe = node_with_kind("AudioProbeNode", "audio_probe", None, None, json!({}));
    let image_gen = node_with_kind("GenerateImageNode", "image_gen", None, None, json!({}));
    let video_gen = node_with_kind("GenerateVideoNode", "video_gen", None, None, json!({}));
    let trim = node_with_kind("TrimVideoNode", "trim", None, None, json!({}));
    let audio_output = node_with_kind("AudioOutputNode", "audio_output", None, None, json!({}));

    validate_canvas_graph_record_input(&graph_input(
        vec![
            text,
            text_probe,
            image,
            image_probe,
            video,
            video_probe,
            audio,
            audio_probe,
            image_gen,
            video_gen,
            trim,
            audio_output,
        ],
        vec![
            typed_edge("text", "text_probe", "text", "text", "context"),
            typed_edge("text_probe", "image_gen", "text", "text", "context"),
            typed_edge("image", "image_probe", "image", "image", "context"),
            typed_edge(
                "image_probe",
                "video_gen",
                "image",
                "first_frame",
                "first_frame",
            ),
            typed_edge("video", "video_probe", "video", "video", "context"),
            typed_edge("video_probe", "trim", "video", "video", "source_video"),
            typed_edge("audio", "audio_probe", "audio", "audio", "context"),
            typed_edge("audio_probe", "audio_output", "audio", "audio", "context"),
        ],
    ))
    .expect("probe nodes should pass through their own data type");
}

#[test]
fn canvas_edge_contract_rejects_cross_type_probe_connections() {
    let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
    let image_probe = node_with_kind("ImageProbeNode", "image_probe", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![text, image_probe],
        vec![typed_edge(
            "text",
            "image_probe",
            "text",
            "image",
            "context",
        )],
    ))
    .expect_err("text output must not feed image probe");

    assert!(err.to_string().contains("端口不匹配"));
}

#[test]
fn canvas_edge_contract_accepts_text_for_llm_transform() {
    let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
    let llm = node_with_kind("LlmTextNode", "llm", None, None, json!({}));
    validate_canvas_graph_record_input(&graph_input(
        vec![text, llm],
        vec![typed_edge("text", "llm", "text", "input", "context")],
    ))
    .expect("TextInputNode should feed LlmTextNode input");
}

#[test]
fn canvas_edge_contract_accepts_text_producers_for_render_text_input() {
    for source_kind in ["TextInputNode", "RenderTextNode", "LlmTextNode"] {
        let source = node_with_kind(source_kind, "source", Some("inline"), None, json!({}));
        let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));

        validate_canvas_graph_record_input(&graph_input(
            vec![source, render],
            vec![typed_edge("source", "render", "text", "input", "context")],
        ))
        .unwrap_or_else(|err| {
            panic!("{source_kind}.text -> RenderTextNode.input should be accepted: {err}")
        });
    }
}

#[test]
fn canvas_edge_contract_accepts_llm_transform_chaining() {
    let first = node_with_kind("LlmTextNode", "first", None, None, json!({}));
    let second = node_with_kind("LlmTextNode", "second", None, None, json!({}));

    validate_canvas_graph_record_input(&graph_input(
        vec![first, second],
        vec![typed_edge("first", "second", "text", "input", "context")],
    ))
    .expect("LlmTextNode output should feed another LlmTextNode");
}

#[test]
fn canvas_edge_contract_uses_assembly_clip_role_for_assemble_node() {
    let video = node_with_kind(
        "VideoInputNode",
        "video",
        Some("legacy_generated_video"),
        Some(9),
        json!({}),
    );
    let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

    validate_canvas_graph_record_input(&graph_input(
        vec![video.clone(), assemble.clone()],
        vec![assemble_clip_edge("video", "assemble", 1)],
    ))
    .expect("VideoInputNode clip edges should use assembly_clip role");

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![video, assemble],
        vec![typed_edge(
            "video",
            "assemble",
            "video",
            "clip",
            "source_video",
        )],
    ))
    .expect_err("AssembleVideoNode clip edges must not use source_video role");

    assert!(err.to_string().contains("role 无效"));
}

#[test]
fn canvas_edge_contract_accepts_generator_outputs_as_downstream_sources() {
    let prompt = node_with_kind("TextInputNode", "prompt", Some("inline"), None, json!({}));
    let generate = node_with_kind("GenerateVideoNode", "generate", None, None, json!({}));
    let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));
    let export = node_with_kind("VideoOutputNode", "export", None, None, json!({}));

    validate_canvas_graph_record_input(&graph_input(
        vec![prompt, generate, assemble, export],
        vec![
            typed_edge("prompt", "generate", "text", "text", "context"),
            assemble_clip_edge("generate", "assemble", 1),
            typed_edge("assemble", "export", "video", "video", "final_video"),
        ],
    ))
    .expect("generated video should flow into assemble and export sink");
}

#[test]
fn canvas_edge_contract_rejects_sink_output_connections() {
    let export = node_with_kind("VideoOutputNode", "export", None, None, json!({}));
    let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![export, video],
        vec![typed_edge("export", "video", "video", "text", "context")],
    ))
    .expect_err("sink nodes must not produce downstream edges");

    assert!(err.to_string().contains("端口不匹配"));
}

#[test]
fn canvas_edge_contract_requires_explicit_assembly_clip_order() {
    let video = node_with_kind(
        "VideoInputNode",
        "video",
        Some("legacy_generated_video"),
        Some(9),
        json!({}),
    );
    let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![video, assemble],
        vec![typed_edge(
            "video",
            "assemble",
            "video",
            "clip",
            "assembly_clip",
        )],
    ))
    .expect_err("AssembleVideoNode clip edges must carry order_index");

    assert!(err.to_string().contains("必须设置 order_index"));
}

#[test]
fn canvas_edge_contract_rejects_duplicate_assembly_clip_order() {
    let first_video = node_with_kind(
        "VideoInputNode",
        "video-1",
        Some("legacy_generated_video"),
        Some(9),
        json!({}),
    );
    let second_video = node_with_kind(
        "VideoInputNode",
        "video-2",
        Some("legacy_generated_video"),
        Some(10),
        json!({}),
    );
    let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![first_video, second_video, assemble],
        vec![
            assemble_clip_edge("video-1", "assemble", 1),
            assemble_clip_edge("video-2", "assemble", 1),
        ],
    ))
    .expect_err("AssembleVideoNode clip order_index must be unique per target");

    assert!(err.to_string().contains("片段顺序重复"));
}

#[test]
fn canvas_edge_contract_rejects_invalid_role_for_port_pair() {
    let image = node_with_kind(
        "ImageInputNode",
        "image",
        Some("legacy_generated_image"),
        Some(9),
        json!({}),
    );
    let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

    let err = validate_canvas_graph_record_input(&graph_input(
        vec![image, video],
        vec![typed_edge(
            "image",
            "video",
            "image",
            "first_frame",
            "context",
        )],
    ))
    .expect_err("first_frame edge must require first_frame role");

    assert!(err.to_string().contains("role 无效"));
}

#[test]
fn canvas_edge_contract_accepts_generate_video_end_frame_input() {
    let image = node_with_kind(
        "ImageInputNode",
        "image",
        Some("legacy_generated_image"),
        Some(9),
        json!({}),
    );
    let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

    validate_canvas_graph_record_input(&graph_input(
        vec![image, video],
        vec![typed_edge(
            "image",
            "video",
            "image",
            "end_frame",
            "end_frame",
        )],
    ))
    .expect("GenerateVideoNode end_frame should accept image input with end_frame role");
}

#[test]
fn canvas_viewport_rejects_non_object() {
    let error = validate_canvas_record_viewport_json(Some(&json!([])), "viewport")
        .expect_err("array viewport must be rejected");
    assert!(error.to_string().contains("必须是 JSON 对象"));
}

#[test]
fn canvas_graph_input_rejects_cycle() {
    let nodes = vec![
        node_with_kind("LlmTextNode", "a", Some("inline"), None, json!({})),
        node_with_kind("LlmTextNode", "b", Some("inline"), None, json!({})),
    ];
    let edges = vec![
        typed_edge("a", "b", "text", "input", "context"),
        typed_edge("b", "a", "text", "input", "context"),
    ];
    let error = validate_canvas_graph_record_input(&graph_input(nodes, edges))
        .expect_err("cyclic graph input must be rejected");
    assert!(error.to_string().contains("存在环路"));
}
