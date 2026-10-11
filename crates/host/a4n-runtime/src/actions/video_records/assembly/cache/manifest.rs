use super::super::contracts::{AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip};
use crate::AppState;
use a4n_media_contract::MediaRoot;
#[cfg(test)]
use std::path::Path;
use std::time::UNIX_EPOCH;

pub(in crate::actions::video_records) struct AssemblyCacheManifestInput<'a> {
    pub(in crate::actions::video_records) project_id: i64,
    pub(in crate::actions::video_records) chapter_id: i64,
    pub(in crate::actions::video_records) workflow_mode: &'a str,
    pub(in crate::actions::video_records) width: i32,
    pub(in crate::actions::video_records) height: i32,
    pub(in crate::actions::video_records) fps: i32,
    pub(in crate::actions::video_records) clips: &'a [AssemblyClip],
}

impl AssemblyCacheManifestInput<'_> {
    fn into_manifest(
        self,
        audio_enabled: bool,
        clips: Vec<AssemblyCacheClip>,
    ) -> AssemblyCacheManifest {
        AssemblyCacheManifest {
            version: 3,
            project_id: self.project_id,
            chapter_id: self.chapter_id,
            workflow_mode: self.workflow_mode.to_string(),
            width: self.width,
            height: self.height,
            fps: self.fps,
            audio_enabled,
            clips,
        }
    }
}

pub(in crate::actions::video_records) async fn build_assembly_cache_manifest(
    state: &AppState,
    input: AssemblyCacheManifestInput<'_>,
) -> Result<AssemblyCacheManifest, String> {
    let mut cached_clips = Vec::with_capacity(input.clips.len());
    for clip in input.clips {
        let has_audio = state
            .probe_media_has_audio(
                &clip.path,
                MediaRoot::Data,
                &format!("成片片段视频 #{}", clip.video_id),
            )
            .await?;
        cached_clips.push(assembly_cache_clip_with_audio(clip, has_audio)?);
    }
    let audio_enabled = cached_clips.iter().any(|clip| clip.has_audio);
    Ok(input.into_manifest(audio_enabled, cached_clips))
}

#[cfg(test)]
pub(in crate::actions::video_records) fn build_assembly_cache_manifest_with_audio_probe<F>(
    input: AssemblyCacheManifestInput<'_>,
    has_audio: &F,
) -> Result<AssemblyCacheManifest, String>
where
    F: Fn(&Path) -> Result<bool, String>,
{
    let clips = input
        .clips
        .iter()
        .map(|clip| {
            let has_audio = has_audio(&clip.path)?;
            assembly_cache_clip_with_audio(clip, has_audio)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let audio_enabled = clips.iter().any(|clip| clip.has_audio);
    Ok(input.into_manifest(audio_enabled, clips))
}

fn assembly_cache_clip_with_audio(
    clip: &AssemblyClip,
    has_audio: bool,
) -> Result<AssemblyCacheClip, String> {
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
        has_audio,
        duration_secs: clip.duration_secs,
        width: clip.width,
        height: clip.height,
        fps: clip.fps,
    })
}

fn file_modified_unix_millis(metadata: &std::fs::Metadata) -> Result<u64, String> {
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("文件修改时间早于 UNIX_EPOCH：{e}"))?;
    u64::try_from(duration.as_millis()).map_err(|_| "文件修改时间超过可记录范围".to_string())
}
