use super::super::contracts::{AssemblyCacheManifest, AssemblyClip};
use crate::AppState;
use a4n_media_contract::{MediaAssemblyClip, MediaExecuteInput, MediaExecuteOutput};
use std::path::Path;

pub(in crate::actions::video_records) async fn run_ffmpeg_assemble_videos(
    state: &AppState,
    clips: &[AssemblyClip],
    output_path: &Path,
    width: i32,
    height: i32,
    fps: i32,
    manifest: &AssemblyCacheManifest,
) -> Result<(), String> {
    if clips.len() != manifest.clips.len() {
        return Err("成片缓存清单与片段数量不一致，无法拼接成片".to_string());
    }
    let media_clips = clips
        .iter()
        .zip(&manifest.clips)
        .map(|(clip, cached)| {
            Ok(MediaAssemblyClip {
                input: state
                    .data_media_file_ref(&clip.path, &format!("成片片段视频 #{}", clip.video_id))?,
                duration_secs: clip.duration_secs,
                has_audio: cached.has_audio,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let output = state.data_media_file_ref(output_path, "成片输出视频")?;
    match state
        .execute_media(MediaExecuteInput::AssembleVideos {
            clips: media_clips,
            output,
            width,
            height,
            fps,
            audio_enabled: manifest.audio_enabled,
        })
        .await?
    {
        MediaExecuteOutput::Unit => Ok(()),
        output => Err(format!(
            "拼接成片失败：media worker 返回了不匹配的输出：{output:?}"
        )),
    }
}
