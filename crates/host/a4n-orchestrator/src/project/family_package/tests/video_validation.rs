use super::*;
use crate::project::{ImportMediaPayload, MediaFileEntry, media_file_key};
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::chapter_video_prompt,
};
use std::collections::HashMap;

#[test]
fn validate_video_manifest_accepts_terminal_linked_rows() {
    let package = package_with_video_rows();

    validate_package_manifest(&package).expect("terminal video rows should validate");
}

#[test]
fn validate_video_manifest_rejects_running_prompt() {
    let mut package = package_with_video_rows();
    package.chapter_video_prompts[0].status = chapter_video_prompt::STATUS_RUNNING.to_string();

    let error = validate_package_manifest(&package).expect_err("running prompt must fail");

    assert!(error.contains("运行中的章节视频提示词"), "{error}");
}

#[test]
fn validate_video_manifest_rejects_prompt_runtime_identity() {
    let mut package = package_with_video_rows();
    package.chapter_video_prompts[0].metadata_json = Some(json!({
        "nested": {"workflow_run_id": 7}
    }));

    let error = validate_package_manifest(&package).expect_err("runtime identity must fail");

    assert!(error.contains("metadata_json 不能携带运行身份 workflow_run_id"));
}

#[test]
fn validate_video_manifest_rejects_non_terminal_or_timeline_video() {
    let mut package = package_with_video_rows();
    package.generated_videos[0].status =
        a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED.to_string();
    let error = validate_package_manifest(&package).expect_err("non-terminal video must fail");
    assert!(error.contains("终态生成视频"), "{error}");

    let mut package = package_with_video_rows();
    package.generated_videos[0].timeline_item_id = Some(99);
    let error = validate_package_manifest(&package).expect_err("timeline video must fail");
    assert!(error.contains("timeline_item_id"), "{error}");
}

#[test]
fn validate_video_manifest_rejects_dangling_references() {
    let mut package = package_with_video_rows();
    package.chapter_video_prompts[0].generated_video_id = Some(999);
    let error = validate_package_manifest(&package).expect_err("dangling prompt video must fail");
    assert!(error.contains("不存在的生成视频"), "{error}");

    let mut package = package_with_video_rows();
    package.generated_videos[0].source_image_id = Some(999);
    let error = validate_package_manifest(&package).expect_err("dangling source image must fail");
    assert!(error.contains("不存在的源图片"), "{error}");

    let mut package = package_with_video_rows();
    package.generated_videos[0].source_video_id = Some(999);
    let error = validate_package_manifest(&package).expect_err("dangling source video must fail");
    assert!(error.contains("不存在的源视频"), "{error}");

    let mut package = package_with_video_rows();
    package.chapter_video_prompts[0].metadata_json = Some(json!({
        "video_execution_identity": {
            "segment_plan_prompt_id": 999
        }
    }));
    let error = validate_package_manifest(&package).expect_err("dangling plan prompt must fail");
    assert!(error.contains("不存在的章节视频提示词"), "{error}");

    let mut package = package_with_video_rows();
    package.chapter_video_prompts[0].metadata_json = Some(json!({
        "video_execution_identity": {
            "shooting_script_identity": {
                "record_id": 999,
                "revision": 1,
                "source_revision_key": "revision:1",
                "source_content_sha256": "content",
                "script_sha256": "script"
            }
        }
    }));
    let error = validate_package_manifest(&package).expect_err("dangling script must fail");
    assert!(error.contains("不存在的拍摄剧本"), "{error}");
}

#[test]
fn validate_video_media_manifest_accepts_generated_video_files() {
    let mut package = package_with_video_rows();
    package.generated_videos[0].thumbnail_path = Some("videos/51.jpg".to_string());

    let owners = [
        ("generated_image", 31, "file_path", "images/31.png"),
        ("generated_video", 51, "file_path", "videos/51.mp4"),
        ("generated_video", 51, "thumbnail_path", "videos/51.jpg"),
    ];
    let mut entries = HashMap::new();
    let mut bytes = HashMap::new();
    for (owner_kind, owner_id, field, original_path) in owners {
        let package_path = family_media_package_path(owner_kind, owner_id, field, original_path)
            .expect("fixture path has extension");
        let entry = MediaFileEntry {
            owner_kind: owner_kind.to_string(),
            owner_id,
            field: field.to_string(),
            original_path: original_path.to_string(),
            package_path: package_path.clone(),
        };
        entries.insert(media_file_key(owner_kind, owner_id, field), entry.clone());
        bytes.insert(package_path, b"abc".to_vec());
        package.media_files.push(entry);
    }

    validate_media_manifest(&package, &ImportMediaPayload { entries, bytes })
        .expect("generated video file and thumbnail payload should validate");
}

pub(in crate::project::family_package::tests) fn package_with_video_rows() -> ProjectFamilyPackage {
    let mut package = package_with_derived_project();
    package.text_core.chapters.push(chapter_record(11, 1, None));
    package.generated_images = vec![generated_image::Model {
        id: 31,
        project_id: 2,
        image_type: "scene".to_string(),
        file_path: "images/31.png".to_string(),
        prompt: "reference".to_string(),
        model: "test".to_string(),
        width: 1024,
        height: 1024,
        file_size: Some(3),
        ai_score: None,
        is_favorite: false,
        chapter_id: Some(11),
        panel_id: None,
        metadata_json: None,
        created_at: dt(),
    }];
    package.chapter_video_prompts = vec![chapter_video_prompt::Model {
        id: 41,
        project_id: 2,
        chapter_id: 11,
        style: Some("cinematic".to_string()),
        prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO.to_string(),
        workflow_mode: chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES.to_string(),
        shot_number: 1,
        title: Some("shot".to_string()),
        scene_summary: Some("scene".to_string()),
        visual_prompt: "prompt".to_string(),
        motion_prompt: Some("motion".to_string()),
        negative_prompt: None,
        duration_secs: 6,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: Some(json!([31])),
        status: chapter_video_prompt::STATUS_COMPLETED.to_string(),
        generated_video_id: Some(51),
        timeline_item_id: None,
        metadata_json: None,
        created_at: dt(),
        updated_at: dt(),
    }];
    package.generated_videos = vec![generated_video::Model {
        id: 51,
        project_id: 2,
        chapter_id: Some(11),
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
        workflow_mode: Some(chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES.to_string()),
        file_path: Some("videos/51.mp4".to_string()),
        thumbnail_path: None,
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "test".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 6.0,
        fps: 24,
        file_size: Some(3),
        source_video_id: None,
        source_image_id: Some(31),
        status: a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string(),
        provider_job_id: None,
        error_message: None,
        timeline_item_id: None,
        metadata_json: Some(json!({"prompt_id": 41})),
        is_favorite: false,
        created_at: dt(),
        updated_at: dt(),
    }];
    package
}
