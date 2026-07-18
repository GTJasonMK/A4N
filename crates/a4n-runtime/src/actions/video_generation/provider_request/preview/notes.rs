use super::super::*;

pub(in crate::actions::video_generation) fn append_provider_input_note(
    note: &mut Option<String>,
    addition: impl Into<String>,
) {
    let addition = addition.into();
    if addition.trim().is_empty() {
        return;
    }
    match note {
        Some(note) if !note.trim().is_empty() => {
            note.push('；');
            note.push_str(addition.trim());
        }
        _ => *note = Some(addition.trim().to_string()),
    }
}

pub(in crate::actions::video_generation) fn append_provider_input_notes(
    note: &mut Option<String>,
    additions: Vec<String>,
) {
    for addition in additions {
        append_provider_input_note(note, addition);
    }
}

pub(in crate::actions::video_generation) fn append_preview_duration_note(
    note: &mut Option<String>,
    cfg: &video_gen_config::Model,
    duration_secs: i32,
) {
    if let Err(err) = validate_video_duration_for_config_secs(cfg, duration_secs) {
        append_provider_input_note(
            note,
            format!(
                "当前视频配置不支持 {duration_secs} 秒；预览仍展示请求结构，真实生成会失败：{err}"
            ),
        );
    }
}

#[cfg(feature = "embedded-media-providers")]
pub(super) fn append_optional_input_note(note: &mut String, input_note: Option<&str>) {
    if let Some(input_note) = input_note {
        note.push('；');
        note.push_str(input_note);
    }
}

#[cfg(feature = "embedded-media-providers")]
pub(super) fn append_video_request_prompt_length_note(
    note: &mut String,
    request: &VideoGenerationRequest,
) {
    note.push('；');
    note.push_str(&video_request_prompt_length_context(request));
}
