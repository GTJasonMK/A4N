use crate::actions::image_generation::GenerateCanvasImageInput;
use crate::actions::video_generation::GenerateCanvasVideoInput;
use crate::state::AppState;
use a4n_core::models::canvas::{canvas_edge, canvas_node};
use a4n_services::canvas::CanvasGraphDocument;
use serde_json::Value;

#[derive(Debug)]
pub(super) struct CanvasRenderedPrompt {
    pub(super) prompt_kind: String,
    pub(super) prompt_text: String,
    pub(super) canvas_text_asset_id: Option<i64>,
    pub(super) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(super) struct CanvasLlmTransformInput {
    pub(super) system_prompt: String,
    pub(super) user_prompt: String,
    pub(super) instruction: String,
    pub(super) source_context: String,
    pub(super) output_kind: String,
    pub(super) temperature: Option<f32>,
    pub(super) max_tokens: Option<usize>,
    pub(super) config_id: Option<i64>,
    pub(super) canvas_text_asset_id: Option<i64>,
    pub(super) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(super) struct CanvasExportVideoInput {
    pub(super) video_id: Option<i64>,
    pub(super) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(super) struct CanvasOutputValue {
    pub(super) text: Option<String>,
    pub(super) generated_image_id: Option<i64>,
    pub(super) generated_video_id: Option<i64>,
    pub(super) media_asset_id: Option<i64>,
    pub(super) source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerateVideoInputMode {
    TextOnly,
    FirstFrame,
    FirstEndFrame,
    ReferenceImages,
}

pub(super) async fn build_canvas_video_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasVideoInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut prompt_parts = Vec::new();
    let mut first_frame_image_id = None;
    let mut last_frame_image_id = None;
    let mut reference_image_ids = Vec::new();
    let mut source_node_ids = Vec::new();

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(prompt) = source_text_value(
            state,
            workflow_run_id,
            source,
            document,
            "GenerateVideoNode",
        )
        .await?
        {
            prompt_parts.push(prompt);
        }
        if let Some(image_id) =
            source_generated_image_id(state, workflow_run_id, source, "GenerateVideoNode").await?
        {
            match video_image_input_port(edge)? {
                "first_frame" => set_single_image_input(
                    &mut first_frame_image_id,
                    image_id,
                    "GenerateVideoNode 首帧输入",
                )?,
                "end_frame" => set_single_image_input(
                    &mut last_frame_image_id,
                    image_id,
                    "GenerateVideoNode 尾帧输入",
                )?,
                "reference_image" if !reference_image_ids.contains(&image_id) => {
                    reference_image_ids.push(image_id);
                }
                _ => {}
            }
        }
    }
    let config_id = canvas_config_i64(&node.config_json, "video_config_id");

    let prompt = prompt_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if prompt.trim().is_empty() {
        return Err("GenerateVideoNode 缺少 prompt 输入".to_string());
    }
    let input_mode = generate_video_input_mode(node.config_json.get("input_mode"))?;
    validate_generate_video_image_inputs(
        input_mode,
        first_frame_image_id,
        last_frame_image_id,
        &reference_image_ids,
    )?;
    Ok(GenerateCanvasVideoInput {
        user_id,
        project_id,
        graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt,
        negative_prompt: canvas_config_string(&node.config_json, &["negative_prompt"]),
        config_id,
        width: canvas_config_i64(&node.config_json, "width").and_then(i64_to_i32),
        height: canvas_config_i64(&node.config_json, "height").and_then(i64_to_i32),
        duration_secs: canvas_config_i64(&node.config_json, "duration_secs").and_then(i64_to_i32),
        fps: canvas_config_i64(&node.config_json, "fps").and_then(i64_to_i32),
        first_frame_image_id,
        last_frame_image_id,
        reference_image_ids,
        source_node_ids,
    })
}

pub(super) async fn build_canvas_image_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasImageInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut prompt_parts = Vec::new();
    let mut source_node_ids = Vec::new();

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(prompt) = source_text_value(
            state,
            workflow_run_id,
            source,
            document,
            "GenerateImageNode",
        )
        .await?
        {
            prompt_parts.push(prompt);
        }
    }
    let config_id = canvas_config_i64(&node.config_json, "image_config_id");

    let prompt = prompt_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if prompt.trim().is_empty() {
        return Err("GenerateImageNode 缺少 prompt 输入".to_string());
    }
    let _ = user_id;
    Ok(GenerateCanvasImageInput {
        project_id,
        graph_id,
        node_id: node.id,
        workflow_run_id: None,
        prompt,
        config_id,
        source_node_ids,
    })
}

fn video_image_input_port(edge: &canvas_edge::Model) -> Result<&str, String> {
    match edge.target_port.as_str() {
        "first_frame" | "end_frame" | "reference_image" => Ok(edge.target_port.as_str()),
        _ => Err(format!(
            "GenerateVideoNode 图片输入端口无效: target_port={}, role={}",
            edge.target_port, edge.role
        )),
    }
}

fn set_single_image_input(
    slot: &mut Option<i64>,
    image_id: i64,
    label: &str,
) -> Result<(), String> {
    if let Some(existing) = *slot
        && existing != image_id
    {
        return Err(format!("{label}只允许一个来源"));
    }
    *slot = Some(image_id);
    Ok(())
}

fn generate_video_input_mode(value: Option<&Value>) -> Result<GenerateVideoInputMode, String> {
    match value.and_then(Value::as_str).map(str::trim) {
        Some("text_only") => Ok(GenerateVideoInputMode::TextOnly),
        Some("first_frame") => Ok(GenerateVideoInputMode::FirstFrame),
        Some("first_end_frame") => Ok(GenerateVideoInputMode::FirstEndFrame),
        Some("reference_images") => Ok(GenerateVideoInputMode::ReferenceImages),
        Some(other) if !other.is_empty() => {
            Err(format!("GenerateVideoNode input_mode 无效: {other}"))
        }
        _ => Ok(GenerateVideoInputMode::TextOnly),
    }
}

fn validate_generate_video_image_inputs(
    mode: GenerateVideoInputMode,
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: &[i64],
) -> Result<(), String> {
    match mode {
        GenerateVideoInputMode::TextOnly => {
            if first_frame_image_id.is_some()
                || last_frame_image_id.is_some()
                || !reference_image_ids.is_empty()
            {
                return Err("GenerateVideoNode 当前为仅提示词模式，不能连接图片输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstFrame => {
            if first_frame_image_id.is_none() {
                return Err("GenerateVideoNode 首帧模式缺少 first_frame 输入".to_string());
            }
            if last_frame_image_id.is_some() || !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首帧模式只能连接 first_frame 输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstEndFrame => {
            if first_frame_image_id.is_none() || last_frame_image_id.is_none() {
                return Err(
                    "GenerateVideoNode 首尾帧模式必须同时连接 first_frame 和 end_frame 输入"
                        .to_string(),
                );
            }
            if !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首尾帧模式不能连接 reference_image 输入".to_string());
            }
        }
        GenerateVideoInputMode::ReferenceImages => {
            if reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 参考图模式缺少 reference_image 输入".to_string());
            }
            if first_frame_image_id.is_some() || last_frame_image_id.is_some() {
                return Err("GenerateVideoNode 参考图模式只能连接 reference_image 输入".to_string());
            }
        }
    }
    Ok(())
}

pub(super) async fn build_canvas_trim_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, f32, f32, Vec<i64>), String> {
    let incoming = incoming_edges(document, node.id);
    let mut video_id = None;
    let mut source_node_ids = Vec::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "TrimVideoNode").await?
        {
            video_id = Some(id);
        }
    }
    let video_id = video_id.ok_or_else(|| "TrimVideoNode 缺少源视频".to_string())?;
    let start_secs = canvas_config_f32(&node.config_json, "start_secs").unwrap_or(0.0);
    let end_secs = canvas_config_f32(&node.config_json, "end_secs")
        .ok_or_else(|| "TrimVideoNode 缺少 end_secs".to_string())?;
    Ok((video_id, start_secs, end_secs, source_node_ids))
}

pub(super) async fn build_canvas_extract_frame_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, String, Option<f32>, Vec<i64>), String> {
    let incoming = incoming_edges(document, node.id);
    let mut video_id = None;
    let mut source_node_ids = Vec::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "ExtractFrameNode").await?
        {
            video_id = Some(id);
        }
    }
    let video_id = video_id.ok_or_else(|| "ExtractFrameNode 缺少源视频".to_string())?;
    let frame_kind = canvas_config_string(&node.config_json, &["frame_kind", "kind"])
        .unwrap_or_else(|| "first".to_string());
    let at_secs = canvas_config_f32(&node.config_json, "at_secs");
    Ok((video_id, frame_kind, at_secs, source_node_ids))
}

pub(super) async fn build_canvas_assemble_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(Vec<i64>, Option<String>, Vec<i64>), String> {
    if node.config_json.get("video_ids").is_some() {
        return Err(
            "AssembleVideoNode 不支持 config_json.video_ids，请通过 VideoInputNode 连线提供视频片段"
                .to_string(),
        );
    }
    let incoming = incoming_edges(document, node.id);
    let mut clips = Vec::<(i64, i64, i64)>::new();
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        if let Some(id) =
            source_generated_video_id(state, workflow_run_id, source, "AssembleVideoNode").await?
        {
            clips.push((canvas_edge_order_index(edge)?, id, source.id));
        }
    }
    if clips.is_empty() {
        return Err("AssembleVideoNode 缺少视频片段".to_string());
    }
    clips.sort_by_key(|(order_index, _, _)| *order_index);
    validate_canvas_clip_order(&clips)?;
    let video_ids = clips.iter().map(|(_, video_id, _)| *video_id).collect();
    let source_node_ids = clips
        .iter()
        .map(|(_, _, source_node_id)| *source_node_id)
        .collect();
    let title = canvas_config_string(&node.config_json, &["title", "name"]);
    Ok((video_ids, title, source_node_ids))
}

pub(super) async fn build_canvas_export_video_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasExportVideoInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut video_id = None;
    let mut source_node_ids = Vec::new();

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if edge.target_port == "video"
            && let Some(id) =
                source_generated_video_id(state, workflow_run_id, source, "VideoOutputNode").await?
        {
            video_id = Some(id);
        }
    }

    if video_id.is_none() {
        return Err("VideoOutputNode 缺少可输出的视频输入".to_string());
    }
    Ok(CanvasExportVideoInput {
        video_id,
        source_node_ids,
    })
}

pub(super) async fn build_canvas_output_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    expected_type: &str,
    consumer_label: &str,
) -> Result<CanvasOutputValue, String> {
    let incoming = incoming_edges(document, node.id);
    let mut output = CanvasOutputValue {
        text: None,
        generated_image_id: None,
        generated_video_id: None,
        media_asset_id: None,
        source_node_ids: Vec::new(),
    };
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        output.source_node_ids.push(source.id);
        match expected_type {
            "text" => {
                if let Some(text) =
                    source_text_value(state, workflow_run_id, source, document, consumer_label)
                        .await?
                {
                    output.text = Some(text);
                }
            }
            "image" => {
                output.generated_image_id =
                    source_generated_image_id(state, workflow_run_id, source, consumer_label)
                        .await?;
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            "video" => {
                output.generated_video_id =
                    source_generated_video_id(state, workflow_run_id, source, consumer_label)
                        .await?;
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            "audio" => {
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            other => return Err(format!("输出节点数据类型无效: {other}")),
        }
    }
    match expected_type {
        "text" if output.text.as_deref().unwrap_or("").trim().is_empty() => {
            Err(format!("{consumer_label} 缺少文本输入"))
        }
        "image" if output.generated_image_id.is_none() && output.media_asset_id.is_none() => {
            Err(format!("{consumer_label} 缺少图片输入"))
        }
        "video" if output.generated_video_id.is_none() && output.media_asset_id.is_none() => {
            Err(format!("{consumer_label} 缺少视频输入"))
        }
        "audio" if output.media_asset_id.is_none() => Err(format!("{consumer_label} 缺少音频输入")),
        _ => Ok(output),
    }
}

pub(super) async fn build_canvas_prompt_artifact_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasRenderedPrompt, String> {
    let incoming = incoming_edges(document, node.id);
    let mut source_parts = Vec::new();
    let mut source_node_ids = Vec::new();
    let mut canvas_text_asset_id = node
        .ref_type
        .as_deref()
        .is_some_and(|value| value == "canvas_text_asset")
        .then_some(node.ref_id)
        .flatten()
        .or_else(|| canvas_config_i64(&node.config_json, "canvas_text_asset_id"));

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(asset_id) = source_canvas_text_asset_id(source)
            && canvas_text_asset_id.is_none()
        {
            canvas_text_asset_id = Some(asset_id);
        }
        if let Some(text) =
            source_text_value(state, workflow_run_id, source, document, "RenderTextNode").await?
        {
            source_parts.push(text);
        }
    }

    let template = canvas_config_string(&node.config_json, &["template", "template_text"]);
    let context = source_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let template = template.ok_or_else(|| "RenderTextNode 缺少模板".to_string())?;
    let prompt_text = template
        .replace("{{context}}", &context)
        .replace("{{input}}", &context)
        .replace("{{content}}", &context);
    if prompt_text.trim().is_empty() {
        return Err("RenderTextNode 缺少可渲染的文本内容".to_string());
    }
    let prompt_kind = canvas_config_string(&node.config_json, &["output_kind", "prompt_kind"])
        .unwrap_or_else(|| "canvas_prompt".to_string());
    Ok(CanvasRenderedPrompt {
        prompt_kind,
        prompt_text,
        canvas_text_asset_id,
        source_node_ids,
    })
}

pub(super) async fn build_canvas_llm_transform_input_for_run(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasLlmTransformInput, String> {
    let incoming = incoming_edges(document, node.id);
    let mut source_parts = Vec::new();
    let mut source_node_ids = Vec::new();
    let mut canvas_text_asset_id = node
        .ref_type
        .as_deref()
        .is_some_and(|value| value == "canvas_text_asset")
        .then_some(node.ref_id)
        .flatten()
        .or_else(|| canvas_config_i64(&node.config_json, "canvas_text_asset_id"));

    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        source_node_ids.push(source.id);
        if let Some(asset_id) = source_canvas_text_asset_id(source)
            && canvas_text_asset_id.is_none()
        {
            canvas_text_asset_id = Some(asset_id);
        }
        if let Some(text) =
            source_text_value(state, workflow_run_id, source, document, "LlmTextNode").await?
        {
            source_parts.push(text);
        }
    }
    let config_id = canvas_config_i64(&node.config_json, "llm_config_id");

    let instruction = canvas_config_string(&node.config_json, &["instruction", "prompt"])
        .unwrap_or_else(|| {
            "将输入内容改写为适合视频画布后续节点使用的结构化创作文本。".to_string()
        });
    let context = source_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if context.trim().is_empty() {
        return Err("LlmTextNode 缺少输入文本".to_string());
    }
    let output_kind = canvas_config_string(&node.config_json, &["output_kind", "text_kind"])
        .unwrap_or_else(|| "shot_script".to_string());
    let system_prompt = format!(
        "你是 A4N 视频画布的文本转换节点。请根据用户指令转换输入文本，输出类型为 {output_kind}。只返回转换后的正文，不要解释过程。"
    );
    let user_prompt = format!("指令：{instruction}\n\n输入：\n{context}");
    let temperature = canvas_config_f32(&node.config_json, "temperature");
    let max_tokens = canvas_config_i64(&node.config_json, "max_tokens")
        .and_then(|value| usize::try_from(value).ok());

    Ok(CanvasLlmTransformInput {
        system_prompt,
        user_prompt,
        instruction,
        source_context: context,
        output_kind,
        temperature,
        max_tokens,
        config_id,
        canvas_text_asset_id,
        source_node_ids,
    })
}

pub(super) fn first_canvas_text_asset_id(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Option<i64> {
    if node.ref_type.as_deref() == Some("canvas_text_asset") {
        return node.ref_id;
    }
    canvas_config_i64(&node.config_json, "canvas_text_asset_id").or_else(|| {
        incoming_edges(document, node.id)
            .into_iter()
            .find_map(|edge| {
                document
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == edge.source_node_id)
                    .and_then(|source| {
                        if source.ref_type.as_deref() == Some("canvas_text_asset") {
                            source.ref_id
                        } else {
                            canvas_config_i64(&source.config_json, "canvas_text_asset_id")
                        }
                    })
            })
    })
}

fn incoming_edges(document: &CanvasGraphDocument, node_id: i64) -> Vec<&canvas_edge::Model> {
    document
        .edges
        .iter()
        .filter(|edge| edge.target_node_id == node_id)
        .collect()
}

fn canvas_edge_order_index(edge: &canvas_edge::Model) -> Result<i64, String> {
    let order_index = edge
        .config_json
        .get("order_index")
        .and_then(Value::as_i64)
        .ok_or_else(|| "AssembleVideoNode 片段连线必须设置 order_index".to_string())?;
    if order_index <= 0 {
        return Err("AssembleVideoNode 片段连线 order_index 必须是正整数".to_string());
    }
    Ok(order_index)
}

fn validate_canvas_clip_order(clips: &[(i64, i64, i64)]) -> Result<(), String> {
    for (index, (order_index, _, _)) in clips.iter().enumerate() {
        let expected = i64::try_from(index + 1).unwrap_or(i64::MAX);
        if *order_index != expected {
            return Err(format!(
                "AssembleVideoNode 片段顺序必须从 1 连续递增，缺少 order_index={expected}"
            ));
        }
    }
    Ok(())
}

fn canvas_config_string(config: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        config
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn canvas_config_i64(config: &Value, key: &str) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
}

async fn source_text_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    consumer_label: &str,
) -> Result<Option<String>, String> {
    if canvas_node_is_operation(&node.node_kind) {
        let Some(value) = checkpoint_output_value(state, workflow_run_id, node.id).await? else {
            return Err(unresolved_upstream_operation_error(consumer_label));
        };
        return Ok(value
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string));
    }
    if let Some(asset_id) = source_canvas_text_asset_id(node)
        && let Some(asset) = document
            .text_assets
            .iter()
            .find(|candidate| candidate.id == asset_id)
    {
        return Ok(Some(asset.content.clone()));
    }
    Ok(canvas_config_string(
        &node.config_json,
        &[
            "text",
            "content",
            "prompt_text",
            "prompt",
            "title",
            "review_note",
        ],
    ))
}

async fn source_generated_image_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if canvas_node_is_operation(&node.node_kind) {
        let Some(value) = checkpoint_output_value(state, workflow_run_id, node.id).await? else {
            return Err(unresolved_upstream_operation_error(consumer_label));
        };
        return Ok(value.get("generated_image_id").and_then(Value::as_i64));
    }
    if matches!(
        node.ref_type.as_deref(),
        Some("generated_image" | "legacy_generated_image")
    ) {
        return Ok(node.ref_id);
    }
    Ok(None)
}

async fn source_generated_video_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if canvas_node_is_operation(&node.node_kind) {
        let Some(value) = checkpoint_output_value(state, workflow_run_id, node.id).await? else {
            return Err(unresolved_upstream_operation_error(consumer_label));
        };
        return Ok(value.get("generated_video_id").and_then(Value::as_i64));
    }
    if matches!(
        node.ref_type.as_deref(),
        Some("generated_video" | "legacy_generated_video")
    ) {
        return Ok(node.ref_id);
    }
    Ok(None)
}

async fn source_media_asset_id(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    consumer_label: &str,
) -> Result<Option<i64>, String> {
    if canvas_node_is_operation(&node.node_kind) {
        let Some(value) = checkpoint_output_value(state, workflow_run_id, node.id).await? else {
            return Err(unresolved_upstream_operation_error(consumer_label));
        };
        return Ok(value.get("media_asset_id").and_then(Value::as_i64));
    }
    if node.ref_type.as_deref() == Some("media_asset") {
        return Ok(node.ref_id);
    }
    Ok(None)
}

async fn checkpoint_output_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node_id: i64,
) -> Result<Option<Value>, String> {
    let Some(state) = state else {
        return Ok(None);
    };
    let step_key = format!("canvas_node:{node_id}");
    if let Some(workflow_run_id) = workflow_run_id {
        let checkpoints = state
            .workflow_run
            .checkpoints(workflow_run_id)
            .await
            .map_err(|err| err.to_string())?;
        if let Some(value) = checkpoints
            .into_iter()
            .rev()
            .find(|checkpoint| checkpoint.step_key == step_key && checkpoint.status == "completed")
            .and_then(|checkpoint| checkpoint.checkpoint_json.or(checkpoint.artifacts_json))
        {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn unresolved_upstream_operation_error(consumer_label: &str) -> String {
    format!(
        "{consumer_label} 不能直接读取未运行的上游操作节点，请先使用“运行到此节点”或整图运行生成当前 workflow checkpoint"
    )
}

fn source_canvas_text_asset_id(node: &canvas_node::Model) -> Option<i64> {
    node.ref_type
        .as_deref()
        .is_some_and(|value| value == "canvas_text_asset")
        .then_some(node.ref_id)
        .flatten()
        .or_else(|| canvas_config_i64(&node.config_json, "canvas_text_asset_id"))
}

fn canvas_node_is_operation(kind: &str) -> bool {
    matches!(
        kind,
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
    )
}

fn canvas_config_f32(config: &Value, key: &str) -> Option<f32> {
    config
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value as f32)
}

fn i64_to_i32(value: i64) -> Option<i32> {
    i32::try_from(value).ok().filter(|value| *value > 0)
}

#[cfg(test)]
fn block_on_input<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(future)
}

#[cfg(test)]
fn build_canvas_video_input(
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasVideoInput, String> {
    block_on_input(build_canvas_video_input_for_run(
        None, None, user_id, project_id, graph_id, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_image_input(
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<GenerateCanvasImageInput, String> {
    block_on_input(build_canvas_image_input_for_run(
        None, None, user_id, project_id, graph_id, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_trim_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, f32, f32, Vec<i64>), String> {
    block_on_input(build_canvas_trim_input_for_run(None, None, node, document))
}

#[cfg(test)]
fn build_canvas_extract_frame_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(i64, String, Option<f32>, Vec<i64>), String> {
    block_on_input(build_canvas_extract_frame_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_assemble_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<(Vec<i64>, Option<String>, Vec<i64>), String> {
    block_on_input(build_canvas_assemble_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_export_video_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasExportVideoInput, String> {
    block_on_input(build_canvas_export_video_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_prompt_artifact_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasRenderedPrompt, String> {
    block_on_input(build_canvas_prompt_artifact_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_llm_transform_input(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
) -> Result<CanvasLlmTransformInput, String> {
    block_on_input(build_canvas_llm_transform_input_for_run(
        None, None, node, document,
    ))
}

#[cfg(test)]
fn build_canvas_output_value_for_test(
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    expected_type: &str,
    consumer_label: &str,
) -> Result<CanvasOutputValue, String> {
    block_on_input(build_canvas_output_value(
        None,
        None,
        node,
        document,
        expected_type,
        consumer_label,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::canvas::canvas_graph;
    use serde_json::json;

    fn test_graph_document(
        nodes: Vec<canvas_node::Model>,
        edges: Vec<canvas_edge::Model>,
    ) -> CanvasGraphDocument {
        let now = chrono::Utc::now().naive_utc();
        CanvasGraphDocument {
            graph: canvas_graph::Model {
                id: 10,
                project_id: 20,
                name: "测试画布".to_string(),
                graph_kind: "video_studio".to_string(),
                version: 1,
                viewport_json: None,
                created_at: now,
                updated_at: now,
            },
            nodes,
            edges,
            text_assets: Vec::new(),
        }
    }

    fn test_node(id: i64, kind: &str, config_json: Value) -> canvas_node::Model {
        let now = chrono::Utc::now().naive_utc();
        canvas_node::Model {
            id,
            graph_id: 10,
            node_kind: kind.to_string(),
            ref_type: None,
            ref_id: None,
            config_json,
            position_json: json!({"x": 0, "y": 0}),
            state: "ready".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn test_edge(source_node_id: i64, target_node_id: i64, role: &str) -> canvas_edge::Model {
        canvas_edge::Model {
            id: source_node_id * 100 + target_node_id,
            graph_id: 10,
            source_node_id,
            source_port: "output".to_string(),
            target_node_id,
            target_port: "input".to_string(),
            role: role.to_string(),
            config_json: json!({}),
        }
    }

    fn test_typed_edge(
        source_node_id: i64,
        target_node_id: i64,
        source_port: &str,
        target_port: &str,
        role: &str,
    ) -> canvas_edge::Model {
        canvas_edge::Model {
            source_port: source_port.to_string(),
            target_port: target_port.to_string(),
            ..test_edge(source_node_id, target_node_id, role)
        }
    }

    fn test_edge_with_order(
        source_node_id: i64,
        target_node_id: i64,
        role: &str,
        order_index: i64,
    ) -> canvas_edge::Model {
        canvas_edge::Model {
            config_json: json!({
                "order_index": order_index
            }),
            ..test_edge(source_node_id, target_node_id, role)
        }
    }

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

        let input = build_canvas_image_input(30, 20, 10, &image_node, &document).unwrap();

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

        let input = build_canvas_image_input(30, 20, 10, &image_node, &document).unwrap();

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

        let input = build_canvas_image_input(30, 20, 10, &image_node, &document).unwrap();

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
        let image_input =
            build_canvas_image_input(30, 20, 10, &image_node, &image_document).unwrap();
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
        let video_input =
            build_canvas_video_input(30, 20, 10, &video_node, &video_document).unwrap();
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
        let err = build_canvas_image_input(30, 20, 10, &image_node, &image_document)
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
            30,
            20,
            10,
            &image_node,
            &document,
        ))
        .expect_err("upstream operation without current-run checkpoint must fail");

        assert!(err.contains("当前 workflow checkpoint"));
    }

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
        let input =
            build_canvas_video_input(30, 20, 10, &reference_node, &reference_document).unwrap();
        assert_eq!(input.reference_image_ids, vec![92]);
        assert_eq!(input.first_frame_image_id, None);
        assert_eq!(input.last_frame_image_id, None);
    }

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
        let err = build_canvas_output_value_for_test(
            &audio_output,
            &document,
            "audio",
            "AudioOutputNode",
        )
        .expect_err("media_asset_id config must not be accepted as a hidden audio output");
        assert!(err.contains("缺少音频输入"));
    }

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
}
