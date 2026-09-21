mod batch_api;
mod config;
mod direct;
mod generation_io;
mod generation_task;
mod host_port;
mod page;
mod preview;
mod profiles;

use crate::AppState;
use crate::actions::request_preview::{ModelRequestPreview, model_request_preview};

pub(crate) use generation_io::{
    persist_reserved_generated_image_file, prepare_generated_image_file,
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};
pub(in crate::actions) use host_port::RuntimeImageGenerationHostPort;

pub use self::{
    batch_api::preview_batch_images_request,
    direct::preview_scene_image_request,
    page::preview_page_image_request,
    profiles::{
        generate_object_profile_image_prompt, generate_scene_profile_image_prompt,
        preview_object_profile_image_request,
    },
};
pub(crate) use a4n_canvas_contract::GenerateCanvasImageInput;
pub(crate) use direct::generate_galgame_image;
pub(crate) use direct::preview_canvas_image_request;
pub use generation_task::start_image_generation_typed;

pub(crate) use self::config::{resolve_image_config, resolve_image_provider_config};
