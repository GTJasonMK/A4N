use super::super::contracts::{AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip};
use std::path::Path;
use std::time::UNIX_EPOCH;

pub(in crate::actions::video_records) fn build_assembly_cache_manifest(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    width: i32,
    height: i32,
    fps: i32,
    clips: &[AssemblyClip],
) -> Result<AssemblyCacheManifest, String> {
    build_assembly_cache_manifest_with_audio_probe(
        project_id,
        chapter_id,
        workflow_mode,
        width,
        height,
        fps,
        clips,
        &video_file_has_audio,
    )
}

pub(in crate::actions::video_records) fn build_assembly_cache_manifest_with_audio_probe<F>(
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    width: i32,
    height: i32,
    fps: i32,
    clips: &[AssemblyClip],
    has_audio: &F,
) -> Result<AssemblyCacheManifest, String>
where
    F: Fn(&Path) -> Result<bool, String>,
{
    let clips = clips
        .iter()
        .map(|clip| assembly_cache_clip(clip, has_audio))
        .collect::<Result<Vec<_>, _>>()?;
    let audio_enabled = clips.iter().any(|clip| clip.has_audio);
    Ok(AssemblyCacheManifest {
        version: 2,
        project_id,
        chapter_id,
        workflow_mode: workflow_mode.to_string(),
        width,
        height,
        fps,
        audio_enabled,
        clips,
    })
}

fn assembly_cache_clip<F>(clip: &AssemblyClip, has_audio: &F) -> Result<AssemblyCacheClip, String>
where
    F: Fn(&Path) -> Result<bool, String>,
{
    let metadata = std::fs::metadata(&clip.path)
        .map_err(|e| format!("无法读取成片片段视频 #{} 文件元数据：{e}", clip.video_id))?;
    if !metadata.is_file() {
        return Err(format!("成片片段视频 #{} 不是文件", clip.video_id));
    }
    let modified_unix_millis = file_modified_unix_millis(&metadata)
        .map_err(|e| format!("无法读取成片片段视频 #{} 修改时间：{e}", clip.video_id))?;
    Ok(AssemblyCacheClip {
        video_id: clip.video_id,
        video_type: clip.video_type.clone(),
        shot_number: clip.shot_number,
        sort_order: clip.sort_order,
        insert_after_video_id: clip.insert_after_video_id,
        file_path: clip.file_path.clone(),
        file_size: metadata.len(),
        modified_unix_millis,
        has_audio: has_audio(&clip.path)?,
        duration_secs: clip.duration_secs,
        width: clip.width,
        height: clip.height,
        fps: clip.fps,
    })
}

fn video_file_has_audio(path: &Path) -> Result<bool, String> {
    a4n_media::probe_video_has_audio(path).map_err(|error| match error {
        a4n_media::FfprobeCommandError::Unavailable { .. } => {
            "未检测到 FFprobe，无法判断成片片段是否包含音频".to_string()
        }
        a4n_media::FfprobeCommandError::Failed { stderr } => {
            format!("判断成片片段音频失败：{stderr}")
        }
    })
}

fn file_modified_unix_millis(metadata: &std::fs::Metadata) -> Result<u64, String> {
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("文件修改时间早于 UNIX_EPOCH：{e}"))?;
    u64::try_from(duration.as_millis()).map_err(|_| "文件修改时间超过可记录范围".to_string())
}
