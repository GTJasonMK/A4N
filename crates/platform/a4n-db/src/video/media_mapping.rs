use a4n_db_models::models::ai::{generated_image, generated_video};
use a4n_domain::media::{
    GeneratedImageRecord, GeneratedImageReferenceRecord, GeneratedVideoRecord,
};

pub(crate) fn generated_image_record_from_model(
    model: generated_image::Model,
) -> GeneratedImageRecord {
    GeneratedImageRecord {
        id: model.id,
        project_id: model.project_id,
        image_type: model.image_type,
        file_path: model.file_path,
        prompt: model.prompt,
        model: model.model,
        width: model.width,
        height: model.height,
        file_size: model.file_size,
        ai_score: model.ai_score,
        is_favorite: model.is_favorite,
        chapter_id: model.chapter_id,
        panel_id: model.panel_id,
        metadata_json: model.metadata_json,
        created_at: model.created_at,
    }
}

pub(crate) fn generated_image_reference_record_from_model(
    model: generated_image::Reference,
) -> GeneratedImageReferenceRecord {
    GeneratedImageReferenceRecord {
        id: model.id,
        image_type: model.image_type,
        metadata_json: model.metadata_json,
    }
}

pub(crate) fn generated_video_record_from_model(
    model: generated_video::Model,
) -> GeneratedVideoRecord {
    GeneratedVideoRecord {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        video_type: model.video_type,
        workflow_mode: model.workflow_mode,
        file_path: model.file_path,
        thumbnail_path: model.thumbnail_path,
        prompt: model.prompt,
        negative_prompt: model.negative_prompt,
        model: model.model,
        width: model.width,
        height: model.height,
        duration_secs: model.duration_secs,
        fps: model.fps,
        file_size: model.file_size,
        source_video_id: model.source_video_id,
        source_image_id: model.source_image_id,
        status: model.status,
        provider_job_id: model.provider_job_id,
        error_message: model.error_message,
        timeline_item_id: model.timeline_item_id,
        metadata_json: model.metadata_json,
        is_favorite: model.is_favorite,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn generated_image_mapping_preserves_every_field() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let record = generated_image_record_from_model(generated_image::Model {
            id: 1,
            project_id: 2,
            image_type: "video_frame".into(),
            file_path: "images/frame.png".into(),
            prompt: "frame prompt".into(),
            model: "image-model".into(),
            width: 1280,
            height: 720,
            file_size: Some(4096),
            ai_score: Some(0.75),
            is_favorite: true,
            chapter_id: Some(3),
            panel_id: Some("panel-1".into()),
            metadata_json: Some(json!({"prompt_id": 9})),
            created_at,
        });

        assert_eq!(
            record,
            GeneratedImageRecord {
                id: 1,
                project_id: 2,
                image_type: "video_frame".into(),
                file_path: "images/frame.png".into(),
                prompt: "frame prompt".into(),
                model: "image-model".into(),
                width: 1280,
                height: 720,
                file_size: Some(4096),
                ai_score: Some(0.75),
                is_favorite: true,
                chapter_id: Some(3),
                panel_id: Some("panel-1".into()),
                metadata_json: Some(json!({"prompt_id": 9})),
                created_at,
            }
        );
    }

    #[test]
    fn generated_video_mapping_preserves_every_field() {
        let created_at = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let updated_at = created_at + chrono::Duration::minutes(1);
        let record = generated_video_record_from_model(generated_video::Model {
            id: 1,
            project_id: 2,
            chapter_id: Some(3),
            video_type: "chapter_video".into(),
            workflow_mode: Some("boundary_frames".into()),
            file_path: Some("videos/clip.mp4".into()),
            thumbnail_path: Some("videos/clip.jpg".into()),
            prompt: "video prompt".into(),
            negative_prompt: Some("text".into()),
            model: "video-model".into(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            file_size: Some(8192),
            source_video_id: Some(4),
            source_image_id: Some(5),
            status: "completed".into(),
            provider_job_id: Some("job-1".into()),
            error_message: None,
            timeline_item_id: Some(6),
            metadata_json: Some(json!({"prompt_id": 9})),
            is_favorite: true,
            created_at,
            updated_at,
        });

        assert_eq!(
            record,
            GeneratedVideoRecord {
                id: 1,
                project_id: 2,
                chapter_id: Some(3),
                video_type: "chapter_video".into(),
                workflow_mode: Some("boundary_frames".into()),
                file_path: Some("videos/clip.mp4".into()),
                thumbnail_path: Some("videos/clip.jpg".into()),
                prompt: "video prompt".into(),
                negative_prompt: Some("text".into()),
                model: "video-model".into(),
                width: 1280,
                height: 720,
                duration_secs: 5.0,
                fps: 24,
                file_size: Some(8192),
                source_video_id: Some(4),
                source_image_id: Some(5),
                status: "completed".into(),
                provider_job_id: Some("job-1".into()),
                error_message: None,
                timeline_item_id: Some(6),
                metadata_json: Some(json!({"prompt_id": 9})),
                is_favorite: true,
                created_at,
                updated_at,
            }
        );
    }
}
