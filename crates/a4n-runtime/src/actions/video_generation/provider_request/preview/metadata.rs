use super::super::*;
use super::notes::append_provider_input_note;

mod fields;
mod placeholders;
mod sections;

use fields::{ensure_preview_object_field, ensure_preview_shot_sections};
use placeholders::{preview_placeholder_frame, preview_placeholder_shot};

pub(in crate::actions::video_generation) fn ensure_chapter_video_prompt_preview_metadata(
    prompt: &mut chapter_video_prompt::Model,
    note: &mut Option<String>,
) -> Result<(), String> {
    let provider_workflow = prompt_provider_workflow(prompt)?;
    let mut metadata = match prompt.metadata_json.take() {
        Some(Value::Object(map)) => Value::Object(map),
        Some(_) => {
            append_provider_input_note(
                note,
                "视频提示词 metadata_json 不是对象；预览使用结构化占位元数据展示 provider 请求，真实生成仍需重新生成完整视频提示词。",
            );
            json!({})
        }
        None => {
            append_provider_input_note(
                note,
                "视频提示词缺少结构化 metadata_json；预览使用结构化占位元数据展示 provider 请求，真实生成仍需重新生成完整视频提示词。",
            );
            json!({})
        }
    };
    let root = metadata
        .as_object_mut()
        .ok_or_else(|| "预览 metadata 占位构造失败".to_string())?;
    ensure_preview_object_field(root, "shot", preview_placeholder_shot(prompt), note, "shot");
    if let Some(shot) = root.get_mut("shot").and_then(Value::as_object_mut) {
        ensure_preview_shot_sections(shot, note);
    }
    if provider_workflow.uses_first_frame_chain() {
        if prompt.shot_number <= 1 {
            ensure_preview_object_field(
                root,
                "start_first_frame",
                preview_placeholder_frame(prompt, "首帧串联起始帧"),
                note,
                "start_first_frame",
            );
        }
    } else if provider_workflow.uses_storyboard() {
        ensure_preview_object_field(
            root,
            "storyboard_frame",
            preview_placeholder_frame(prompt, "分镜图"),
            note,
            "storyboard_frame",
        );
    } else {
        ensure_preview_object_field(
            root,
            "start_boundary_frame",
            preview_placeholder_frame(prompt, "首帧边界"),
            note,
            "start_boundary_frame",
        );
        ensure_preview_object_field(
            root,
            "end_boundary_frame",
            preview_placeholder_frame(prompt, "尾帧边界"),
            note,
            "end_boundary_frame",
        );
    }
    prompt.metadata_json = Some(metadata);
    Ok(())
}
