use super::super::*;
#[cfg(feature = "embedded-media-providers")]
use super::contracts::ReferenceMediaProviderRequestPreviewContext;

#[cfg(feature = "embedded-media-providers")]
pub(in crate::actions::video_generation) fn reference_media_provider_request_preview(
    preview_id: &str,
    preview_title: &str,
    source_label: &str,
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<ModelRequestPreview, String> {
    let (request_title, request_kind, note, request) =
        reference_media_provider_preview_payload(source_label, context)?;
    Ok(model_request_preview(
        preview_id,
        preview_title,
        vec![provider_request_step(
            "reference_media_video",
            request_title,
            request_kind,
            Some("reference_media_video".to_string()),
            "ready",
            Some(note),
            &request,
        )?],
    ))
}

#[cfg(feature = "embedded-media-providers")]
pub(super) fn reference_media_provider_request_snapshot(
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<Value, String> {
    let (request_title, request_kind, note, request) =
        reference_media_provider_preview_payload("生成时请求快照", context)?;
    Ok(provider_request_snapshot_value(
        "reference_media_video",
        request_title,
        request_kind,
        Some("reference_media_video"),
        Some(note),
        request,
    ))
}

#[cfg(feature = "embedded-media-providers")]
fn reference_media_provider_preview_payload(
    source_label: &str,
    context: &ReferenceMediaProviderRequestPreviewContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    let request = Seedance2VideoProvider::request_body_preview(&context.request)
        .map_err(|e| e.to_string())?;
    let reference_media = context
        .request
        .reference_media
        .as_ref()
        .ok_or_else(|| "参考媒体请求缺少 reference_media".to_string())?;
    let config_note = match (context.config_id, context.provider_backend.as_deref()) {
        (Some(config_id), Some(backend)) => format!("config_id={config_id}，backend={backend}；"),
        (Some(config_id), None) => format!("config_id={config_id}；"),
        (None, Some(backend)) => format!("backend={backend}；"),
        (None, None) => String::new(),
    };
    let mut note = format!(
        "{source_label}；{config_note}Seedance 2.0 参考媒体模式使用 functionMode=omni_reference；image_files={}，video_files={}，audio_files={}；prompt 必须用 @image_file_N / @video_file_N / @audio_file_N 明确绑定每个媒体用途。",
        reference_media.image_files.len(),
        reference_media.video_files.len(),
        reference_media.audio_files.len()
    );
    if let Some(input_note) = context.provider_input_note.as_deref() {
        note.push('；');
        note.push_str(input_note);
    }
    Ok((
        "Seedance 2.0 /task/submit 请求",
        "video_generation_seedance2_omni_reference",
        note,
        request,
    ))
}
