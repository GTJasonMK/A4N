use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use a4n_media::{
    VideoAssemblyClip, VideoAssemblyRequest, VideoPostprocessOperation, VideoPostprocessRequest,
    assemble_videos, check_ffmpeg_available, extract_video_frame, extract_video_tail_frame,
    generate_video_thumbnail, postprocess_video, probe_video_has_audio, probe_video_properties,
    trim_video_segment,
};

#[test]
fn ffmpeg_basic_video_transforms_create_non_empty_outputs_when_available() {
    if let Err(error) = check_ffmpeg_available() {
        eprintln!("skipping real FFmpeg smoke test because FFmpeg is unavailable: {error}");
        return;
    }

    let temp_dir = unique_temp_dir();
    std::fs::create_dir_all(&temp_dir).expect("create FFmpeg smoke directory");
    let input = temp_dir.join("input.mp4");
    let silent_input = temp_dir.join("silent-input.mp4");
    let trimmed = temp_dir.join("trimmed.mp4");
    let frame = temp_dir.join("frame.jpg");
    let tail = temp_dir.join("tail.jpg");
    let thumbnail = temp_dir.join("thumbnail.jpg");
    let assembled = temp_dir.join("assembled.mp4");
    let inserted = temp_dir.join("inserted.mp4");
    let deleted = temp_dir.join("deleted.mp4");
    let replaced = temp_dir.join("replaced.mp4");

    create_fixture_video(&input, true);
    create_fixture_video(&silent_input, false);
    trim_video_segment(&input, &trimmed, 0.1, 0.8).expect("trim fixture video");
    extract_video_frame(&input, &frame, 0.2).expect("extract fixture frame");
    extract_video_tail_frame(&input, &tail).expect("extract fixture tail frame");
    generate_video_thumbnail(&input, &thumbnail).expect("generate fixture thumbnail");
    let assembly_clips = [
        VideoAssemblyClip {
            path: &input,
            duration_secs: 1.0,
            has_audio: true,
        },
        VideoAssemblyClip {
            path: &silent_input,
            duration_secs: 1.0,
            has_audio: false,
        },
    ];
    assemble_videos(VideoAssemblyRequest {
        clips: &assembly_clips,
        output_path: &assembled,
        width: 64,
        height: 64,
        fps: 30,
        audio_enabled: true,
    })
    .expect("assemble audio and silent fixture videos");
    assert!(
        probe_video_has_audio(&assembled).expect("probe assembled output audio"),
        "assembled output should preserve or synthesize audio for every clip"
    );
    postprocess_video(postprocess_request(
        &input,
        Some(&silent_input),
        &inserted,
        VideoPostprocessOperation::Insert,
        0.4,
        0.4,
    ))
    .expect("insert fixture video");
    postprocess_video(postprocess_request(
        &input,
        None,
        &deleted,
        VideoPostprocessOperation::Delete,
        0.2,
        0.6,
    ))
    .expect("delete fixture video range");
    postprocess_video(postprocess_request(
        &input,
        Some(&silent_input),
        &replaced,
        VideoPostprocessOperation::Replace,
        0.2,
        0.6,
    ))
    .expect("replace fixture video range");

    let inserted_duration = probe_video_properties(&inserted)
        .expect("probe inserted output")
        .duration_secs;
    let deleted_duration = probe_video_properties(&deleted)
        .expect("probe deleted output")
        .duration_secs;
    let replaced_duration = probe_video_properties(&replaced)
        .expect("probe replaced output")
        .duration_secs;
    assert!(
        (1.7..=2.2).contains(&inserted_duration),
        "inserted output duration should include the source clip: {inserted_duration}"
    );
    assert!(
        (0.3..=0.9).contains(&deleted_duration),
        "deleted output duration should remove the selected range: {deleted_duration}"
    );
    assert!(
        (1.2..=1.8).contains(&replaced_duration),
        "replaced output duration should exchange the selected range: {replaced_duration}"
    );

    for output in [
        &input,
        &silent_input,
        &trimmed,
        &frame,
        &tail,
        &thumbnail,
        &assembled,
        &inserted,
        &deleted,
        &replaced,
    ] {
        let metadata = std::fs::metadata(output)
            .unwrap_or_else(|error| panic!("read {} metadata: {error}", output.display()));
        assert!(
            metadata.is_file() && metadata.len() > 0,
            "{} should be a non-empty file",
            output.display()
        );
    }

    std::fs::remove_dir_all(&temp_dir).expect("remove FFmpeg smoke directory");
}

fn postprocess_request<'a>(
    base_path: &'a Path,
    source_path: Option<&'a Path>,
    output_path: &'a Path,
    operation: VideoPostprocessOperation,
    start_secs: f32,
    end_secs: f32,
) -> VideoPostprocessRequest<'a> {
    VideoPostprocessRequest {
        base_path,
        source_path,
        output_path,
        operation,
        start_secs,
        end_secs,
        base_duration_secs: 1.0,
        width: 64,
        height: 64,
        fps: 30,
    }
}

fn create_fixture_video(path: &Path, with_audio: bool) {
    let mut command = Command::new("ffmpeg");
    command.args([
        "-y",
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=64x64:rate=30",
    ]);
    if with_audio {
        command.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"]);
    }
    command.args(["-t", "1", "-c:v", "mpeg4", "-pix_fmt", "yuv420p"]);
    if with_audio {
        command.args(["-c:a", "aac", "-shortest"]);
    }
    let output = command
        .arg(path)
        .output()
        .expect("start FFmpeg fixture generation");
    assert!(
        output.status.success(),
        "generate FFmpeg fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unique_temp_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "a4n_media_ffmpeg_smoke_{}_{}",
        std::process::id(),
        stamp
    ))
}
