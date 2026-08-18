use super::{
    AudioProbeError, VideoAssemblyClip, VideoPostprocessError, VideoPostprocessRequest,
    VideoProbeError, build_video_assembly_filter, build_video_postprocess_filter,
    parse_audio_duration_secs, parse_frame_rate, parse_video_probe_properties,
};
use a4n_domain::AssemblyPostprocessOperation;
use std::path::Path;

#[test]
fn parse_frame_rate_accepts_fractional_and_plain_values() {
    assert_eq!(parse_frame_rate("24000/1001"), Some(24));
    assert_eq!(parse_frame_rate("30"), Some(30));
    assert_eq!(parse_frame_rate("0/0"), None);
}

#[test]
fn parse_audio_duration_requires_audio_stream_and_positive_duration() {
    let duration = parse_audio_duration_secs(
        br#"{
            "streams": [{"index": 0}],
            "format": {"duration": "2.500000"}
        }"#,
    )
    .expect("valid audio duration should parse");
    assert!((duration - 2.5).abs() < f32::EPSILON);

    assert!(matches!(
        parse_audio_duration_secs(br#"{"streams": [], "format": {"duration": "2.5"}}"#),
        Err(AudioProbeError::MissingAudioStream)
    ));
    assert!(matches!(
        parse_audio_duration_secs(br#"{"streams": [{"index": 0}], "format": {"duration": "0"}}"#),
        Err(AudioProbeError::InvalidDuration)
    ));
}

#[test]
fn parse_video_probe_properties_accepts_valid_video_stream() {
    let properties = parse_video_probe_properties(
        br#"{
            "streams": [
                {
                    "codec_name": "h264",
                    "profile": "Main",
                    "has_b_frames": 0,
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "30000/1001"
                }
            ],
            "format": {
                "duration": "12.345"
            }
        }"#,
    )
    .expect("valid ffprobe output should parse");

    assert_eq!(properties.width, 1920);
    assert_eq!(properties.height, 1080);
    assert!((properties.duration_secs - 12.345).abs() < f32::EPSILON);
    assert_eq!(properties.fps, 30);
    assert_eq!(properties.codec_name.as_deref(), Some("h264"));
    assert_eq!(properties.profile.as_deref(), Some("Main"));
    assert_eq!(properties.has_b_frames, Some(0));
}

#[test]
fn parse_video_probe_properties_rejects_invalid_json() {
    let error =
        parse_video_probe_properties(b"{not-json").expect_err("invalid ffprobe JSON should fail");

    assert!(matches!(error, VideoProbeError::InvalidJson { .. }));
}

#[test]
fn parse_video_probe_properties_rejects_missing_video_stream() {
    let error = parse_video_probe_properties(
        br#"{
            "streams": [],
            "format": {
                "duration": "8.0"
            }
        }"#,
    )
    .expect_err("missing video stream should fail");

    assert!(matches!(error, VideoProbeError::MissingVideoStream));
}

#[test]
fn parse_video_probe_properties_rejects_missing_duration() {
    let error = parse_video_probe_properties(
        br#"{
            "streams": [
                {
                    "width": 1280,
                    "height": 720,
                    "r_frame_rate": "24"
                }
            ],
            "format": {
                "duration": "0"
            }
        }"#,
    )
    .expect_err("missing positive duration should fail");

    assert!(matches!(error, VideoProbeError::InvalidDuration));
}

#[test]
fn trim_video_segment_rejects_invalid_range_before_starting_ffmpeg() {
    let error = super::trim_video_segment(
        Path::new("missing-input.mp4"),
        Path::new("unused-output.mp4"),
        2.0,
        2.0,
    )
    .expect_err("empty trim range should fail before starting ffmpeg");

    assert!(matches!(error, super::FfmpegTrimError::InvalidTimeRange));
}

#[test]
fn extract_video_frame_rejects_invalid_timestamp_before_starting_ffmpeg() {
    let error = super::extract_video_frame(
        Path::new("missing-input.mp4"),
        Path::new("unused-output.jpg"),
        -1.0,
    )
    .expect_err("negative frame timestamp should fail before starting ffmpeg");

    assert!(matches!(error, super::FfmpegFrameError::InvalidTimestamp));
}

#[test]
fn assembly_filter_normalizes_and_concats_video_streams() {
    let clips = [
        assembly_clip("clip-1.mp4", false),
        assembly_clip("clip-2.mp4", false),
    ];

    let filter = build_video_assembly_filter(&clips, 1280, 720, 24, false).expect("build filter");

    assert!(filter.contains("[0:v:0]scale=1280:720"));
    assert!(filter.contains("[1:v:0]scale=1280:720"));
    assert!(filter.contains("fps=24"));
    assert!(filter.ends_with("[v0][v1]concat=n=2:v=1:a=0[outv]"));
}

#[test]
fn assembly_filter_handles_single_video_only_clip() {
    let clips = [assembly_clip("clip-1.mp4", false)];

    let filter = build_video_assembly_filter(&clips, 1280, 720, 24, false).expect("build filter");

    assert!(filter.contains("[0:v:0]scale=1280:720"));
    assert!(filter.ends_with("[v0]concat=n=1:v=1:a=0[outv]"));
}

#[test]
fn assembly_filter_preserves_audio_and_pads_silent_clips() {
    let clips = [
        assembly_clip("clip-1.mp4", true),
        assembly_clip("clip-2.mp4", false),
    ];

    let filter = build_video_assembly_filter(&clips, 1280, 720, 24, true).expect("build filter");

    assert!(filter.contains("[0:a:0]aresample=async=1:first_pts=0"));
    assert!(filter.contains("anullsrc=channel_layout=stereo:sample_rate=48000"));
    assert!(filter.contains("atrim=duration=8.000"));
    assert!(filter.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"));
}

#[test]
fn assembly_filter_rejects_invalid_clip_duration() {
    let clips = [VideoAssemblyClip {
        path: Path::new("clip.mp4"),
        duration_secs: 0.0,
        has_audio: false,
    }];

    assert!(matches!(
        build_video_assembly_filter(&clips, 1280, 720, 24, true),
        Err(super::VideoAssemblyError::InvalidClipDuration { .. })
    ));
}

fn assembly_clip(path: &'static str, has_audio: bool) -> VideoAssemblyClip<'static> {
    VideoAssemblyClip {
        path: Path::new(path),
        duration_secs: 8.0,
        has_audio,
    }
}

#[test]
fn postprocess_insert_filter_keeps_prefix_source_and_tail() {
    let request = postprocess_request(
        AssemblyPostprocessOperation::Insert,
        Some("source.mp4"),
        2.0,
        2.0,
    );

    let filter = build_video_postprocess_filter(&request).expect("build insert filter");

    assert!(filter.contains("trim=start=0:end=2.000"));
    assert!(filter.contains("[1:v:0]scale=642:360"));
    assert!(filter.contains("fps=24"));
    assert!(filter.contains("trim=start=2.000:end=8.000"));
    assert!(filter.contains("[0:a:0]atrim=start=0.000:end=2.000"));
    assert!(filter.contains("anullsrc=channel_layout=stereo:sample_rate=48000"));
    assert!(filter.ends_with("[v0][a0][vsrc][asrc][vtail][atail]concat=n=3:v=1:a=1[outv][outa]"));
}

#[test]
fn postprocess_delete_filter_joins_prefix_and_tail_without_source() {
    let request = postprocess_request(AssemblyPostprocessOperation::Delete, None, 2.0, 4.0);

    let filter = build_video_postprocess_filter(&request).expect("build delete filter");

    assert!(filter.contains("trim=start=0:end=2.000"));
    assert!(!filter.contains("[1:v:0]"));
    assert!(filter.contains("trim=start=4.000:end=8.000"));
    assert!(filter.ends_with("[v0][a0][vtail][atail]concat=n=2:v=1:a=1[outv][outa]"));
}

#[test]
fn postprocess_replace_filter_requires_and_inserts_source() {
    let missing_source =
        postprocess_request(AssemblyPostprocessOperation::Replace, None, 2.0, 4.0);
    assert!(matches!(
        build_video_postprocess_filter(&missing_source),
        Err(VideoPostprocessError::MissingSource)
    ));

    let request = postprocess_request(
        AssemblyPostprocessOperation::Replace,
        Some("source.mp4"),
        2.0,
        4.0,
    );
    let filter = build_video_postprocess_filter(&request).expect("build replace filter");

    assert!(filter.ends_with("[v0][a0][vsrc][asrc][vtail][atail]concat=n=3:v=1:a=1[outv][outa]"));
}

#[test]
fn postprocess_delete_rejects_an_empty_result_range() {
    let request = postprocess_request(AssemblyPostprocessOperation::Delete, None, 0.0, 8.0);

    assert!(matches!(
        build_video_postprocess_filter(&request),
        Err(VideoPostprocessError::InvalidTimeRange { .. })
    ));
}

#[test]
fn postprocess_filter_rejects_invalid_media_properties() {
    let mut request =
        postprocess_request(AssemblyPostprocessOperation::Delete, None, 2.0, 4.0);
    request.fps = 0;
    assert!(matches!(
        build_video_postprocess_filter(&request),
        Err(VideoPostprocessError::InvalidFps { .. })
    ));

    request.fps = 24;
    request.base_duration_secs = 0.0;
    assert!(matches!(
        build_video_postprocess_filter(&request),
        Err(VideoPostprocessError::InvalidBaseDuration { .. })
    ));
}

#[test]
fn postprocess_silent_inputs_keep_video_only_output() {
    let mut request =
        postprocess_request(AssemblyPostprocessOperation::Delete, None, 2.0, 4.0);
    request.base_has_audio = false;

    let filter = build_video_postprocess_filter(&request).expect("build silent delete filter");

    assert!(!filter.contains("anullsrc"));
    assert!(filter.ends_with("[v0][vtail]concat=n=2:v=1:a=0[outv]"));
}

fn postprocess_request(
    operation: AssemblyPostprocessOperation,
    source_path: Option<&'static str>,
    start_secs: f32,
    end_secs: f32,
) -> VideoPostprocessRequest<'static> {
    VideoPostprocessRequest {
        base_path: Path::new("base.mp4"),
        source_path: source_path.map(Path::new),
        output_path: Path::new("output.mp4"),
        operation,
        start_secs,
        end_secs,
        base_duration_secs: 8.0,
        base_has_audio: true,
        source_duration_secs: source_path.map(|_| 1.0),
        source_has_audio: false,
        width: 641,
        height: 359,
        fps: 24,
    }
}
