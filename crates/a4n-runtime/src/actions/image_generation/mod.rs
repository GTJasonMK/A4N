mod batch_api;
mod config;
mod contracts;
mod direct;
mod generation_io;
mod generation_task;
mod host_port;
mod page;
mod preview;
mod profiles;

use crate::AppState;
use crate::actions::request_preview::{
    ModelRequestPreview, ModelRequestPreviewStep, image_provider_request_step,
    model_request_preview, provider_request_step,
};
use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;

pub(crate) use a4n_orchestrator::media::generated_image_summary as to_summary;
pub(crate) use generation_io::{
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};
pub(in crate::actions) use host_port::RuntimeImageGenerationHostPort;

pub use self::{
    batch_api::preview_batch_images_request,
    contracts::{
        GeneratedImageSummary, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
        SceneProfileImageInput, SceneProfileImagePromptOutput,
    },
    direct::{preview_canvas_image_request, preview_scene_image_request},
    page::preview_page_image_request,
    profiles::{
        generate_object_profile_image_prompt, generate_scene_profile_image_prompt,
        preview_object_profile_image_request,
    },
};
pub(crate) use contracts::{GenerateCanvasImageInput, GenerateGalgameImageInput};
pub(crate) use direct::generate_galgame_image;
pub use generation_task::start_image_generation_typed;

pub(crate) use self::config::resolve_image_config;
