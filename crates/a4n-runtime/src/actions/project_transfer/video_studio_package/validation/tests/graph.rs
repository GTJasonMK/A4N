use super::*;

#[test]
fn validate_video_studio_manifest_rejects_missing_canvas_node_media_ref() {
    let mut package = package();
    package.canvas_graphs.push(graph());
    package.canvas_nodes.push(node_with_ref("media_asset", 999));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("missing media asset node ref must fail");

    assert!(err.contains("画布节点引用了不存在的媒体资产"));
    assert!(err.contains("ref_id=999"));
}

#[test]
fn validate_video_studio_manifest_rejects_invalid_canvas_edge_role() {
    let mut package = package();
    package.canvas_graphs.push(graph());
    package.canvas_nodes.push(canvas_node(31, "VideoInputNode"));
    package
        .canvas_nodes
        .push(canvas_node(32, "AssembleVideoNode"));
    package.canvas_edges.push(canvas_edge(
        60,
        31,
        32,
        "video",
        "clip",
        "source_video",
        serde_json::json!({ "order_index": 1 }),
    ));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("import package must reject invalid canvas edge role");

    assert!(err.contains("画布连线合同无效"));
    assert!(err.contains("role 无效"));
    assert!(err.contains("source_video"));
}

#[test]
fn validate_video_studio_manifest_rejects_assembly_clip_without_order_index() {
    let mut package = package();
    package.canvas_graphs.push(graph());
    package.canvas_nodes.push(canvas_node(31, "VideoInputNode"));
    package
        .canvas_nodes
        .push(canvas_node(32, "AssembleVideoNode"));
    package.canvas_edges.push(canvas_edge(
        60,
        31,
        32,
        "video",
        "clip",
        "assembly_clip",
        serde_json::json!({}),
    ));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("assembly clip edge must keep explicit order_index");

    assert!(err.contains("画布连线合同无效"));
    assert!(err.contains("order_index"));
}

#[test]
fn validate_video_studio_manifest_rejects_cyclic_canvas_graph() {
    let mut package = package();
    package.canvas_graphs.push(graph());
    package.canvas_nodes.push(canvas_node(31, "LlmTextNode"));
    package.canvas_nodes.push(canvas_node(32, "LlmTextNode"));
    package.canvas_edges.push(canvas_edge(
        60,
        31,
        32,
        "text",
        "input",
        "context",
        serde_json::json!({}),
    ));
    package.canvas_edges.push(canvas_edge(
        61,
        32,
        31,
        "text",
        "input",
        "context",
        serde_json::json!({}),
    ));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("import package must reject cyclic canvas graph");

    assert!(err.contains("画布结构无效"));
    assert!(err.contains("有向无环图"));
}

#[test]
fn validate_video_studio_manifest_rejects_timeline_cache_key_missing_node() {
    let mut package = package();
    package.canvas_graphs.push(graph());
    package
        .assembly_timelines
        .push(assembly_timeline_with_cache_key("canvas_graph:20:node:999"));

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("timeline cache key must reference a package canvas node");

    assert!(err.contains("成片时间线缓存键引用了不存在的画布节点"));
    assert!(err.contains("node_id=999"));
}

#[test]
fn validate_video_studio_manifest_rejects_non_object_graph_viewport() {
    let mut package = package();
    let mut graph = graph();
    graph.viewport_json = Some(serde_json::json!([0, 0, 1]));
    package.canvas_graphs.push(graph);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("graph viewport must be a JSON object when present");

    assert!(err.contains("画布视口无效"));
    assert!(err.contains("viewport_json 必须是 JSON 对象"));
}
