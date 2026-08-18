//! 成片片段（chapter_video / assembly_insert）metadata 与排序纯规则。

use a4n_core::models::ai::generated_video;
use serde_json::{Map, Value, json};

/// 成片排序步长（章节镜头 sort_order = shot_number * step）。
const ASSEMBLY_ORDER_STEP: f64 = 1000.0;

/// 从 generated_video.metadata_json.assembly_insert 解析出的插入片段纯字段。
#[derive(Debug, Clone, PartialEq)]
struct AssemblyInsertMetadata {
    sort_order: f64,
    insert_after_video_id: Option<i64>,
    insert_after_shot_number: Option<i32>,
}

/// 计算插入位置时只需要的锚点字段（无路径/IO）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssemblyClipOrderAnchor {
    pub video_id: i64,
    pub sort_order: f64,
    pub shot_number: i32,
}

/// 是否为成片可拼接片段类型。
pub fn is_assembly_clip_video_type(video_type: &str) -> bool {
    matches!(
        video_type,
        generated_video::VIDEO_TYPE_CHAPTER
            | generated_video::VIDEO_TYPE_REFERENCE_MEDIA
            | generated_video::VIDEO_TYPE_ASSEMBLY_INSERT
    )
}

pub fn assembly_media_properties<I>(
    clips: I,
) -> Result<(i32, i32, i32, f32), String>
where
    I: IntoIterator<Item = (i64, f32, i32, i32, i32)>,
{
    let mut clips = clips.into_iter();
    let (first_id, first_duration, first_width, first_height, first_fps) = clips
        .next()
        .ok_or_else(|| "成片至少需要一个视频片段".to_string())?;
    validate_assembly_media_properties(
        first_id,
        first_duration,
        first_width,
        first_height,
        first_fps,
    )?;
    let width = even_dimension(first_id, first_width, "宽度")?;
    let height = even_dimension(first_id, first_height, "高度")?;
    let mut duration_secs = first_duration;
    for (video_id, duration, clip_width, clip_height, clip_fps) in clips {
        validate_assembly_media_properties(
            video_id,
            duration,
            clip_width,
            clip_height,
            clip_fps,
        )?;
        duration_secs += duration;
        if !duration_secs.is_finite() {
            return Err("成片总时长超过可记录范围".to_string());
        }
    }
    Ok((width, height, first_fps, duration_secs))
}

fn validate_assembly_media_properties(
    video_id: i64,
    duration_secs: f32,
    width: i32,
    height: i32,
    fps: i32,
) -> Result<(), String> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return Err(format!("成片片段视频 #{video_id} 时长无效：{duration_secs}"));
    }
    if width <= 0 || height <= 0 {
        return Err(format!("成片片段视频 #{video_id} 尺寸无效：{width}x{height}"));
    }
    if fps <= 0 {
        return Err(format!("成片片段视频 #{video_id} 帧率无效：{fps}"));
    }
    Ok(())
}

fn even_dimension(video_id: i64, value: i32, label: &str) -> Result<i32, String> {
    if value % 2 == 0 {
        Ok(value)
    } else {
        value.checked_add(1).ok_or_else(|| {
            format!("成片片段视频 #{video_id} {label}超过可编码范围：{value}")
        })
    }
}

/// 解析可选正整数（`None`/`null` 允许，0 与非整数拒绝）。
fn optional_positive_i64(value: Option<&Value>) -> Result<Option<i64>, String> {
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

/// 从章节镜头 metadata 读取 shot_number（须为正整数）。
fn assembly_clip_shot_number(video_id: i64, metadata_json: Option<&Value>) -> Result<i32, String> {
    metadata_json
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{video_id} 缺少有效 shot_number，无法确定成片拼接顺序"))
}

/// 从 assembly_insert 对象解析插入片段排序字段。
fn parse_assembly_insert_metadata(
    video_id: i64,
    metadata_json: Option<&Value>,
) -> Result<AssemblyInsertMetadata, String> {
    let metadata = metadata_json
        .and_then(|metadata| metadata.get("assembly_insert"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("插入片段 #{video_id} 缺少 assembly_insert metadata"))?;
    parse_assembly_insert_object(video_id, metadata)
}

fn parse_assembly_insert_object(
    video_id: i64,
    metadata: &Map<String, Value>,
) -> Result<AssemblyInsertMetadata, String> {
    let sort_order = metadata
        .get("sort_order")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("插入片段 #{video_id} 缺少有效 sort_order，无法确定成片拼接顺序"))?;
    let insert_after_video_id = optional_positive_i64(metadata.get("insert_after_video_id"))
        .map_err(|err| {
            format!("插入片段 #{video_id} 的 assembly_insert.insert_after_video_id {err}")
        })?;
    let insert_after_shot_number = optional_positive_i32(metadata.get("insert_after_shot_number"))
        .map_err(|err| {
            format!("插入片段 #{video_id} 的 assembly_insert.insert_after_shot_number {err}")
        })?;
    Ok(AssemblyInsertMetadata {
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    })
}

/// 成片列表展示用 shot_number：章节镜头读 metadata；插入片段用 insert_after 或 0。
pub fn assembly_clip_display_shot_number(
    video_id: i64,
    video_type: &str,
    metadata_json: Option<&Value>,
) -> Result<i32, String> {
    match video_type {
        generated_video::VIDEO_TYPE_CHAPTER | generated_video::VIDEO_TYPE_REFERENCE_MEDIA => {
            assembly_clip_shot_number(video_id, metadata_json)
        }
        generated_video::VIDEO_TYPE_ASSEMBLY_INSERT => Ok(parse_assembly_insert_metadata(video_id, metadata_json)?
            .insert_after_shot_number
            .unwrap_or(0)),
        _ => Err(format!("视频 #{video_id} 不是成片片段")),
    }
}

/// 成片拼接排序键。
pub fn assembly_clip_sort_order(
    video_id: i64,
    video_type: &str,
    metadata_json: Option<&Value>,
) -> Result<f64, String> {
    match video_type {
        generated_video::VIDEO_TYPE_CHAPTER | generated_video::VIDEO_TYPE_REFERENCE_MEDIA => Ok(f64::from(
            assembly_clip_shot_number(video_id, metadata_json)?,
        ) * ASSEMBLY_ORDER_STEP),
        generated_video::VIDEO_TYPE_ASSEMBLY_INSERT => {
            Ok(parse_assembly_insert_metadata(video_id, metadata_json)?.sort_order)
        }
        _ => Err(format!("视频 #{video_id} 不是成片片段")),
    }
}

/// 插入片段锚定的前序 video_id；章节镜头恒为 None。
pub fn assembly_clip_insert_after_video_id(
    video_id: i64,
    video_type: &str,
    metadata_json: Option<&Value>,
) -> Result<Option<i64>, String> {
    match video_type {
        generated_video::VIDEO_TYPE_CHAPTER | generated_video::VIDEO_TYPE_REFERENCE_MEDIA => Ok(None),
        generated_video::VIDEO_TYPE_ASSEMBLY_INSERT => {
            Ok(parse_assembly_insert_metadata(video_id, metadata_json)?.insert_after_video_id)
        }
        _ => Err(format!("视频 #{video_id} 不是成片片段")),
    }
}

/// 写入/更新 metadata 中的 assembly_insert 位置字段与展示用 shot_number。
pub fn assembly_insert_metadata_with_position(
    base_metadata: Option<Value>,
    sort_order: f64,
    insert_after_video_id: Option<i64>,
    insert_after_shot_number: Option<i32>,
) -> Result<Value, String> {
    let mut metadata = match base_metadata {
        Some(Value::Object(map)) => Value::Object(map),
        Some(_) => return Err("调整插入片段位置时 metadata 必须是对象".to_string()),
        None => json!({}),
    };
    let map = metadata
        .as_object_mut()
        .ok_or_else(|| "调整插入片段位置时 metadata 必须是对象".to_string())?;
    map.insert(
        "shot_number".to_string(),
        json!(insert_after_shot_number.unwrap_or(0)),
    );
    let entry = map
        .entry("assembly_insert".to_string())
        .or_insert_with(|| json!({}));
    let insert = entry
        .as_object_mut()
        .ok_or_else(|| "调整插入片段位置时 assembly_insert metadata 必须是对象".to_string())?;
    insert.insert("sort_order".to_string(), json!(sort_order));
    insert.insert(
        "insert_after_video_id".to_string(),
        json!(insert_after_video_id),
    );
    insert.insert(
        "insert_after_shot_number".to_string(),
        json!(insert_after_shot_number),
    );
    Ok(metadata)
}

/// 根据已有片段锚点计算新插入片段的 sort_order 与 insert_after_shot_number。
pub fn next_assembly_insert_sort_order(
    clips: &[AssemblyClipOrderAnchor],
    insert_after_video_id: Option<i64>,
) -> Result<(f64, Option<i32>), String> {
    let Some(after_video_id) = insert_after_video_id else {
        let first_order = clips.first().map(|clip| clip.sort_order);
        return Ok((
            first_order.unwrap_or(ASSEMBLY_ORDER_STEP) - ASSEMBLY_ORDER_STEP / 2.0,
            None,
        ));
    };

    let index = clips
        .iter()
        .position(|clip| clip.video_id == after_video_id)
        .ok_or_else(|| format!("插入位置视频 #{after_video_id} 不在当前成片片段中"))?;
    let anchor = &clips[index];
    let next_order = clips.get(index + 1).map(|clip| clip.sort_order);
    let order = match next_order {
        Some(next) if next > anchor.sort_order => (anchor.sort_order + next) / 2.0,
        _ => anchor.sort_order + ASSEMBLY_ORDER_STEP / 2.0,
    };
    if !order.is_finite() {
        return Err("无法计算插入片段的成片顺序".to_string());
    }
    let anchor_shot_number = (anchor.shot_number > 0).then_some(anchor.shot_number);
    Ok((order, anchor_shot_number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_clip_shot_number_requires_positive_integer_metadata() {
        assert_eq!(
            assembly_clip_shot_number(1, Some(&json!({ "shot_number": 3 }))).expect("valid"),
            3
        );

        for metadata in [
            None,
            Some(json!({})),
            Some(json!({ "shot_number": 0 })),
            Some(json!({ "shot_number": "3" })),
        ] {
            let err = assembly_clip_shot_number(2, metadata.as_ref())
                .expect_err("invalid shot number should fail");
            assert!(err.contains("缺少有效 shot_number"));
        }
    }

    #[test]
    fn assembly_clip_sort_order_supports_insert_clips_between_shots() {
        let first_order =
            assembly_clip_sort_order(
                1,
                generated_video::VIDEO_TYPE_CHAPTER,
                Some(&json!({ "shot_number": 1 })),
            )
                .expect("first");
        let second_order =
            assembly_clip_sort_order(
                2,
                generated_video::VIDEO_TYPE_CHAPTER,
                Some(&json!({ "shot_number": 2 })),
            )
                .expect("second");
        let insert_order = assembly_clip_sort_order(
            3,
            generated_video::VIDEO_TYPE_ASSEMBLY_INSERT,
            Some(&json!({
                "shot_number": 1,
                "assembly_insert": {
                    "sort_order": 1500.0,
                    "insert_after_video_id": 1,
                    "insert_after_shot_number": 1
                }
            })),
        )
        .expect("insert");

        assert!(first_order < insert_order);
        assert!(insert_order < second_order);
        assert_eq!(first_order, ASSEMBLY_ORDER_STEP);
        assert_eq!(second_order, 2.0 * ASSEMBLY_ORDER_STEP);
    }

    #[test]
    fn assembly_clip_sort_order_rejects_malformed_insert_metadata() {
        let err = assembly_clip_sort_order(
            3,
            generated_video::VIDEO_TYPE_ASSEMBLY_INSERT,
            Some(&json!({
                "shot_number": 1,
                "assembly_insert": {
                    "sort_order": 1500.0,
                    "insert_after_video_id": "1",
                    "insert_after_shot_number": 1
                }
            })),
        )
        .expect_err("invalid insert_after_video_id");
        assert!(err.contains("insert_after_video_id"));
        assert!(err.contains("必须为空或正整数"));

        let err = assembly_clip_sort_order(
            4,
            generated_video::VIDEO_TYPE_ASSEMBLY_INSERT,
            Some(&json!({
                "shot_number": 1,
                "assembly_insert": {
                    "sort_order": 1500.0,
                    "insert_after_video_id": 1,
                    "insert_after_shot_number": 0
                }
            })),
        )
        .expect_err("invalid insert_after_shot_number");
        assert!(err.contains("insert_after_shot_number"));
        assert!(err.contains("必须为空或正整数"));
    }

    #[test]
    fn assembly_insert_metadata_with_position_rejects_malformed_base_metadata() {
        let err = assembly_insert_metadata_with_position(Some(json!("legacy")), 500.0, None, None)
            .expect_err("non-object metadata should fail");
        assert!(err.contains("metadata 必须是对象"));

        let err = assembly_insert_metadata_with_position(
            Some(json!({ "assembly_insert": "legacy" })),
            500.0,
            Some(1),
            Some(1),
        )
        .expect_err("non-object assembly_insert should fail");
        assert!(err.contains("assembly_insert metadata 必须是对象"));
    }

    #[test]
    fn next_assembly_insert_sort_order_places_clip_after_anchor() {
        let clips = [
            AssemblyClipOrderAnchor {
                video_id: 1,
                sort_order: 1000.0,
                shot_number: 1,
            },
            AssemblyClipOrderAnchor {
                video_id: 2,
                sort_order: 2000.0,
                shot_number: 2,
            },
        ];

        let (order, shot_number) =
            next_assembly_insert_sort_order(&clips, Some(1)).expect("anchor exists");

        assert!(order > clips[0].sort_order);
        assert!(order < clips[1].sort_order);
        assert_eq!(shot_number, Some(1));
    }

    #[test]
    fn next_assembly_insert_sort_order_places_clip_before_first_without_anchor() {
        let clips = [
            AssemblyClipOrderAnchor {
                video_id: 1,
                sort_order: 1000.0,
                shot_number: 1,
            },
            AssemblyClipOrderAnchor {
                video_id: 2,
                sort_order: 2000.0,
                shot_number: 2,
            },
        ];

        let (order, shot_number) =
            next_assembly_insert_sort_order(&clips, None).expect("start insert");

        assert!(order < clips[0].sort_order);
        assert_eq!(shot_number, None);
    }

    #[test]
    fn next_assembly_insert_sort_order_rejects_missing_anchor() {
        let clips = [AssemblyClipOrderAnchor {
            video_id: 1,
            sort_order: 1000.0,
            shot_number: 1,
        }];

        let err = next_assembly_insert_sort_order(&clips, Some(999)).expect_err("missing anchor");
        assert!(err.contains("不在当前成片片段中"));
    }

    #[test]
    fn is_assembly_clip_video_type_accepts_known_types() {
        assert!(is_assembly_clip_video_type(
            generated_video::VIDEO_TYPE_CHAPTER
        ));
        assert!(is_assembly_clip_video_type(
            generated_video::VIDEO_TYPE_ASSEMBLY_INSERT
        ));
        assert!(!is_assembly_clip_video_type(
            generated_video::VIDEO_TYPE_ASSEMBLY_PUSH
        ));
    }

    #[test]
    fn assembly_media_properties_are_strict_and_normalized_once() {
        assert_eq!(
            assembly_media_properties([
                (1, 1.5, 1281, 721, 25),
                (2, 2.0, 640, 360, 30),
            ])
            .unwrap(),
            (1282, 722, 25, 3.5)
        );
        assert!(
            assembly_media_properties(std::iter::empty::<(i64, f32, i32, i32, i32)>()).is_err()
        );
        assert!(assembly_media_properties([(1, 0.0, 1280, 720, 24)]).is_err());
    }
}
