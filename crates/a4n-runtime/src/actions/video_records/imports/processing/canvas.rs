use super::super::prelude::{
    AppState, AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip, Path, PathBuf, ProjectPaths,
    Utc,
};

pub(in crate::actions::video_records::imports) fn normalize_canvas_frame_kind(
    value: &str,
) -> Result<String, String> {
    let value = value.trim();
    match value {
        "" | "first" | "first_frame" => Ok("first".to_string()),
        "tail" | "last" | "last_frame" | "end" => Ok("tail".to_string()),
        "time" | "key" | "key_frame" | "at_secs" => Ok("time".to_string()),
        other => Err(format!("抽帧类型无效: {other}")),
    }
}

pub(in crate::actions::video_records::imports) fn canvas_frame_timestamp(
    video: &a4n_core::models::ai::generated_video::Model,
    frame_kind: &str,
    at_secs: Option<f32>,
) -> Result<f32, String> {
    let duration = video.duration_secs.max(0.0);
    let timestamp = match frame_kind {
        "first" => 0.0,
        "tail" => {
            if duration > 0.12 {
                duration - 0.08
            } else {
                0.0
            }
        }
        "time" => at_secs.ok_or_else(|| "按时间抽帧必须提供 at_secs".to_string())?,
        _ => return Err(format!("抽帧类型无效: {frame_kind}")),
    };
    if !timestamp.is_finite() || timestamp < 0.0 {
        return Err("抽帧时间必须是非负数".to_string());
    }
    if duration > 0.0 && timestamp > duration + 0.5 {
        return Err("抽帧时间超过视频时长".to_string());
    }
    Ok(timestamp)
}

pub(in crate::actions::video_records::imports) fn canvas_frame_output_path(
    state: &AppState,
    project_id: i64,
    video_id: i64,
    frame_kind: &str,
) -> Result<PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).images_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建画布抽帧图片目录：{err}"))?;
    let stamp = Utc::now().timestamp_millis();
    Ok(dir.join(format!(
        "canvas_frame_video_{video_id}_{frame_kind}_{stamp}.jpg"
    )))
}

pub(in crate::actions::video_records::imports) fn run_ffmpeg_extract_frame(
    video_path: &Path,
    output: &Path,
    at_secs: f32,
) -> Result<(), String> {
    a4n_media::extract_video_frame(video_path, output, at_secs).map_err(|error| match error {
        a4n_media::FfmpegCommandError::Unavailable { source } => {
            format!("画布抽帧失败：未能启动 FFmpeg：{source}")
        }
        a4n_media::FfmpegCommandError::Failed { stderr } => {
            format!("画布抽帧失败：{stderr}")
        }
    })
}

pub(in crate::actions::video_records::imports) fn canvas_assembly_manifest(
    project_id: i64,
    clips: &[AssemblyClip],
    width: i32,
    height: i32,
    fps: i32,
) -> Result<AssemblyCacheManifest, String> {
    let clips = clips
        .iter()
        .map(|clip| {
            let metadata = std::fs::metadata(&clip.path).map_err(|error| {
                format!(
                    "读取画布成片输入视频元数据失败（{}）：{error}",
                    clip.path.display()
                )
            })?;
            let modified = metadata.modified().map_err(|error| {
                format!(
                    "读取画布成片输入视频修改时间失败（{}）：{error}",
                    clip.path.display()
                )
            })?;
            let modified_unix_millis = modified
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| {
                    format!(
                        "画布成片输入视频修改时间早于 UNIX_EPOCH（{}）：{error}",
                        clip.path.display()
                    )
                })?
                .as_millis() as u64;
            Ok(AssemblyCacheClip {
                video_id: clip.video_id,
                video_type: clip.video_type.clone(),
                shot_number: clip.shot_number,
                sort_order: clip.sort_order,
                insert_after_video_id: clip.insert_after_video_id,
                file_path: clip.file_path.clone(),
                file_size: metadata.len(),
                modified_unix_millis,
                has_audio: false,
                duration_secs: clip.duration_secs,
                width: clip.width,
                height: clip.height,
                fps: clip.fps,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(AssemblyCacheManifest {
        version: 1,
        project_id,
        chapter_id: 0,
        workflow_mode: "canvas_assembly".to_string(),
        width,
        height,
        fps,
        audio_enabled: false,
        clips,
    })
}
