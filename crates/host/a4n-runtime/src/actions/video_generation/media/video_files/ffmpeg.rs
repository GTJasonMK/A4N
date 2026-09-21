use crate::AppState;
use a4n_media_contract::MediaExecuteInput;
use std::path::Path;

pub(in crate::actions::video_generation::media) async fn capture_video_tail_frame(
    state: &AppState,
    video_path: &Path,
    output: &Path,
) -> Result<(), String> {
    let input = state.data_media_file_ref(video_path, "尾帧源视频")?;
    let output = state.data_media_file_ref(output, "尾帧输出图片")?;
    match state
        .execute_media(MediaExecuteInput::ExtractVideoTailFrame { input, output })
        .await?
    {
        a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
        output => Err(format!("采集视频尾帧返回了不匹配的输出：{output:?}")),
    }
}

pub(crate) async fn run_ffmpeg_trim(
    state: &AppState,
    input: &Path,
    output: &Path,
    start_secs: f32,
    end_secs: f32,
) -> Result<(), String> {
    let input = state.data_media_file_ref(input, "裁剪源视频")?;
    let output = state.data_media_file_ref(output, "裁剪输出视频")?;
    match state
        .execute_media(MediaExecuteInput::TrimVideoSegment {
            input,
            output,
            start_secs,
            end_secs,
        })
        .await?
    {
        a4n_media_contract::MediaExecuteOutput::Unit => Ok(()),
        output => Err(format!("FFmpeg 裁剪返回了不匹配的输出：{output:?}")),
    }
}
