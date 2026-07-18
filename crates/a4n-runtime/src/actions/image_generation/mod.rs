mod batch;
mod generation_io;
mod source_binding;
mod video_frame_context;
mod workflow;

use crate::AppState;
use crate::actions::access;
use crate::actions::profile_materials::{
    ProfileMaterialStyleMetadata, object_profile_content, object_profile_image_prompt,
    profile_material_metadata, scene_profile_content, scene_profile_image_prompt,
};
use crate::actions::request_preview::{
    ModelRequestPreview, ModelRequestPreviewStep, image_provider_request_step,
    model_request_preview, provider_request_step,
};
use a4n_core::models::ai::{generated_image, image_gen_config};
use a4n_image::ImageGenerationRequest;
use a4n_orchestrator::config::image_backend_supports_reference_images;
use a4n_orchestrator::manga::{MANGA_PAGE_PROMPT_TYPE, ensure_manga_source_can_write};
use batch::{batch_image_step_note, build_batch_image_jobs};
use futures::stream::{self, StreamExt};
use generation_io::{
    GeneratedImageSaveInput, image_generation_request_trace_json,
    run_image_generation_provider_request_with_workflow, save_generated_image_result,
};
use serde_json::{Value, json};
use source_binding::{
    ImageChapterBindingAccess, collect_bound_reference_images,
    ensure_manga_page_panels_match_current_source, resolve_image_chapter_binding,
};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use workflow::{
    batch_image_item_fingerprint, batch_image_item_step_key, finalize_batch_image_workflow,
    finalize_single_image_workflow, load_completed_image_checkpoint,
    save_completed_batch_image_checkpoint, save_completed_image_checkpoint,
    start_batch_image_workflow, start_image_generation_workflow,
};

pub(crate) use generation_io::{
    run_queued_request_with_cancel, run_queued_request_with_cancel_and_workflow,
};

mod batch_api;
mod config;
mod contracts;
mod direct;
mod metadata;
mod page;
mod preview;
mod profiles;
mod request;
mod scene;

pub(crate) use self::config::{normalize_image_metadata, resolve_image_config, to_summary};
pub use self::{
    batch_api::batch_generate_images,
    contracts::{
        GenerateCanvasImageInput, GenerateGalgameImageInput, GeneratedImageSummary,
        ImageGenerationProgress, ImageGenerationTaskScope, ObjectProfileImageInput,
        ObjectProfileImagePromptOutput, SceneProfileImageInput, SceneProfileImagePromptOutput,
    },
    direct::{
        generate_canvas_image, generate_galgame_image, preview_canvas_image_request,
        preview_scene_image_request,
    },
    page::generate_page_image,
    profiles::{
        generate_object_profile_image, generate_object_profile_image_prompt,
        generate_scene_profile_image, generate_scene_profile_image_prompt,
        preview_batch_images_request, preview_object_profile_image_request,
        preview_page_image_request,
    },
    scene::{
        batch_image_generation_task_scope, generate_scene_image,
        object_profile_image_generation_task_scope, page_image_generation_task_scope,
        scene_image_generation_task_scope, scene_profile_image_generation_task_scope,
    },
};
use self::{
    contracts::{
        DEFAULT_IMAGE_GENERATION_PROGRESS, DirectImageGenerationInput,
        IMAGE_METADATA_SCHEMA_VERSION, OBJECT_PROFILE_IMAGE_SPEC, PageImageRequestContext,
        ProfileImageMaterial, SCENE_PROFILE_IMAGE_SPEC, SceneImageRequestContext,
    },
    direct::preview_direct_image_request,
    metadata::{
        merge_direct_image_metadata, merge_direct_image_source_metadata, merge_image_metadata,
    },
    preview::image_generation_preview_step,
    request::{
        build_direct_image_request, build_object_profile_material, build_page_image_request,
        build_scene_image_request, build_scene_profile_material, profile_material_direct_input,
    },
    scene::generate_direct_image,
};

#[cfg(test)]
mod tests;
