use crate::canvas::CanvasDbError;

const CONTEXT_EDGE_ROLES: &[&str] = &[
    "context",
    "direct_content",
    "previous_context",
    "next_context",
];
const FIRST_FRAME_EDGE_ROLES: &[&str] = &["first_frame"];
const END_FRAME_EDGE_ROLES: &[&str] = &["end_frame"];
const REFERENCE_IMAGE_EDGE_ROLES: &[&str] = &[
    "character_reference",
    "object_reference",
    "scene_reference",
    "style_reference",
];
const SOURCE_VIDEO_EDGE_ROLES: &[&str] = &["source_video"];
const ASSEMBLY_CLIP_EDGE_ROLES: &[&str] = &["assembly_clip"];
const FINAL_VIDEO_EDGE_ROLES: &[&str] = &["final_video"];

pub(in crate::canvas::graph_record::validation) fn validate_canvas_record_edge_contract(
    source_kind: &str,
    source_port: &str,
    target_kind: &str,
    target_port: &str,
    role: &str,
) -> Result<(), CanvasDbError> {
    let source_kind = source_kind.trim();
    let source_port = source_port.trim();
    let target_kind = target_kind.trim();
    let target_port = target_port.trim();
    let role = role.trim();
    let Some(allowed_roles) =
        canvas_record_edge_allowed_roles(source_kind, source_port, target_kind, target_port)
    else {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布连线端口不匹配: {source_kind}.{source_port} -> {target_kind}.{target_port}"
        )));
    };
    if !allowed_roles.contains(&role) {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布连线 role 无效: {source_kind}.{source_port} -> {target_kind}.{target_port} 不允许 role={role}"
        )));
    }
    Ok(())
}

fn canvas_record_edge_allowed_roles(
    source_kind: &str,
    source_port: &str,
    target_kind: &str,
    target_port: &str,
) -> Option<&'static [&'static str]> {
    if is_canvas_record_text_producer(source_kind) && source_port == "text" {
        return match (target_kind, target_port) {
            ("RenderTextNode", "input")
            | ("LlmTextNode", "input")
            | ("GenerateImageNode", "text")
            | ("GenerateVideoNode", "text")
            | ("TextProbeNode", "text")
            | ("TextOutputNode", "text") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    if is_canvas_record_image_producer(source_kind) && source_port == "image" {
        return match (target_kind, target_port) {
            ("GenerateVideoNode", "first_frame") => Some(FIRST_FRAME_EDGE_ROLES),
            ("GenerateVideoNode", "end_frame") => Some(END_FRAME_EDGE_ROLES),
            ("GenerateVideoNode", "reference_image") => Some(REFERENCE_IMAGE_EDGE_ROLES),
            ("ImageProbeNode", "image") | ("ImageOutputNode", "image") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    if is_canvas_record_video_producer(source_kind) && source_port == "video" {
        return match (target_kind, target_port) {
            ("ExtractFrameNode", "video") | ("TrimVideoNode", "video") => {
                Some(SOURCE_VIDEO_EDGE_ROLES)
            }
            ("AssembleVideoNode", "clip") => Some(ASSEMBLY_CLIP_EDGE_ROLES),
            ("VideoProbeNode", "video") => Some(CONTEXT_EDGE_ROLES),
            ("VideoOutputNode", "video") => Some(FINAL_VIDEO_EDGE_ROLES),
            _ => None,
        };
    }
    if is_canvas_record_audio_producer(source_kind) && source_port == "audio" {
        return match (target_kind, target_port) {
            ("AudioProbeNode", "audio") | ("AudioOutputNode", "audio") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    None
}

fn is_canvas_record_text_producer(kind: &str) -> bool {
    matches!(
        kind,
        "TextInputNode" | "RenderTextNode" | "LlmTextNode" | "TextProbeNode"
    )
}

fn is_canvas_record_image_producer(kind: &str) -> bool {
    matches!(
        kind,
        "ImageInputNode" | "GenerateImageNode" | "ExtractFrameNode" | "ImageProbeNode"
    )
}

fn is_canvas_record_video_producer(kind: &str) -> bool {
    matches!(
        kind,
        "VideoInputNode"
            | "GenerateVideoNode"
            | "TrimVideoNode"
            | "AssembleVideoNode"
            | "VideoProbeNode"
    )
}

fn is_canvas_record_audio_producer(kind: &str) -> bool {
    matches!(kind, "AudioInputNode" | "AudioProbeNode")
}
