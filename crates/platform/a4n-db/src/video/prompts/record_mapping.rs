use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;

pub(crate) fn chapter_video_prompt_record_from_model(
    model: chapter_video_prompt::Model,
) -> ChapterVideoPromptRecord {
    ChapterVideoPromptRecord {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        style: model.style,
        prompt_type: model.prompt_type,
        workflow_mode: model.workflow_mode,
        shot_number: model.shot_number,
        title: model.title,
        scene_summary: model.scene_summary,
        visual_prompt: model.visual_prompt,
        motion_prompt: model.motion_prompt,
        negative_prompt: model.negative_prompt,
        duration_secs: model.duration_secs,
        aspect_ratio: model.aspect_ratio,
        reference_image_ids_json: model.reference_image_ids_json,
        status: model.status,
        generated_video_id: model.generated_video_id,
        timeline_item_id: model.timeline_item_id,
        metadata_json: model.metadata_json,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn chapter_video_prompt_model_from_record(
    record: ChapterVideoPromptRecord,
) -> chapter_video_prompt::Model {
    chapter_video_prompt::Model {
        id: record.id,
        project_id: record.project_id,
        chapter_id: record.chapter_id,
        style: record.style,
        prompt_type: record.prompt_type,
        workflow_mode: record.workflow_mode,
        shot_number: record.shot_number,
        title: record.title,
        scene_summary: record.scene_summary,
        visual_prompt: record.visual_prompt,
        motion_prompt: record.motion_prompt,
        negative_prompt: record.negative_prompt,
        duration_secs: record.duration_secs,
        aspect_ratio: record.aspect_ratio,
        reference_image_ids_json: record.reference_image_ids_json,
        status: record.status,
        generated_video_id: record.generated_video_id,
        timeline_item_id: record.timeline_item_id,
        metadata_json: record.metadata_json,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::*;

    #[test]
    fn chapter_video_prompt_mapping_preserves_every_field() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 24)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = chapter_video_prompt::Model {
            id: 9,
            project_id: 4,
            chapter_id: 5,
            style: Some("cinematic".to_string()),
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "storyboard".to_string(),
            shot_number: 3,
            title: Some("雨夜相遇".to_string()),
            scene_summary: Some("车站灯光下相遇".to_string()),
            visual_prompt: "wide shot".to_string(),
            motion_prompt: Some("slow push in".to_string()),
            negative_prompt: Some("text".to_string()),
            duration_secs: 8,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([12, 13])),
            status: "completed".to_string(),
            generated_video_id: Some(20),
            timeline_item_id: Some(21),
            metadata_json: Some(json!({"seed": 17})),
            created_at,
            updated_at: created_at + chrono::Duration::minutes(1),
        };

        assert_eq!(
            chapter_video_prompt_model_from_record(chapter_video_prompt_record_from_model(
                model.clone()
            )),
            model
        );
    }
}
