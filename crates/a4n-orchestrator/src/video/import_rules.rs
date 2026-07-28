//! 视频导入/裁剪/抽帧纯规则（无 DB、无 Host 路径 IO）。

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

/// 上传视频文件名（禁止路径分隔）。
pub fn required_upload_video_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("上传视频文件名不能为空".to_string());
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传视频文件名无效".to_string())?;
    if name != trimmed {
        return Err("上传视频文件名不能包含路径".to_string());
    }
    Ok(name.to_string())
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

/// 规范化画布抽帧类型。
pub fn normalize_canvas_frame_kind(value: &str) -> Result<String, String> {
    let value = value.trim();
    match value {
        "" | "first" | "first_frame" => Ok("first".to_string()),
        "tail" | "last" | "last_frame" | "end" => Ok("tail".to_string()),
        "time" | "key" | "key_frame" | "at_secs" => Ok("time".to_string()),
        other => Err(format!("抽帧类型无效: {other}")),
    }
}

/// 按帧类型与视频时长计算抽帧时间点。
pub fn canvas_frame_timestamp(
    duration_secs: f32,
    frame_kind: &str,
    at_secs: Option<f32>,
) -> Result<f32, String> {
    let duration = duration_secs.max(0.0);
    let timestamp = match frame_kind {
        "first" => 0.0,
        "tail" => {
            if duration > 0.12 {
                duration - 0.08
            } else {
                0.0
            }
        }
        "time" => at_secs.ok_or_else(|| "按时间抽帧必须提供 at_secs".to_string())?,
        _ => return Err(format!("抽帧类型无效: {frame_kind}")),
    };
    if !timestamp.is_finite() || timestamp < 0.0 {
        return Err("抽帧时间必须是非负数".to_string());
    }
    if duration > 0.0 && timestamp > duration + 0.5 {
        return Err("抽帧时间超过视频时长".to_string());
    }
    Ok(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn upload_name_mime_and_data_url() {
        assert_eq!(
            required_upload_video_file_name(" clip.mp4 ").unwrap(),
            "clip.mp4"
        );
        assert!(required_upload_video_file_name("../x.mp4").is_err());
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
            Some(&json!({ "source": { "content_sha256": "abc" } })),
        );
        assert_eq!(meta["prompt_id"], json!(9));
        assert_eq!(meta["source"]["content_sha256"], json!("abc"));

        let trim = trimmed_video_metadata(Some(&json!({ "source": { "k": 1 } })), 1.0, 2.0);
        assert_eq!(trim["trim"]["start_secs"], json!(1.0));
        assert_eq!(trim["source"]["k"], json!(1));

        assert_eq!(normalize_canvas_frame_kind("last_frame").unwrap(), "tail");
        assert_eq!(canvas_frame_timestamp(10.0, "first", None).unwrap(), 0.0);
        assert!((canvas_frame_timestamp(10.0, "tail", None).unwrap() - 9.92).abs() < 1e-3);
        assert_eq!(
            canvas_frame_timestamp(10.0, "time", Some(3.0)).unwrap(),
            3.0
        );
        assert!(canvas_frame_timestamp(10.0, "time", None).is_err());
    }
}
