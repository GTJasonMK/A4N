use super::super::contracts::{AssemblyCacheManifest, AssemblyClip};
use crate::actions::video_generation::media::{
    append_secondary_error, ensure_ffmpeg, remove_partial_video_file,
};
use std::path::Path;

pub(in crate::actions::video_records) fn run_ffmpeg_assemble_videos(
    clips: &[AssemblyClip],
    output_path: &Path,
    width: i32,
    height: i32,
    fps: i32,
    manifest: &AssemblyCacheManifest,
) -> Result<(), String> {
    ensure_ffmpeg().map_err(|err| format!("{err}，无法拼接成片"))?;
    if clips.len() != manifest.clips.len() {
        return Err("成片缓存清单与片段数量不一致，无法拼接成片".to_string());
    }
    let media_clips = clips
        .iter()
        .zip(&manifest.clips)
        .map(|(clip, cached)| a4n_media::VideoAssemblyClip {
            path: &clip.path,
            duration_secs: clip.duration_secs,
            has_audio: cached.has_audio,
        })
        .collect::<Vec<_>>();
    a4n_media::assemble_videos(a4n_media::VideoAssemblyRequest {
        clips: &media_clips,
        output_path,
        width,
        height,
        fps,
        audio_enabled: manifest.audio_enabled,
    })
    .map_err(|error| match error {
        a4n_media::VideoAssemblyError::EmptyClips => "没有可拼接的成片片段".to_string(),
        a4n_media::VideoAssemblyError::InvalidDimensions { width, height } => {
            format!("成片尺寸无效：{width}x{height}")
        }
        a4n_media::VideoAssemblyError::InvalidFps { fps } => {
            format!("成片帧率无效：{fps}")
        }
        a4n_media::VideoAssemblyError::Unavailable { source } => {
            format!("拼接成片失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::VideoAssemblyError::Failed { stderr } => {
            let cleanup_error = remove_partial_video_file(output_path, "成片预览文件").err();
            append_secondary_error(format!("拼接成片失败：{stderr}"), cleanup_error)
        }
    })
}
