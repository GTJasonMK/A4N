use crate::video::prompt_json::validation::API_PROMPT_UPLOAD_STATUS_TERMS;
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;

pub(in crate::video) fn validate_prompt_has_no_upload_status(
    value: &str,
    field: &str,
) -> Result<(), AppError> {
    if let Some(term) = video_prompt_upload_status_term(value) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能写上传状态说明“{term}”；有输入帧时描述输入帧画面，没有输入帧时直接描述文字起点或上一段尾态"
        )));
    }
    Ok(())
}

pub(in crate::video::prompt_json::validation) fn validate_prompt_has_required_sections(
    value: &str,
    field: &str,
    field_key: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if let Some(section) = video_prompt_stale_section_for_language(field_key, value) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 使用旧版视频提示词章节 {section}；请重新生成或优化为当前格式"
        )));
    }
    if let Some(section) =
        video_prompt_missing_required_section_for_language(field_key, value, prompt_output_language)
    {
        return Err(AppError::InvalidArgument(format!(
            "{field} 缺少必需章节 {section}"
        )));
    }
    if field_key == "visual_prompt" {
        validate_prompt_has_shot_storyboard_section(value, field, prompt_output_language)?;
    }
    Ok(())
}

pub(in crate::video) fn validate_prompt_has_shot_storyboard_section(
    value: &str,
    field: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let sections = prompt_output_language.visual_sections();
    let Some(section_title) = sections.get(2).copied() else {
        return Ok(());
    };
    let section = prompt_section_content(value, section_title, sections).ok_or_else(|| {
        AppError::InvalidArgument(format!("{field} 缺少必需章节 {section_title}"))
    })?;
    if !contains_shot_number_marker(section, prompt_output_language) {
        let example = match prompt_output_language {
            PromptOutputLanguage::ZhCn => "镜头 1：",
            PromptOutputLanguage::En => "Shot 1:",
        };
        return Err(AppError::InvalidArgument(format!(
            "{field} 的 {section_title} 必须使用镜头编号组织，例如“{example}”，不能只写动作时间段"
        )));
    }
    if contains_time_table_marker(section) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 的 {section_title} 必须按镜头编号描述运镜、动作和空间变化，不能写成“总时长/0-3秒/2-4 seconds”这类精确时间表"
        )));
    }
    Ok(())
}

fn prompt_section_content<'a>(
    text: &'a str,
    section_title: &str,
    sections: &[&str],
) -> Option<&'a str> {
    let start = text.find(section_title)?;
    let content_start = start + section_title.len();
    let rest = &text[content_start..];
    let end = sections
        .iter()
        .filter(|section| **section != section_title)
        .filter_map(|section| rest.find(section))
        .min()
        .unwrap_or(rest.len());
    Some(rest[..end].trim())
}

fn contains_shot_number_marker(text: &str, prompt_output_language: PromptOutputLanguage) -> bool {
    match prompt_output_language {
        PromptOutputLanguage::ZhCn => ["镜头 1", "镜头1", "镜头 一", "镜头一", "镜头 １", "镜头１"]
            .iter()
            .any(|marker| text.contains(marker)),
        PromptOutputLanguage::En => {
            let lower = text.to_ascii_lowercase();
            ["shot 1", "shot1", "shot one"]
                .iter()
                .any(|marker| lower.contains(marker))
        }
    }
}

fn contains_time_table_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("总时长") || lower.contains("total duration") || contains_time_range_marker(text)
}

fn contains_time_range_marker(text: &str) -> bool {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        let (_, ch) = chars[i];
        if !ch.is_ascii_digit() {
            i += 1;
            continue;
        }
        i += 1;
        while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.') {
            i += 1;
        }
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || !matches!(chars[i].1, '-' | '–' | '—') {
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || !chars[i].1.is_ascii_digit() {
            continue;
        }
        i += 1;
        while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.') {
            i += 1;
        }
        let lookahead_start = chars.get(i).map(|(index, _)| *index).unwrap_or(text.len());
        let lookahead_end = chars
            .get((i + 12).min(chars.len()))
            .map(|(index, _)| *index)
            .unwrap_or(text.len());
        let lookahead = text[lookahead_start..lookahead_end].to_ascii_lowercase();
        if lookahead.contains('秒') || lookahead.contains("sec") || lookahead.contains("second") {
            return true;
        }
    }
    false
}

pub(in crate::video::prompt_json::validation) fn validate_motion_prompt_has_no_temporal_storyboard(
    value: &str,
) -> Result<(), AppError> {
    if video_motion_prompt_has_temporal_storyboard(value) {
        return Err(AppError::InvalidArgument(
            "视频提示词 motion_prompt 不应重复镜头分镜；镜头编号分镜只写在 visual_prompt 的 [镜头分镜] 中"
                .to_string(),
        ));
    }
    Ok(())
}

pub fn video_prompt_upload_status_term(text: &str) -> Option<&'static str> {
    API_PROMPT_UPLOAD_STATUS_TERMS
        .iter()
        .copied()
        .find(|term| text.contains(term))
}

pub fn video_prompt_missing_required_section_for_language(
    field: &str,
    text: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Option<&'static str> {
    let required_sections = match field {
        "visual_prompt" => prompt_output_language.visual_sections(),
        "motion_prompt" => prompt_output_language.motion_sections(),
        "frame_prompt" => prompt_output_language.frame_sections(),
        _ => return None,
    };
    let mut cursor = 0;
    for section in required_sections {
        let Some(offset) = text[cursor..].find(section) else {
            return Some(section);
        };
        cursor += offset + section.len();
    }
    None
}

pub fn video_motion_prompt_has_temporal_storyboard(text: &str) -> bool {
    text.lines().any(|line| {
        let text = line.trim();
        matches!(
            text,
            "[片段内时间分布]"
                | "【片段内时间分布】"
                | "[时序分镜]"
                | "【时序分镜】"
                | "[镜头分镜]"
                | "【镜头分镜】"
                | "[Timing Breakdown]"
                | "【Timing Breakdown】"
                | "[Temporal Storyboard]"
                | "【Temporal Storyboard】"
                | "[Shot Storyboard]"
                | "【Shot Storyboard】"
        )
    })
}

pub fn video_prompt_stale_section_for_language(field: &str, text: &str) -> Option<&'static str> {
    let stale_sections: &[&str] = match field {
        "visual_prompt" => &[
            "[输入图片介绍]",
            "【输入图片介绍】",
            "[片段内时间分布]",
            "【片段内时间分布】",
            "[时序分镜]",
            "【时序分镜】",
            "[Input Image Notes]",
            "【Input Image Notes】",
            "[Timing Breakdown]",
            "【Timing Breakdown】",
            "[Temporal Storyboard]",
            "【Temporal Storyboard】",
        ],
        "motion_prompt" => &[
            "[片段内时间分布]",
            "【片段内时间分布】",
            "[时序分镜]",
            "【时序分镜】",
            "[连续性要求]",
            "【连续性要求】",
            "[Timing Breakdown]",
            "【Timing Breakdown】",
            "[Temporal Storyboard]",
            "【Temporal Storyboard】",
            "[Continuity Requirements]",
            "【Continuity Requirements】",
        ],
        _ => &[],
    };
    stale_sections
        .iter()
        .copied()
        .find(|section| text.contains(section))
}
