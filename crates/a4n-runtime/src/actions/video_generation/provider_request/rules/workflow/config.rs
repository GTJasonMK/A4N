use crate::AppState;
use a4n_core::models::ai::generated_video;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

pub(in crate::actions::video_generation) fn prompt_source_prompt_type(
    prompt: &chapter_video_prompt::Model,
) -> Option<&str> {
    prompt
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source_prompt_type"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::actions::video_generation) fn required_video_execution_identity(
    prompt: &chapter_video_prompt::Model,
) -> Result<Value, String> {
    let metadata = prompt.metadata_json.as_ref().ok_or_else(|| {
        format!(
            "视频提示词 #{} 缺少 metadata_json，无法验证视频执行身份",
            prompt.id
        )
    })?;
    let identity = metadata
        .get("video_execution_identity")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "视频提示词 #{} 缺少 metadata_json.video_execution_identity，请重新生成视频提示词",
                prompt.id
            )
        })?;
    if identity.get("contract_epoch").and_then(Value::as_u64)
        != Some(a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH as u64)
    {
        return Err(format!(
            "视频提示词 #{} 的 video_execution_identity.contract_epoch 不是当前合同 {}",
            prompt.id,
            a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH
        ));
    }
    for key in [
        "projection_sha256",
        "segment_plan_sha256",
        "unit_plan_sha256",
        "frame_set_sha256",
        "source_content_sha256",
        "style_sha256",
        "aspect_ratio",
        "prompt_output_language",
    ] {
        if identity
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            return Err(format!(
                "视频提示词 #{} 的 video_execution_identity.{key} 缺少有效字符串",
                prompt.id
            ));
        }
    }
    if identity
        .get("duration_secs")
        .and_then(Value::as_i64)
        .is_none_or(|value| value <= 0)
    {
        return Err(format!(
            "视频提示词 #{} 的 video_execution_identity.duration_secs 必须是正整数",
            prompt.id
        ));
    }
    for key in ["shooting_script_identity", "generation_asset_fingerprint"] {
        if !identity.get(key).is_some_and(Value::is_object) {
            return Err(format!(
                "视频提示词 #{} 的 video_execution_identity.{key} 必须是 JSON 对象",
                prompt.id
            ));
        }
    }
    Ok(Value::Object(identity.clone()))
}

pub(in crate::actions::video_generation) async fn provider_request_preview_config_id(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    requested_config_id: Option<i64>,
) -> Result<Option<i64>, String> {
    if requested_config_id.is_some() {
        return Ok(requested_config_id);
    }
    let Some(video_id) = prompt.generated_video_id else {
        return Ok(None);
    };
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| format!("视频提示词 #{} 关联的视频 #{video_id} 不存在", prompt.id))?;
    linked_video_preview_config_id(prompt, &video)
}

pub(in crate::actions::video_generation) fn linked_video_preview_config_id(
    prompt: &chapter_video_prompt::Model,
    video: &generated_video::Model,
) -> Result<Option<i64>, String> {
    if video.project_id != prompt.project_id || video.chapter_id != Some(prompt.chapter_id) {
        return Err(format!(
            "视频提示词 #{} 关联的视频 #{} 不属于同一章节",
            prompt.id, video.id
        ));
    }
    let Some(config_id) = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("config_id"))
    else {
        return Ok(None);
    };
    let config_id = config_id.as_i64().ok_or_else(|| {
        format!(
            "视频提示词 #{} 关联的视频 #{} metadata_json.config_id 必须是正整数",
            prompt.id, video.id
        )
    })?;
    if config_id <= 0 {
        return Err(format!(
            "视频提示词 #{} 关联的视频 #{} metadata_json.config_id 必须是正整数",
            prompt.id, video.id
        ));
    }
    Ok(Some(config_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn prompt_with_metadata(metadata_json: Option<Value>) -> chapter_video_prompt::Model {
        let now = Utc::now().naive_utc();
        chapter_video_prompt::Model {
            id: 7,
            project_id: 1,
            chapter_id: 2,
            style: Some("电影感写实".to_string()),
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "boundary_frames".to_string(),
            shot_number: 1,
            title: Some("镜头一".to_string()),
            scene_summary: Some("场景摘要".to_string()),
            visual_prompt: "完整视频提示词".to_string(),
            motion_prompt: Some("镜头运动".to_string()),
            negative_prompt: None,
            duration_secs: 4,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([])),
            status: "pending".to_string(),
            generated_video_id: None,
            metadata_json,
            created_at: now,
            updated_at: now,
        }
    }

    fn complete_identity() -> Value {
        json!({
            "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
            "shooting_script_identity": {
                "record_id": 1,
                "revision": 2,
                "source_revision_key": "source:2",
                "source_content_sha256": "a".repeat(64),
                "script_sha256": "b".repeat(64)
            },
            "projection_sha256": "c".repeat(64),
            "segment_plan_sha256": "d".repeat(64),
            "unit_plan_sha256": "e".repeat(64),
            "frame_set_sha256": "f".repeat(64),
            "source_content_sha256": "a".repeat(64),
            "style_sha256": "1".repeat(64),
            "duration_secs": 4,
            "aspect_ratio": "16:9",
            "prompt_output_language": "zh-CN",
            "generation_asset_fingerprint": {}
        })
    }

    #[test]
    fn rejects_missing_metadata() {
        let err = required_video_execution_identity(&prompt_with_metadata(None))
            .expect_err("metadata is mandatory");
        assert!(err.contains("缺少 metadata_json"));
    }

    #[test]
    fn rejects_missing_execution_identity() {
        let err = required_video_execution_identity(&prompt_with_metadata(Some(json!({}))))
            .expect_err("identity is mandatory");
        assert!(err.contains("video_execution_identity"));
    }

    #[test]
    fn rejects_old_contract_epoch() {
        let mut identity = complete_identity();
        identity["contract_epoch"] = json!(4);
        let err = required_video_execution_identity(&prompt_with_metadata(Some(json!({
            "video_execution_identity": identity
        }))))
        .expect_err("old epoch must be rejected");
        assert!(err.contains("contract_epoch"));
    }

    #[test]
    fn rejects_missing_frame_set_hash() {
        let mut identity = complete_identity();
        identity
            .as_object_mut()
            .expect("identity object")
            .remove("frame_set_sha256");
        let err = required_video_execution_identity(&prompt_with_metadata(Some(json!({
            "video_execution_identity": identity
        }))))
        .expect_err("final prompt identity requires selected frame set");
        assert!(err.contains("frame_set_sha256"));
    }

    #[test]
    fn accepts_complete_v5_identity() {
        let identity = complete_identity();
        let resolved = required_video_execution_identity(&prompt_with_metadata(Some(json!({
            "video_execution_identity": identity.clone()
        }))))
        .expect("complete identity should be accepted");
        assert_eq!(resolved, identity);
    }
}
