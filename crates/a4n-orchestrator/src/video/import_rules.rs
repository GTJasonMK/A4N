//! 视频导入/裁剪/抽帧纯规则（无 DB、无 Host 路径 IO）。

use a4n_domain::{CanvasFrameKind, VIDEO_TAIL_FRAME_OFFSET_SECS};
use serde_json::{Value, json};
use std::path::Path;

/// 裁剪时间范围必须有限、非负且 end > start。
pub fn validate_video_trim_range(start_secs: f32, end_secs: f32) -> Result<(), String> {
    if !start_secs.is_finite()
        || !end_secs.is_finite()
        || start_secs < 0.0
        || end_secs <= start_secs
    {
        return Err("裁剪时间范围无效".into());
    }
    Ok(())
}

/// 从路径解析并校验导入视频扩展名（mp4/webm/mov/m4v）。
pub fn import_video_extension(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("导入视频缺少文件扩展名：{}", path.display()))?;
    match extension.to_ascii_lowercase().as_str() {
        "mp4" => Ok("mp4"),
        "webm" => Ok("webm"),
        "mov" => Ok("mov"),
        "m4v" => Ok("m4v"),
        _ => Err(format!("导入视频格式不支持：{extension}")),
    }
}

/// MIME 与扩展名匹配校验。
pub fn validate_upload_video_mime(mime_type: &str, extension: &str) -> Result<(), String> {
    let mime = mime_type.trim().to_ascii_lowercase();
    let valid = match extension {
        "mp4" => matches!(mime.as_str(), "video/mp4" | "application/mp4"),
        "webm" => mime == "video/webm",
        "mov" => matches!(mime.as_str(), "video/quicktime" | "video/mov"),
        "m4v" => matches!(
            mime.as_str(),
            "video/x-m4v" | "video/mp4" | "application/mp4"
        ),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("上传视频 MIME 与文件扩展名不匹配".to_string())
    }
}

/// 将源 metadata 中的 source 上下文拷入目标 metadata 对象。
fn insert_source_context(metadata: &mut Value, source_metadata: Option<&Value>) {
    if let Some(source_context) = source_metadata
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        && let Some(object) = metadata.as_object_mut()
    {
        object.insert("source".to_string(), source_context);
    }
}

/// 章节镜头导入 metadata。
pub fn imported_chapter_video_metadata(
    prompt_id: i64,
    shot_number: i32,
    source_prompt_type: &str,
    source_file_name: &str,
    source_extension: &str,
    imported_at: &str,
    source_metadata: Option<&Value>,
) -> Value {
    let mut metadata = json!({
        "prompt_id": prompt_id,
        "shot_number": shot_number,
        "source_prompt_type": source_prompt_type,
        "import": {
            "external": true,
            "source_file_name": source_file_name,
            "source_extension": source_extension,
            "imported_at": imported_at,
        }
    });
    insert_source_context(&mut metadata, source_metadata);
    if let Some(identity) = source_metadata
        .and_then(|source| source.get("video_execution_identity"))
        .cloned()
        && let Some(object) = metadata.as_object_mut()
    {
        object.insert("video_execution_identity".to_string(), identity);
    }
    metadata
}

/// 裁剪结果 metadata。
pub fn trimmed_video_metadata(
    source_metadata: Option<&Value>,
    start_secs: f32,
    end_secs: f32,
) -> Value {
    let mut metadata = json!({
        "trim": {
            "start_secs": start_secs,
            "end_secs": end_secs,
        }
    });
    insert_source_context(&mut metadata, source_metadata);
    metadata
}

/// 按帧类型与视频时长计算抽帧时间点。
pub fn canvas_frame_timestamp(
    duration_secs: f32,
    frame_kind: CanvasFrameKind,
    at_secs: Option<f32>,
) -> Result<f32, String> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err("源视频时长必须是有限正数".to_string());
    }
    match (frame_kind, at_secs) {
        (CanvasFrameKind::First, None) => Ok(0.0),
        (CanvasFrameKind::Tail, None) => Ok(if duration_secs > VIDEO_TAIL_FRAME_OFFSET_SECS {
            duration_secs - VIDEO_TAIL_FRAME_OFFSET_SECS
        } else {
            0.0
        }),
        (CanvasFrameKind::Time, Some(timestamp)) => {
            if !timestamp.is_finite() || timestamp < 0.0 {
                return Err("抽帧时间必须是有限非负数".to_string());
            }
            if timestamp >= duration_secs {
                return Err("抽帧时间必须早于视频结束时间".to_string());
            }
            Ok(timestamp)
        }
        (CanvasFrameKind::Time, None) => Err("按时间抽帧必须提供 at_secs".to_string()),
        (_, Some(_)) => Err("首帧或尾帧抽取不接受 at_secs".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::required_upload_file_name;

    #[test]
    fn trim_range_and_extension() {
        validate_video_trim_range(0.0, 1.5).unwrap();
        assert!(validate_video_trim_range(2.0, 1.0).is_err());
        assert!(validate_video_trim_range(f32::NAN, 1.0).is_err());

        assert_eq!(import_video_extension(Path::new("a.MP4")).unwrap(), "mp4");
        assert!(import_video_extension(Path::new("a.avi")).is_err());
        assert!(import_video_extension(Path::new("noext")).is_err());
    }

    #[test]
    fn upload_name_and_mime() {
        assert_eq!(
            required_upload_file_name(" clip.mp4 ", "上传视频文件名").unwrap(),
            "clip.mp4"
        );
        assert!(required_upload_file_name("../x.mp4", "上传视频文件名").is_err());
        validate_upload_video_mime("video/mp4", "mp4").unwrap();
        assert!(validate_upload_video_mime("image/png", "mp4").is_err());
    }

    #[test]
    fn metadata_and_frame_helpers() {
        let meta = imported_chapter_video_metadata(
            9,
            2,
            "video_prompt",
            "a.mp4",
            "mp4",
            "t0",
            Some(&json!({
                "source": { "content_sha256": "abc" },
                "video_execution_identity": { "segment_number": 2 }
            })),
        );
        assert_eq!(meta["prompt_id"], json!(9));
        assert_eq!(meta["source"]["content_sha256"], json!("abc"));
        assert_eq!(meta["video_execution_identity"]["segment_number"], json!(2));

        let trim = trimmed_video_metadata(Some(&json!({ "source": { "k": 1 } })), 1.0, 2.0);
        assert_eq!(trim["trim"]["start_secs"], json!(1.0));
        assert_eq!(trim["source"]["k"], json!(1));

        assert_eq!(
            canvas_frame_timestamp(10.0, CanvasFrameKind::First, None).unwrap(),
            0.0
        );
        assert!(
            (canvas_frame_timestamp(10.0, CanvasFrameKind::Tail, None).unwrap()
                - (10.0 - VIDEO_TAIL_FRAME_OFFSET_SECS))
                .abs()
                < 1e-3
        );
        assert_eq!(
            canvas_frame_timestamp(10.0, CanvasFrameKind::Time, Some(3.0)).unwrap(),
            3.0
        );
        assert!(canvas_frame_timestamp(10.0, CanvasFrameKind::Time, None).is_err());
        assert!(canvas_frame_timestamp(10.0, CanvasFrameKind::Time, Some(10.0)).is_err());
        assert!(canvas_frame_timestamp(0.0, CanvasFrameKind::First, None).is_err());
        assert!("last_frame".parse::<CanvasFrameKind>().is_err());
    }
}
