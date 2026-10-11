use a4n_error::AppError;
use a4n_video::ProviderImageInputTransport;
use a4n_video_host_contract::{
    ChapterVideoImageInput, ResolvedChapterVideoImageInputs, VideoProviderValidationMode,
};

use crate::{AppState, actions::video_generation::media::*};

use super::provider_host::host_error;

pub(super) async fn resolve_chapter_video_image_inputs(
    state: &AppState,
    input: ChapterVideoImageInput,
) -> Result<ResolvedChapterVideoImageInputs, AppError> {
    let video = state.video_record_read_application();
    match input.mode {
        VideoProviderValidationMode::Generate => {
            let jwt_secret = if input.transport == ProviderImageInputTransport::PublicUrl {
                Some(state.required_jwt_secret()?)
            } else {
                None
            };
            let provider_context = ReferenceImageProviderContext {
                video,
                data_dir: state.data_dir(),
                jwt_secret,
                user_id: input.user_id,
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                transport: input.transport,
            };
            if input.transport == ProviderImageInputTransport::RemoteTempUpload {
                let mut staging_artifacts = Vec::new();
                let reference_image_urls = reference_image_provider_inputs_by_ids(
                    &provider_context,
                    input.reference_image_ids,
                    &mut staging_artifacts,
                )
                .await
                .map_err(host_error)?;
                let first_frame_image_url = reference_image_provider_input_by_id(
                    &provider_context,
                    input.first_frame_image_id,
                    &mut staging_artifacts,
                )
                .await
                .map_err(host_error)?;
                let last_frame_image_url = reference_image_provider_input_by_id(
                    &provider_context,
                    input.last_frame_image_id,
                    &mut staging_artifacts,
                )
                .await
                .map_err(host_error)?;
                return Ok(ResolvedChapterVideoImageInputs {
                    reference_image_urls,
                    first_frame_image_url,
                    last_frame_image_url,
                    staging_artifacts,
                });
            }
            let mut reference_staging_artifacts = Vec::new();
            let mut first_frame_staging_artifacts = Vec::new();
            let mut last_frame_staging_artifacts = Vec::new();
            let (reference_result, first_frame_result, last_frame_result) = futures::join!(
                reference_image_provider_inputs_by_ids(
                    &provider_context,
                    input.reference_image_ids,
                    &mut reference_staging_artifacts,
                ),
                reference_image_provider_input_by_id(
                    &provider_context,
                    input.first_frame_image_id,
                    &mut first_frame_staging_artifacts,
                ),
                reference_image_provider_input_by_id(
                    &provider_context,
                    input.last_frame_image_id,
                    &mut last_frame_staging_artifacts,
                ),
            );
            let reference_image_urls = reference_result.map_err(host_error)?;
            let first_frame_image_url = first_frame_result.map_err(host_error)?;
            let last_frame_image_url = last_frame_result.map_err(host_error)?;
            let mut staging_artifacts = reference_staging_artifacts;
            staging_artifacts.extend(first_frame_staging_artifacts);
            staging_artifacts.extend(last_frame_staging_artifacts);
            Ok(ResolvedChapterVideoImageInputs {
                reference_image_urls,
                first_frame_image_url,
                last_frame_image_url,
                staging_artifacts,
            })
        }
        VideoProviderValidationMode::Preview => {
            let (reference_result, first_frame_result, last_frame_result) = futures::join!(
                reference_image_request_preview_urls_by_ids(
                    video,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.reference_image_ids,
                ),
                reference_image_request_preview_url_by_id(
                    video,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.first_frame_image_id,
                ),
                reference_image_request_preview_url_by_id(
                    video,
                    input.user_id,
                    input.project_id,
                    input.chapter_id,
                    input.last_frame_image_id,
                ),
            );
            let reference_image_urls = reference_result.map_err(host_error)?;
            let first_frame_image_url = first_frame_result.map_err(host_error)?;
            let last_frame_image_url = last_frame_result.map_err(host_error)?;
            Ok(ResolvedChapterVideoImageInputs {
                reference_image_urls,
                first_frame_image_url,
                last_frame_image_url,
                staging_artifacts: Vec::new(),
            })
        }
    }
}
