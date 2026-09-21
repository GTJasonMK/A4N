use serde_json::Value;

pub(super) fn extract_task_id(v: &Value) -> Option<String> {
    text_at(v, &["task_id"])
        .or_else(|| text_at(v, &["id"]))
        .or_else(|| text_at(v, &["data", "task_id"]))
        .or_else(|| text_at(v, &["data", "id"]))
        .map(str::to_string)
}

pub(super) fn extract_status(v: &Value) -> Option<&str> {
    text_at(v, &["status"]).or_else(|| text_at(v, &["data", "status"]))
}

pub(super) fn result_video_url(v: &Value) -> Option<&str> {
    text_at(v, &["result", "video_url"])
        .or_else(|| text_at(v, &["data", "result", "video_url"]))
        .or_else(|| text_at(v, &["video_url"]))
}

pub(super) fn failure_message(v: &Value) -> String {
    text_at(v, &["error", "message"])
        .or_else(|| text_at(v, &["data", "error", "message"]))
        .or_else(|| text_at(v, &["message"]))
        .unwrap_or("Seedance 2.0 视频任务失败")
        .to_string()
}

pub(super) fn text_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}
