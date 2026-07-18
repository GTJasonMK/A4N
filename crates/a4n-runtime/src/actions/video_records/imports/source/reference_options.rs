use super::super::prelude::reference_media_dimensions_for_aspect_ratio;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::actions::video_records) struct ReferenceMediaImportOptions {
    pub(in crate::actions::video_records) aspect_ratio: String,
}

pub(in crate::actions::video_records) fn validate_reference_media_import_options(
    duration_secs: i32,
    aspect_ratio: &str,
) -> Result<ReferenceMediaImportOptions, String> {
    if !(1..=60).contains(&duration_secs) {
        return Err("参考媒体视频时长必须在 1 到 60 秒之间".to_string());
    }
    let aspect_ratio = aspect_ratio.trim();
    reference_media_dimensions_for_aspect_ratio(aspect_ratio)?;
    Ok(ReferenceMediaImportOptions {
        aspect_ratio: aspect_ratio.to_string(),
    })
}
