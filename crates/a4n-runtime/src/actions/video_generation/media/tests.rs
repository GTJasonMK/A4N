use super::*;
use crate::file_utils::{canonicalize_data_file_path, data_dir_relative_file_path};
use a4n_orchestrator::video::{
    encode_query_component, image_mime_from_path, is_provider_supported_image_content_type,
    tail_frame_image_belongs_to_video, video_shot_number,
};
use chrono::Utc;
use serde_json::{Value, json};

fn video_with_chapter(chapter_id: Option<i64>) -> generated_video::Model {
    generated_video::Model {
        id: 42,
        project_id: 1,
        chapter_id,
        video_type: "chapter_clip".to_string(),
        workflow_mode: None,
        file_path: None,
        thumbnail_path: None,
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "model".to_string(),
        width: 1,
        height: 1,
        duration_secs: 1.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json: None,
        is_favorite: false,
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    }
}

fn video_with_shot_number(shot_number: Option<Value>) -> generated_video::Model {
    let mut video = video_with_chapter(Some(2));
    video.video_type = "chapter_video".to_string();
    video.workflow_mode = Some("first_frame_chain".to_string());
    video.width = 1280;
    video.height = 720;
    video.duration_secs = 8.0;
    video.metadata_json = shot_number.map(|shot_number| json!({ "shot_number": shot_number }));
    video
}

fn image_for_preview(
    project_id: i64,
    chapter_id: Option<i64>,
    file_path: &str,
    file_size: Option<i64>,
) -> generated_image::Model {
    generated_image::Model {
        id: 7,
        project_id,
        image_type: "illustration".to_string(),
        file_path: file_path.to_string(),
        prompt: "prompt".to_string(),
        model: "model".to_string(),
        width: 1280,
        height: 720,
        file_size,
        ai_score: None,
        is_favorite: false,
        chapter_id,
        panel_id: None,
        metadata_json: None,
        created_at: Utc::now().naive_utc(),
    }
}

#[test]
fn tail_frame_image_binds_to_capture_video_id() {
    let mut image = image_for_preview(1, Some(2), "images/tail.jpg", Some(100));
    image.image_type = "video_tail_frame".to_string();
    image.metadata_json = Some(json!({
        "generation_context": {
            "source": "video_tail_frame_capture",
            "video_id": 42,
        }
    }));

    assert!(tail_frame_image_belongs_to_video(&image, 42));
    assert!(!tail_frame_image_belongs_to_video(&image, 41));

    image.metadata_json = Some(json!({
        "generation_context": {
            "source": "video_boundary_frame_prompt",
            "video_id": 42,
        }
    }));
    assert!(!tail_frame_image_belongs_to_video(&image, 42));
}

#[test]
fn reference_image_ids_accepts_missing_and_integer_array() {
    assert_eq!(reference_image_ids(None), Ok(Vec::new()));
    assert_eq!(
        reference_image_ids(Some(&serde_json::json!([3, 5]))),
        Ok(vec![3, 5])
    );
}

#[test]
fn reference_image_ids_rejects_invalid_or_duplicate_values() {
    for value in [
        serde_json::json!("3"),
        serde_json::json!(["3"]),
        serde_json::json!([0]),
        serde_json::json!([3, 3]),
    ] {
        reference_image_ids(Some(&value))
            .expect_err("invalid reference image ids should be rejected");
    }
}

#[test]
fn reference_image_project_ids_preserve_reference_order_and_deduplicate_projects() {
    let mut first = image_for_preview(20, Some(1), "first.png", Some(10));
    first.id = 11;
    let mut second = image_for_preview(10, Some(1), "second.png", Some(10));
    second.id = 12;
    let mut third_same_project = image_for_preview(20, Some(1), "third.png", Some(10));
    third_same_project.id = 13;
    let images_by_id = HashMap::from([
        (first.id, first),
        (second.id, second),
        (third_same_project.id, third_same_project),
    ]);

    let project_ids =
        reference_image_project_ids_in_reference_order(&[11, 404, 12, 13], &images_by_id);

    assert_eq!(project_ids, vec![20, 10]);
}

#[test]
fn image_mime_from_path_requires_supported_extension() {
    for path in ["image", "image.bmp", "image.gif"] {
        let err = image_mime_from_path(std::path::Path::new(path))
            .expect_err("unsupported image extension should be rejected");

        assert!(err.contains("参考图片"));
    }
}

#[test]
fn image_mime_from_path_accepts_supported_extensions() {
    assert_eq!(
        image_mime_from_path(std::path::Path::new("image.PNG")),
        Ok("image/png")
    );
    assert_eq!(
        image_mime_from_path(std::path::Path::new("image.jpeg")),
        Ok("image/jpeg")
    );
    assert_eq!(
        image_mime_from_path(std::path::Path::new("image.webp")),
        Ok("image/webp")
    );
}

#[test]
fn provider_image_content_type_accepts_provider_supported_images() {
    for content_type in [
        "image/png",
        "image/jpeg",
        "image/jpg",
        "image/webp",
        "image/x-icon",
        "image/vnd.microsoft.icon",
        "image/png; charset=binary",
        " IMAGE/WEBP ",
    ] {
        assert!(
            is_provider_supported_image_content_type(content_type),
            "{content_type} should be accepted"
        );
    }
}

#[test]
fn provider_image_content_type_rejects_non_images() {
    for content_type in [
        "",
        "application/json",
        "application/json; charset=utf-8",
        "text/html",
        "application/octet-stream",
    ] {
        assert!(
            !is_provider_supported_image_content_type(content_type),
            "{content_type} should be rejected"
        );
    }
}

#[test]
fn preview_placeholder_records_invalid_reference_notes_without_blocking_url() {
    let image = image_for_preview(2, Some(9), "bad.bmp", None);

    let notes = reference_image_preview_notes(1, 9, &[1], &image);
    let placeholder = reference_image_preview_placeholder(&image);

    assert!(notes.iter().any(|note| note.contains("不属于当前视频项目")));
    assert!(
        notes
            .iter()
            .any(|note| note.contains("文件类型无法作为视频图片输入"))
    );
    assert!(placeholder.starts_with("https://example.invalid/a4n-media/preview/generated-image-7"));
    assert!(placeholder.contains("validation=deferred"));
}

#[test]
fn preview_placeholder_uses_image_metadata_without_reading_file() {
    let image = image_for_preview(1, Some(9), "still.jpeg", Some(1234));

    let notes = reference_image_preview_notes(1, 9, &[1], &image);
    let placeholder = reference_image_preview_placeholder(&image);

    assert!(notes.is_empty());
    assert!(placeholder.starts_with("https://example.invalid/a4n-media/preview/generated-image-7"));
    assert!(placeholder.contains("1234%20bytes"));
}

#[test]
fn required_video_file_extension_rejects_missing_or_unknown_format() {
    for format in ["", "   ", "avi"] {
        let err = required_video_file_extension(format)
            .expect_err("unsupported video format should be rejected");

        assert!(err.contains("文件格式"));
    }
}

#[test]
fn required_video_file_extension_accepts_supported_formats() {
    assert_eq!(required_video_file_extension("mp4"), Ok("mp4"));
    assert_eq!(required_video_file_extension(" WEBM "), Ok("webm"));
    assert_eq!(required_video_file_extension("mov"), Ok("mov"));
    assert_eq!(required_video_file_extension("m4v"), Ok("m4v"));
}

#[test]
fn generated_video_round_trips_through_the_configured_data_directory() {
    let root = tempfile::tempdir().expect("create video data directory");
    let data_dir = root.path().join("configured-data");
    std::fs::create_dir_all(&data_dir).expect("create configured data directory");

    let written =
        save_video_bytes(&data_dir, 21, 5, 154, b"video", "mp4").expect("write generated video");
    let relative = data_dir_relative_file_path(&data_dir, &written, "视频文件")
        .expect("store data-relative path");
    let resolved = canonicalize_data_file_path(&data_dir, &relative, "视频文件")
        .expect("resolve stored video path");

    assert_eq!(relative, "projects/21/videos/chapter_5/video_154.mp4");
    assert_eq!(resolved, written.canonicalize().unwrap());
    assert_eq!(std::fs::read(resolved).unwrap(), b"video");
}

#[test]
fn canvas_video_persistence_uses_project_video_directory_without_chapter() {
    let root = tempfile::tempdir().expect("create canvas video test directory");
    let data_dir = root.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("create data directory");

    let (relative_path, file_size) = persist_canvas_video_bytes(&data_dir, 7, 42, b"video", "mp4")
        .expect("persist canvas video");

    assert_eq!(relative_path, "projects/7/videos/canvas_video_42.mp4");
    assert_eq!(file_size, 5);
    assert_eq!(
        std::fs::read(data_dir.join(relative_path)).unwrap(),
        b"video"
    );
}

#[test]
fn required_trim_chapter_id_rejects_missing_or_invalid_id() {
    for id in [None, Some(0), Some(-1)] {
        let video = video_with_chapter(id);
        let err = required_trim_chapter_id(&video).expect_err("trim chapter id should be required");

        assert!(err.contains("源视频 #42 缺少 chapter_id"));
    }
}

#[test]
fn required_trim_chapter_id_accepts_positive_id() {
    let video = video_with_chapter(Some(7));

    assert_eq!(required_trim_chapter_id(&video), Ok(7));
}

#[test]
fn chapter_videos_dir_scopes_assets_by_chapter() {
    let dir =
        chapter_videos_dir(Path::new("/tmp/a4n"), 42, 7).expect("chapter video dir should resolve");

    assert_eq!(dir, Path::new("/tmp/a4n/projects/42/videos/chapter_7"));
}

#[test]
fn chapter_tail_frame_images_dir_scopes_assets_by_chapter() {
    let dir = chapter_tail_frame_images_dir(Path::new("/tmp/a4n"), 42, 7)
        .expect("chapter tail frame image dir should resolve");

    assert_eq!(dir, Path::new("/tmp/a4n/projects/42/images/chapter_7"));
}

#[test]
fn chapter_asset_dirs_reject_invalid_chapter_id() {
    let err = chapter_videos_dir(Path::new("/tmp/a4n"), 42, 0)
        .expect_err("invalid chapter id should be rejected");

    assert!(err.contains("有效 chapter_id"));
}

#[test]
fn video_thumbnail_persistence_removes_partial_output_on_generation_error() {
    let root = std::env::temp_dir().join(format!(
        "a4n-thumbnail-persistence-test-partial-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/thumb.jpg");
    std::fs::create_dir_all(output.parent().expect("thumbnail parent"))
        .expect("create thumbnail parent");

    let error = persist_video_thumbnail_path(&data_dir, &output, || {
        std::fs::write(&output, b"partial thumbnail").expect("write partial thumbnail");
        Err("ffmpeg failed".to_string())
    })
    .expect_err("generation failure must remove partial thumbnail");

    assert_eq!(error, "ffmpeg failed");
    assert!(!output.exists());
    std::fs::remove_dir_all(&root).expect("remove thumbnail test root");
}

#[test]
fn video_thumbnail_persistence_removes_output_on_relative_path_error() {
    let root = std::env::temp_dir().join(format!(
        "a4n-thumbnail-persistence-test-relative-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let data_dir = root.join("data");
    let output = root.join("outside/thumb.jpg");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(output.parent().expect("thumbnail parent"))
        .expect("create thumbnail parent");

    let error = persist_video_thumbnail_path(&data_dir, &output, || {
        std::fs::write(&output, b"thumbnail").expect("write thumbnail");
        Ok(())
    })
    .expect_err("outside thumbnail must fail relative path normalization");

    assert!(error.contains("视频缩略图不在应用数据目录内"));
    assert!(!output.exists());
    std::fs::remove_dir_all(&root).expect("remove thumbnail test root");
}

#[test]
fn video_thumbnail_persistence_keeps_successful_output() {
    let root = std::env::temp_dir().join(format!(
        "a4n-thumbnail-persistence-test-success-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/thumb.jpg");
    std::fs::create_dir_all(output.parent().expect("thumbnail parent"))
        .expect("create thumbnail parent");

    let relative_path = persist_video_thumbnail_path(&data_dir, &output, || {
        std::fs::write(&output, b"thumbnail").expect("write thumbnail");
        Ok(())
    })
    .expect("successful thumbnail should be retained");

    assert_eq!(relative_path, "projects/1/thumb.jpg");
    assert!(output.exists());
    std::fs::remove_dir_all(&root).expect("remove thumbnail test root");
}

#[test]
fn media_url_query_component_encodes_path_separators_and_spaces() {
    assert_eq!(
        encode_query_component("projects/1/images/chapter 1/frame.png"),
        "projects%2F1%2Fimages%2Fchapter%201%2Fframe.png"
    );
}

#[test]
fn video_shot_number_requires_explicit_positive_metadata() {
    assert_eq!(
        video_shot_number(&video_with_shot_number(Some(json!(3)))).unwrap(),
        3
    );

    let missing = video_shot_number(&video_with_shot_number(None))
        .expect_err("missing shot number must not fall back to first shot");
    assert!(missing.contains("缺少有效 shot_number"));

    let zero = video_shot_number(&video_with_shot_number(Some(json!(0))))
        .expect_err("zero shot number must be rejected");
    assert!(zero.contains("缺少有效 shot_number"));

    let string_value = video_shot_number(&video_with_shot_number(Some(json!("3"))))
        .expect_err("string shot number must be rejected");
    assert!(string_value.contains("缺少有效 shot_number"));
}
