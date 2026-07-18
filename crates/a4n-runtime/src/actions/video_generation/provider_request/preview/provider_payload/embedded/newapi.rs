use super::super::super::super::*;
use super::super::super::notes::{
    append_optional_input_note, append_video_request_prompt_length_note,
};

pub(super) fn newapi_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    let request =
        NewApiVideoProvider::request_body_preview(&context.request).map_err(|e| e.to_string())?;
    if context.workflow_mode == "storyboard" {
        let mut note = format!(
            "New API 的 `image` 字段只作为视频首帧/初始图，不支持分镜模式需要的 reference_images；provider 参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，真实生成会被阻止，请切换到支持参考图输入的视频 provider。",
            context.provider_reference_image_count
        );
        append_optional_input_note(&mut note, context.provider_input_note.as_deref());
        append_video_request_prompt_length_note(&mut note, &context.request);
        return Ok((
            "video_generation_newapi",
            "New API /video/generations 请求",
            note,
            request,
        ));
    }
    let image_source = if context.workflow_mode == "first_frame_chain" {
        "首帧串联起始帧"
    } else {
        "首尾帧模式首帧"
    };
    let mut note = format!(
        "New API 的 `image` 字段只作为视频首帧/初始图；提示词参考图不会放入 `image`，也不会作为参考图参数发送。{} image 绑定 generated_image：{}；尾帧 generated_image：{}，参数不发送；提示词参考图 ID 数量：{}，参数不发送；预览省略真实媒体输入，生成时仅首帧会作为 PNG/JPEG data URL 发送。",
        image_source,
        context
            .boundary_frame_ids
            .first
            .map(|id| id.to_string())
            .unwrap_or_else(|| "无".to_string()),
        context
            .boundary_frame_ids
            .last
            .map(|id| id.to_string())
            .unwrap_or_else(|| "无".to_string()),
        context.prompt_reference_image_count
    );
    append_optional_input_note(&mut note, context.provider_input_note.as_deref());
    append_video_request_prompt_length_note(&mut note, &context.request);
    return Ok((
        "video_generation_newapi",
        "New API /video/generations 请求",
        note,
        request,
    ));
}
