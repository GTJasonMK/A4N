use a4n_core::models::ai::generated_video;

pub(crate) fn is_canvas_video_record(video: &generated_video::Model) -> bool {
    video.chapter_id.is_none()
        && matches!(
            video.workflow_mode.as_deref(),
            Some("canvas_video" | "canvas_reference_media" | "canvas_trim" | "canvas_assembly")
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CanvasVideoWorkflowContext {
    pub(crate) workflow_run_id: i64,
    pub(crate) graph_id: i64,
    pub(crate) node_id: i64,
}

pub(crate) fn provider_poll_config_id(
    video: &generated_video::Model,
) -> Result<Option<i64>, String> {
    let raw_config_id = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("config_id"));
    if !is_canvas_video_record(video) {
        return raw_config_id
            .map(|value| positive_provider_poll_config_id(video, value))
            .transpose();
    }
    let config_id = raw_config_id.ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json.config_id，不能恢复 provider 轮询",
            video.id
        )
    })?;
    positive_provider_poll_config_id(video, config_id).map(Some)
}

fn positive_provider_poll_config_id(
    video: &generated_video::Model,
    value: &serde_json::Value,
) -> Result<i64, String> {
    let config_id = value.as_i64().ok_or_else(|| {
        format!(
            "视频 #{} 的 metadata_json.config_id 必须是正整数，不能恢复 provider 轮询",
            video.id
        )
    })?;
    if config_id <= 0 {
        return Err(format!(
            "视频 #{} 的 metadata_json.config_id 必须是正整数，不能恢复 provider 轮询",
            video.id
        ));
    }
    Ok(config_id)
}

pub(crate) fn canvas_video_workflow_context(
    video: &generated_video::Model,
) -> Result<Option<CanvasVideoWorkflowContext>, String> {
    if !is_canvas_video_record(video) {
        return Ok(None);
    }
    let metadata = video.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json，不能同步 canvas workflow",
            video.id
        )
    })?;
    let canvas = metadata.get("canvas").ok_or_else(|| {
        format!(
            "画布视频 #{} 缺少 metadata_json.canvas，不能同步 canvas workflow",
            video.id
        )
    })?;
    let context = CanvasVideoWorkflowContext {
        workflow_run_id: required_canvas_video_context_id(video.id, canvas, "workflow_run_id")?,
        graph_id: required_canvas_video_context_id(video.id, canvas, "graph_id")?,
        node_id: required_canvas_video_context_id(video.id, canvas, "node_id")?,
    };
    Ok(Some(context))
}

fn required_canvas_video_context_id(
    video_id: i64,
    canvas: &serde_json::Value,
    field: &str,
) -> Result<i64, String> {
    let value = canvas
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| {
            format!(
                "画布视频 #{video_id} 缺少 metadata_json.canvas.{field}，不能同步 canvas workflow"
            )
        })?;
    if value <= 0 {
        return Err(format!(
            "画布视频 #{video_id} 的 metadata_json.canvas.{field} 必须是正整数"
        ));
    }
    Ok(value)
}
