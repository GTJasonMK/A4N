pub(in crate::actions::canvas_workflow::execution) fn canvas_node_kind_is_executable(
    node_kind: &str,
) -> bool {
    matches!(
        node_kind,
        "RenderTextNode"
            | "LlmTextNode"
            | "GenerateImageNode"
            | "GenerateVideoNode"
            | "ExtractFrameNode"
            | "TrimVideoNode"
            | "AssembleVideoNode"
            | "TextProbeNode"
            | "ImageProbeNode"
            | "VideoProbeNode"
            | "AudioProbeNode"
            | "TextOutputNode"
            | "ImageOutputNode"
            | "VideoOutputNode"
            | "AudioOutputNode"
    )
}

pub(in crate::actions::canvas_workflow::execution) fn canvas_node_kind_is_output(
    node_kind: &str,
) -> bool {
    matches!(
        node_kind,
        "TextOutputNode" | "ImageOutputNode" | "VideoOutputNode" | "AudioOutputNode"
    )
}

pub(in crate::actions::canvas_workflow) fn unsupported_canvas_node_execution_error(
    node_kind: &str,
) -> String {
    format!("画布节点类型不支持执行: {node_kind}")
}
