use a4n_domain::AssemblyPostprocessOperation;
use a4n_media_contract::{MediaExecuteInput, MediaExecuteOutput, MediaPostprocessOperation};
use a4n_media_render_core::MediaRoots;

use crate::{
    VideoAssemblyClip, VideoAssemblyRequest, VideoPostprocessRequest, assemble_videos,
    extract_video_frame, extract_video_tail_frame, generate_video_thumbnail, postprocess_video,
    probe_audio_duration_secs, probe_video_has_audio, probe_video_properties, trim_video_segment,
};

pub fn execute(roots: &MediaRoots, input: MediaExecuteInput) -> Result<MediaExecuteOutput, String> {
    match input {
        MediaExecuteInput::ProbeVideoProperties { input } => {
            let properties =
                probe_video_properties(&roots.input(&input)?).map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::VideoProbeProperties {
                width: properties.width,
                height: properties.height,
                duration_secs: properties.duration_secs,
                fps: properties.fps,
                codec_name: properties.codec_name,
                profile: properties.profile,
                has_b_frames: properties.has_b_frames,
            })
        }
        MediaExecuteInput::ProbeVideoHasAudio { input } => Ok(MediaExecuteOutput::HasAudio {
            has_audio: probe_video_has_audio(&roots.input(&input)?)
                .map_err(|error| error.to_string())?,
        }),
        MediaExecuteInput::ProbeAudioDuration { input } => Ok(MediaExecuteOutput::AudioDuration {
            duration_secs: probe_audio_duration_secs(&roots.input(&input)?)
                .map_err(|error| error.to_string())?,
        }),
        MediaExecuteInput::GenerateVideoScrubProxy { .. }
        | MediaExecuteInput::GenerateVideoScrubFrames { .. } => {
            Ok(a4n_media_render_core::execute(roots, input)?)
        }
        MediaExecuteInput::ExtractVideoFrame {
            input,
            output,
            at_secs,
        } => {
            extract_video_frame(&roots.input(&input)?, &roots.output(&output)?, at_secs)
                .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
        MediaExecuteInput::ExtractVideoTailFrame { input, output } => {
            extract_video_tail_frame(&roots.input(&input)?, &roots.output(&output)?)
                .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
        MediaExecuteInput::GenerateVideoThumbnail { input, output } => {
            generate_video_thumbnail(&roots.input(&input)?, &roots.output(&output)?)
                .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
        MediaExecuteInput::TrimVideoSegment {
            input,
            output,
            start_secs,
            end_secs,
        } => {
            trim_video_segment(
                &roots.input(&input)?,
                &roots.output(&output)?,
                start_secs,
                end_secs,
            )
            .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
        MediaExecuteInput::AssembleVideos {
            clips,
            output,
            width,
            height,
            fps,
            audio_enabled,
        } => {
            let paths = clips
                .iter()
                .map(|clip| roots.input(&clip.input))
                .collect::<Result<Vec<_>, _>>()?;
            let media_clips = clips
                .iter()
                .zip(&paths)
                .map(|(clip, path)| VideoAssemblyClip {
                    path,
                    duration_secs: clip.duration_secs,
                    has_audio: clip.has_audio,
                })
                .collect::<Vec<_>>();
            let output = roots.output(&output)?;
            assemble_videos(VideoAssemblyRequest {
                clips: &media_clips,
                output_path: &output,
                width,
                height,
                fps,
                audio_enabled,
            })
            .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
        MediaExecuteInput::PostprocessVideo {
            base,
            source,
            output,
            operation,
            start_secs,
            end_secs,
            base_duration_secs,
            base_has_audio,
            source_duration_secs,
            source_has_audio,
            width,
            height,
            fps,
        } => {
            let base = roots.input(&base)?;
            let source = source
                .as_ref()
                .map(|source| roots.input(source))
                .transpose()?;
            let output = roots.output(&output)?;
            postprocess_video(VideoPostprocessRequest {
                base_path: &base,
                source_path: source.as_deref(),
                output_path: &output,
                operation: map_postprocess_operation(operation),
                start_secs,
                end_secs,
                base_duration_secs,
                base_has_audio,
                source_duration_secs,
                source_has_audio,
                width,
                height,
                fps,
            })
            .map_err(|error| error.to_string())?;
            Ok(MediaExecuteOutput::Unit)
        }
    }
}

fn map_postprocess_operation(operation: MediaPostprocessOperation) -> AssemblyPostprocessOperation {
    match operation {
        MediaPostprocessOperation::Insert => AssemblyPostprocessOperation::Insert,
        MediaPostprocessOperation::Delete => AssemblyPostprocessOperation::Delete,
        MediaPostprocessOperation::Replace => AssemblyPostprocessOperation::Replace,
    }
}
