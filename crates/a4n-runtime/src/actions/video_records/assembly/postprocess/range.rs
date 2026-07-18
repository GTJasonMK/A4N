use super::super::contracts::{ApplyAssemblyPostprocessEditInput, PostprocessEditSource};
use crate::actions::video_records::imports::probe_video_properties;
use a4n_core::models::ai::generated_video;

pub(in crate::actions::video_records) fn validate_postprocess_operation(
    value: &str,
) -> Result<&'static str, String> {
    match value.trim() {
        "insert" => Ok("insert"),
        "delete" => Ok("delete"),
        "replace" => Ok("replace"),
        "" => Err("请选择后处理操作".to_string()),
        other => Err(format!("后处理操作无效：{other}")),
    }
}

pub(in crate::actions::video_records) fn postprocess_edit_range(
    base: &generated_video::Model,
    input: &ApplyAssemblyPostprocessEditInput,
    operation: &str,
    source: Option<&PostprocessEditSource>,
) -> Result<(f32, f32, Option<f32>), String> {
    let base_duration = base.duration_secs.max(0.1);
    let source_duration = if let Some(source) = source {
        Some(probe_video_properties(&source.path, "后处理来源视频")?.duration_secs)
    } else {
        None
    };
    match operation {
        "insert" => {
            let at = input
                .at_secs
                .ok_or_else(|| "插入操作缺少插入时间".to_string())?;
            if !at.is_finite() || at < 0.0 || at > base_duration {
                return Err("插入时间超出当前片段范围".to_string());
            }
            Ok((at, at, source_duration))
        }
        "delete" | "replace" => {
            let start = input
                .start_secs
                .ok_or_else(|| "删除/替换操作缺少起始时间".to_string())?;
            let end = input
                .end_secs
                .ok_or_else(|| "删除/替换操作缺少结束时间".to_string())?;
            if !start.is_finite()
                || !end.is_finite()
                || start < 0.0
                || end <= start
                || end > base_duration + 0.05
            {
                return Err("删除/替换时间范围无效".to_string());
            }
            Ok((start, end.min(base_duration), source_duration))
        }
        _ => Err(format!("后处理操作无效：{operation}")),
    }
}
