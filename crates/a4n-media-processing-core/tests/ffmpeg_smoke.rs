use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use a4n_domain::AssemblyPostprocessOperation;
use a4n_media_processing_core::{
    VIDEO_SCRUB_FRAMES_PER_SECOND, VideoAssemblyClip, VideoAssemblyRequest,
    VideoPostprocessRequest, assemble_videos, extract_video_frame, extract_video_tail_frame,
    generate_video_scrub_frames, generate_video_scrub_proxy, generate_video_thumbnail,
    postprocess_video, probe_audio_duration_secs, probe_video_has_audio, probe_video_properties,
    trim_video_segment,
};

#[test]
#[ignore = "requires ffmpeg and ffprobe"]
fn ffprobe_reports_real_audio_duration() {
    let temp_dir = unique_temp_dir();
    std::fs::create_dir_all(&temp_dir).expect("create FFprobe audio smoke directory");
    let input = temp_dir.join("input.wav");
    create_fixture_audio(&input);

    let duration = probe_audio_duration_secs(&input).expect("probe fixture audio duration");
    assert!(
        (0.9..=1.1).contains(&duration),
        "fixture audio duration should be about one second: {duration}"
    );

    std::fs::remove_dir_all(&temp_dir).expect("remove FFprobe audio smoke directory");
}

#[test]
#[ignore = "requires ffmpeg and ffprobe"]
fn ffmpeg_basic_video_transforms_create_non_empty_outputs() {
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
    let scrub_proxy = temp_dir.join("scrub-proxy.mp4");
    let preview_proxy = temp_dir.join("preview-proxy.mp4");
    let scrub_frames = temp_dir.join("scrub-frames.mjpg");

    create_fixture_video(&input, true);
    create_fixture_video(&silent_input, false);
    trim_video_segment(&input, &trimmed, 0.1, 0.8).expect("trim fixture video");
    extract_video_frame(&input, &frame, 0.2).expect("extract fixture frame");
    extract_video_tail_frame(&input, &tail).expect("extract fixture tail frame");
    generate_video_thumbnail(&input, &thumbnail).expect("generate fixture thumbnail");
    generate_video_scrub_proxy(&input, &scrub_proxy, false).expect("generate scrub proxy");
    generate_video_scrub_proxy(&input, &preview_proxy, true).expect("generate preview proxy");
    generate_video_scrub_frames(&input, &scrub_frames).expect("generate scrub frames");
    let scrub_frame_bytes = std::fs::read(&scrub_frames).expect("read scrub frames");
    assert!(scrub_frame_bytes.starts_with(&[0xff, 0xd8]));
    assert!(scrub_frame_bytes.ends_with(&[0xff, 0xd9]));
    assert!(
        scrub_frame_bytes
            .windows(2)
            .filter(|pair| *pair == [0xff, 0xd8])
            .count()
            >= VIDEO_SCRUB_FRAMES_PER_SECOND as usize
    );
    assert!(
        !probe_video_has_audio(&scrub_proxy).expect("probe scrub proxy audio"),
        "scrub proxy should omit audio for stable WebKitGTK seeking"
    );
    assert!(
        probe_video_has_audio(&preview_proxy).expect("probe preview proxy audio"),
        "preview proxy should preserve source audio packets"
    );
    let input_properties = probe_video_properties(&input).expect("probe scrub source properties");
    let scrub_properties =
        probe_video_properties(&scrub_proxy).expect("probe scrub proxy properties");
    assert_eq!(
        (scrub_properties.width, scrub_properties.height),
        (input_properties.width, input_properties.height),
        "scrub proxy should preserve source resolution"
    );
    assert_webkit_compatible_codec(&scrub_proxy);
    assert_keyframe_gap(&scrub_proxy, 0.05);
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
        AssemblyPostprocessOperation::Insert,
        0.4,
        0.4,
    ))
    .expect("insert fixture video");
    postprocess_video(postprocess_request(
        &input,
        None,
        &deleted,
        AssemblyPostprocessOperation::Delete,
        0.2,
        0.6,
    ))
    .expect("delete fixture video range");
    postprocess_video(postprocess_request(
        &input,
        Some(&silent_input),
        &replaced,
        AssemblyPostprocessOperation::Replace,
        0.2,
        0.6,
    ))
    .expect("replace fixture video range");

    for output in [&inserted, &deleted, &replaced] {
        assert!(
            probe_video_has_audio(output).expect("probe postprocess output audio"),
            "postprocess output should preserve base audio and pad silent segments"
        );
    }

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
    assert_keyframe_gap(&assembled, 1.1);
    assert_keyframe_gap(&inserted, 1.1);

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
        &scrub_proxy,
        &preview_proxy,
        &scrub_frames,
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

fn assert_keyframe_gap(path: &Path, max_gap_secs: f32) {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-skip_frame",
            "nokey",
            "-show_entries",
            "frame=best_effort_timestamp_time",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("start FFprobe keyframe inspection");
    assert!(
        output.status.success(),
        "inspect {} keyframes: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let keyframes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().trim_end_matches(',').parse::<f32>().ok())
        .collect::<Vec<_>>();
    assert!(
        keyframes.len() >= 2,
        "{} should contain at least two keyframes: {keyframes:?}",
        path.display()
    );
    assert!(
        keyframes
            .windows(2)
            .all(|pair| pair[1] - pair[0] <= max_gap_secs),
        "{} keyframes should be no more than {max_gap_secs}s apart: {keyframes:?}",
        path.display()
    );
}

fn assert_webkit_compatible_codec(path: &Path) {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=profile,has_b_frames",
            "-of",
            "default=nw=1",
        ])
        .arg(path)
        .output()
        .expect("start FFprobe scrub codec inspection");
    assert!(
        output.status.success(),
        "inspect {} scrub codec: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let details = String::from_utf8_lossy(&output.stdout);
    assert!(
        details.lines().any(|line| line.trim() == "profile=Main"),
        "{} should use the H.264 Main profile: {details}",
        path.display()
    );
    assert!(
        details.lines().any(|line| line.trim() == "has_b_frames=0"),
        "{} should not use B frames: {details}",
        path.display()
    );
}

fn postprocess_request<'a>(
    base_path: &'a Path,
    source_path: Option<&'a Path>,
    output_path: &'a Path,
    operation: AssemblyPostprocessOperation,
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
        base_has_audio: true,
        source_duration_secs: source_path.map(|_| 1.0),
        source_has_audio: false,
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

fn create_fixture_audio(path: &Path) {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(path)
        .output()
        .expect("start FFmpeg audio fixture generation");
    assert!(
        output.status.success(),
        "generate FFmpeg audio fixture: {}",
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
