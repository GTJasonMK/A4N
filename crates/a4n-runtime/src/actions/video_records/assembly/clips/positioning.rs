use super::super::contracts::{ASSEMBLY_INSERT_VIDEO_TYPE, ASSEMBLY_ORDER_STEP, AssemblyClip};
use super::collection::assembly_clips;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::ai::generated_video;
use serde_json::{Value, json};

pub(in crate::actions::video_records) async fn resolve_assembly_insert_position_metadata(
    state: &AppState,
    video: &generated_video::Model,
    base_metadata: Option<Value>,
    insert_after_video_id: Option<i64>,
) -> Result<Value, String> {
    if video.video_type != ASSEMBLY_INSERT_VIDEO_TYPE {
        return Err(format!(
            "视频 #{} 不是成片插入片段，不能调整插入位置",
            video.id
        ));
    }
    if insert_after_video_id == Some(video.id) {
        return Err("插入片段不能插入到自己之后".to_string());
    }
    let chapter_id = video
        .chapter_id
        .ok_or_else(|| format!("插入片段 #{} 缺少章节绑定", video.id))?;
    let workflow_mode = video
        .workflow_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("插入片段 #{} 缺少 workflow_mode", video.id))?;
    let rows = state
        .orchestrator
        .generated_videos_for_chapter(video.project_id, chapter_id)
        .await
        .map_err(app_error_message)?
        .into_iter()
        .filter(|row| row.id != video.id)
        .collect::<Vec<_>>();
    let clips = assembly_clips(state, rows, video.project_id, chapter_id, workflow_mode)?;
    let (sort_order, insert_after_shot_number) =
        next_assembly_insert_sort_order(&clips, insert_after_video_id)?;
    assembly_insert_metadata_with_position(
        base_metadata,
        sort_order,
        insert_after_video_id,
        insert_after_shot_number,
    )
}

pub(in crate::actions::video_records) fn assembly_insert_metadata_with_position(
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

pub(in crate::actions::video_records) fn next_assembly_insert_sort_order(
    clips: &[AssemblyClip],
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
