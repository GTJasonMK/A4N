use crate::file_utils::persist_created_media_file;
use a4n_orchestrator::canvas::{
    canvas_audio_extension, required_canvas_audio_file_name, validate_canvas_audio_mime,
};
use std::path::{Path, PathBuf};

fn unique_audio_temp_dir(prefix: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn canvas_audio_file_persistence_removes_partial_write_on_error() {
    let root = unique_audio_temp_dir("a4n_canvas_audio_partial");
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/audio/canvas_audio.mp3");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = persist_created_media_file(
        &data_dir,
        &output,
        |output| {
            std::fs::write(output, b"partial audio").expect("write partial audio");
            Err("audio write interrupted".to_string())
        },
        "画布音频文件",
        "画布音频文件",
        "画布音频文件",
    )
    .expect_err("partial audio must be removed");

    assert_eq!(error, "audio write interrupted");
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn canvas_audio_file_persistence_removes_output_on_relative_path_error() {
    let root = unique_audio_temp_dir("a4n_canvas_audio_relative");
    let data_dir = root.join("data");
    let output = root.join("outside/canvas_audio.mp3");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = persist_created_media_file(
        &data_dir,
        &output,
        |output| {
            std::fs::write(output, b"audio").expect("write audio");
            Ok(())
        },
        "画布音频文件",
        "画布音频文件",
        "画布音频文件",
    )
    .expect_err("outside audio path must fail");

    assert!(error.contains("画布音频文件不在应用数据目录内"));
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn canvas_audio_extension_accepts_supported_formats() {
    assert_eq!(
        canvas_audio_extension(Path::new("voice.mp3")).unwrap(),
        "mp3"
    );
    assert_eq!(
        canvas_audio_extension(Path::new("music.WAV")).unwrap(),
        "wav"
    );
    assert_eq!(
        canvas_audio_extension(Path::new("fx.flac")).unwrap(),
        "flac"
    );
}

#[test]
fn canvas_audio_extension_rejects_unsupported_formats() {
    let err = canvas_audio_extension(Path::new("clip.mp4"))
        .expect_err("video extension must not be accepted as audio");

    assert!(err.contains("格式不支持"));
}

#[test]
fn required_canvas_audio_file_name_rejects_paths() {
    let err = required_canvas_audio_file_name("../voice.mp3")
        .expect_err("upload file name must not contain a path");

    assert!(err.contains("不能包含路径"));
}

#[test]
fn validate_canvas_audio_mime_accepts_audio_and_octet_stream() {
    assert!(validate_canvas_audio_mime("audio/mpeg", "mp3").is_ok());
    assert!(validate_canvas_audio_mime("application/octet-stream", "wav").is_ok());
    assert!(validate_canvas_audio_mime("video/mp4", "m4a").is_ok());
}

#[test]
fn validate_canvas_audio_mime_rejects_non_audio_mime() {
    let err = validate_canvas_audio_mime("video/mp4", "mp3")
        .expect_err("video MIME must not be accepted for mp3");

    assert!(err.contains("MIME"));
}
