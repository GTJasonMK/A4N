use super::super::super::*;

pub(in crate::actions::video_generation) fn video_provider_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    Err(format!(
        "视频 provider 请求预览未启用：backend={}；请使用 video sidecar，或用 embedded-media-providers feature 构建",
        context.cfg.backend
    ))
}
