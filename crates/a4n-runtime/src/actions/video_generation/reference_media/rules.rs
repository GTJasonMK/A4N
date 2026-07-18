use super::super::*;

pub(in crate::actions::video_generation) fn reference_media_prompt(
    value: &str,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<String, String> {
    let prompt = value.trim();
    if prompt.chars().count() >= MIN_PROVIDER_VISUAL_PROMPT_CHARS {
        return Ok(prompt.to_string());
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        Err(format!(
            "参考媒体视频 prompt 过短，至少需要 {} 个字符，并明确写出每个 @image_file_N / @video_file_N / @audio_file_N 的用途",
            MIN_PROVIDER_VISUAL_PROMPT_CHARS
        ))
    } else {
        append_provider_input_note(
            note,
            format!(
                "参考媒体视频 prompt 过短，至少需要 {} 个字符；预览使用占位 prompt 展示请求结构，真实生成仍需补全",
                MIN_PROVIDER_VISUAL_PROMPT_CHARS
            ),
        );
        Ok(if prompt.is_empty() {
            REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER.to_string()
        } else {
            format!("{prompt}\n\n{REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER}")
        })
    }
}

pub(in crate::actions::video_generation) fn reference_media_urls(
    values: Vec<String>,
    field: &str,
    max_count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<Vec<String>, String> {
    if values.len() > max_count {
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(format!("{field} 最多允许 {max_count} 个 URL"));
        }
        append_provider_input_note(
            note,
            format!(
                "{field} 选择了 {} 个 URL，超过真实生成上限 {max_count} 个；预览仍展示请求结构，真实生成会失败",
                values.len()
            ),
        );
    }
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let value = value.trim().to_string();
        if value.starts_with("https://") || value.starts_with("http://") {
            normalized.push(value);
            continue;
        }
        if purpose == VideoProviderRequestPurpose::Generate {
            if value.is_empty() {
                return Err(format!("{field}[{index}] 不能为空"));
            }
            return Err(format!(
                "{field}[{index}] 必须以小写 http:// 或 https:// 开头，并且可被 Seedance 2.0 服务访问"
            ));
        }
        let reason = if value.is_empty() {
            "为空"
        } else {
            "未以小写 http:// 或 https:// 开头"
        };
        append_provider_input_note(
            note,
            format!(
                "{field}[{index}] {reason}；预览使用占位 URL 展示请求结构，真实生成仍需填写以小写 http:// 或 https:// 开头且可访问的媒体 URL"
            ),
        );
        normalized.push(reference_media_preview_placeholder_url(field, index));
    }
    Ok(normalized)
}

pub(super) fn reference_media_preview_placeholder_url(field: &str, index: usize) -> String {
    format!("https://example.invalid/a4n-preview/{field}/{}", index + 1)
}

pub(in crate::actions::video_generation) fn ensure_reference_media_tokens(
    prompt: &mut String,
    token_prefix: &str,
    count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    let missing = (1..=count)
        .map(|index| format!("@{token_prefix}_{index}"))
        .filter(|token| !prompt.contains(token))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    if purpose == VideoProviderRequestPurpose::Generate {
        return Err(format!(
            "参考媒体 prompt 缺少 {} 的用途描述；每个 {token_prefix} 条目都必须在 prompt 中显式引用",
            missing[0]
        ));
    }
    append_provider_input_note(
        note,
        format!(
            "参考媒体 prompt 缺少 {} 的用途描述；预览追加 token 占位说明，真实生成仍需显式引用每个媒体 token",
            missing.join("、")
        ),
    );
    prompt.push_str("\n\n参考媒体绑定占位：\n");
    for token in missing {
        prompt.push_str(&format!(
            "- {token}：请求预览占位用途；真实生成前需要填写该媒体的主体、细节和约束。\n"
        ));
    }
    Ok(())
}

pub(crate) fn reference_media_dimensions_for_aspect_ratio(
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    match aspect_ratio.trim() {
        "21:9" => Ok((1680, 720)),
        "16:9" => Ok((1280, 720)),
        "4:3" => Ok((1024, 768)),
        "1:1" => Ok((1024, 1024)),
        "3:4" => Ok((768, 1024)),
        "9:16" => Ok((720, 1280)),
        other => Err(format!(
            "Seedance 2.0 参考媒体模式不支持画幅 {other}，请选择 21:9、16:9、4:3、1:1、3:4 或 9:16"
        )),
    }
}
