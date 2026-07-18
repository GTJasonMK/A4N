use super::super::contracts::{
    ASSEMBLY_INSERT_VIDEO_TYPE, ASSEMBLY_ORDER_STEP, AssemblyInsertMetadata, CHAPTER_VIDEO_TYPE,
};
use a4n_core::models::ai::generated_video;
use serde_json::Value;

pub(in crate::actions::video_records::assembly) fn assembly_clip_display_shot_number(
    video: &generated_video::Model,
) -> Result<i32, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => assembly_clip_shot_number(video),
        ASSEMBLY_INSERT_VIDEO_TYPE => Ok(parse_assembly_insert_metadata(video)?
            .insert_after_shot_number
            .unwrap_or(0)),
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

pub(in crate::actions::video_records) fn assembly_clip_sort_order(
    video: &generated_video::Model,
) -> Result<f64, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => {
            Ok(f64::from(assembly_clip_shot_number(video)?) * ASSEMBLY_ORDER_STEP)
        }
        ASSEMBLY_INSERT_VIDEO_TYPE => Ok(parse_assembly_insert_metadata(video)?.sort_order),
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

pub(in crate::actions::video_records) fn assembly_clip_shot_number(
    video: &generated_video::Model,
) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            format!(
                "视频 #{} 缺少有效 shot_number，无法确定成片拼接顺序",
                video.id
            )
        })
}

pub(in crate::actions::video_records::assembly) fn assembly_clip_insert_after_video_id(
    video: &generated_video::Model,
) -> Result<Option<i64>, String> {
    match video.video_type.as_str() {
        CHAPTER_VIDEO_TYPE => Ok(None),
        ASSEMBLY_INSERT_VIDEO_TYPE => {
            Ok(parse_assembly_insert_metadata(video)?.insert_after_video_id)
        }
        _ => Err(format!("视频 #{} 不是成片片段", video.id)),
    }
}

fn parse_assembly_insert_metadata(
    video: &generated_video::Model,
) -> Result<AssemblyInsertMetadata, String> {
    let metadata = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("assembly_insert"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("插入片段 #{} 缺少 assembly_insert metadata", video.id))?;
    let sort_order = metadata
        .get("sort_order")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            format!(
                "插入片段 #{} 缺少有效 sort_order，无法确定成片拼接顺序",
                video.id
            )
        })?;
    let insert_after_video_id = optional_positive_i64(metadata.get("insert_after_video_id"))
        .map_err(|err| {
            format!(
                "插入片段 #{} 的 assembly_insert.insert_after_video_id {err}",
                video.id
            )
        })?;
    let insert_after_shot_number = optional_positive_i32(metadata.get("insert_after_shot_number"))
        .map_err(|err| {
            format!(
                "插入片段 #{} 的 assembly_insert.insert_after_shot_number {err}",
                video.id
            )
        })?;
    Ok(AssemblyInsertMetadata {
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    })
}

pub(in crate::actions::video_records::assembly) fn optional_positive_i64(
    value: Option<&Value>,
) -> Result<Option<i64>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|id| *id > 0)
            .map(Some)
            .ok_or_else(|| "必须为空或正整数".to_string()),
    }
}

fn optional_positive_i32(value: Option<&Value>) -> Result<Option<i32>, String> {
    match optional_positive_i64(value)? {
        Some(value) => i32::try_from(value)
            .map(Some)
            .map_err(|_| "超过可记录范围".to_string()),
        None => Ok(None),
    }
}
