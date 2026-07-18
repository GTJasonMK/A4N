use serde_json::Value;

use a4n_core::models::ai::image_gen_config;
use a4n_image::ImageGenerationRequest;

pub(in crate::actions::portrait_generation) struct PortraitImageMirror<'a> {
    pub(in crate::actions::portrait_generation) workflow_run_id: Option<i64>,
    pub(in crate::actions::portrait_generation) project_id: i64,
    pub(in crate::actions::portrait_generation) profile_id: i64,
    pub(in crate::actions::portrait_generation) portrait_id: i64,
    pub(in crate::actions::portrait_generation) character_name: &'a str,
    pub(in crate::actions::portrait_generation) portrait_type: &'a str,
    pub(in crate::actions::portrait_generation) image_path: &'a str,
    pub(in crate::actions::portrait_generation) prompt: &'a str,
    pub(in crate::actions::portrait_generation) model: &'a str,
    pub(in crate::actions::portrait_generation) width: i32,
    pub(in crate::actions::portrait_generation) height: i32,
    pub(in crate::actions::portrait_generation) file_size: i64,
    pub(in crate::actions::portrait_generation) expression: Option<&'a str>,
    pub(in crate::actions::portrait_generation) pose: Option<&'a str>,
    pub(in crate::actions::portrait_generation) variant_kind: Option<&'a str>,
    pub(in crate::actions::portrait_generation) auto_generated: bool,
    pub(in crate::actions::portrait_generation) reference_image_ids: &'a [i64],
    pub(in crate::actions::portrait_generation) provider_trace:
        Option<PortraitImageProviderTrace<'a>>,
}

pub(in crate::actions::portrait_generation) struct PortraitImageProviderTrace<'a> {
    pub(super) provider: &'a str,
    pub(super) request_json: &'a Value,
    pub(super) output_format: &'a str,
}

pub(in crate::actions::portrait_generation) struct PersistGeneratedPortraitInput<'a> {
    pub(in crate::actions::portrait_generation) workflow_run_id: Option<i64>,
    pub(in crate::actions::portrait_generation) project_id: i64,
    pub(in crate::actions::portrait_generation) profile_id: i64,
    pub(in crate::actions::portrait_generation) character_name: &'a str,
    pub(in crate::actions::portrait_generation) portrait_type: &'a str,
    pub(in crate::actions::portrait_generation) prompt: &'a str,
    pub(in crate::actions::portrait_generation) expression: Option<&'a str>,
    pub(in crate::actions::portrait_generation) pose: Option<&'a str>,
    pub(in crate::actions::portrait_generation) is_primary: bool,
    pub(in crate::actions::portrait_generation) variant_kind: Option<&'a str>,
    pub(in crate::actions::portrait_generation) auto_generated: bool,
    pub(in crate::actions::portrait_generation) path_context: &'a str,
    pub(in crate::actions::portrait_generation) reference_image_ids: &'a [i64],
}

pub(in crate::actions::portrait_generation) struct PortraitImageRequestContext {
    pub(in crate::actions::portrait_generation) request: ImageGenerationRequest,
    pub(in crate::actions::portrait_generation) config: image_gen_config::Model,
}

pub(in crate::actions::portrait_generation) struct GeneratedPortraitImage {
    pub(in crate::actions::portrait_generation) bytes: Vec<u8>,
    pub(in crate::actions::portrait_generation) format: String,
    pub(in crate::actions::portrait_generation) model: String,
    pub(in crate::actions::portrait_generation) provider: String,
    pub(in crate::actions::portrait_generation) request_json: Value,
    pub(in crate::actions::portrait_generation) width: i32,
    pub(in crate::actions::portrait_generation) height: i32,
}

pub(super) struct InsertPortraitInput<'a> {
    pub(super) profile_id: i64,
    pub(super) project_id: i64,
    pub(super) portrait_type: &'a str,
    pub(super) image_path: &'a str,
    pub(super) prompt: &'a str,
    pub(super) expression: Option<&'a str>,
    pub(super) pose: Option<&'a str>,
    pub(super) is_primary: bool,
}
