use super::super::super::super::*;
use super::super::notes::append_provider_input_note;
use super::sections::preview_shot_sections;

pub(super) fn ensure_preview_object_field(
    root: &mut Map<String, Value>,
    key: &str,
    replacement: Value,
    note: &mut Option<String>,
    label: &str,
) {
    if root.get(key).and_then(Value::as_object).is_some() {
        return;
    }
    let reason = if root.contains_key(key) {
        "不是对象"
    } else {
        "缺失"
    };
    root.insert(key.to_string(), replacement);
    append_provider_input_note(
        note,
        format!(
            "视频提示词 metadata.{label} {reason}；预览使用占位结构展示 provider 请求，真实生成仍需完整结构化元数据。"
        ),
    );
}

pub(super) fn ensure_preview_shot_sections(
    shot: &mut Map<String, Value>,
    note: &mut Option<String>,
) {
    for (key, value) in preview_shot_sections() {
        if shot.get(key).is_some() {
            continue;
        }
        shot.insert(key.to_string(), value);
        append_provider_input_note(
            note,
            format!(
                "视频提示词 metadata.shot.{key} 缺失；预览使用占位结构展示 provider 请求，真实生成仍需补全。"
            ),
        );
    }
}
