use a4n_core::models::ai::image_gen_config;
use serde_json::Value;

pub(in crate::actions::image_generation) struct GeneratedImageSaveInput<'a> {
    pub(in crate::actions::image_generation) project_id: i64,
    pub(in crate::actions::image_generation) chapter_id: Option<i64>,
    pub(in crate::actions::image_generation) image_type: String,
    pub(in crate::actions::image_generation) command: &'a str,
    pub(in crate::actions::image_generation) config: &'a image_gen_config::Model,
    pub(in crate::actions::image_generation) file_extra: Option<String>,
    pub(in crate::actions::image_generation) file_context: &'a str,
    pub(in crate::actions::image_generation) prompt: String,
    pub(in crate::actions::image_generation) request_json: Value,
    pub(in crate::actions::image_generation) width: i32,
    pub(in crate::actions::image_generation) height: i32,
    pub(in crate::actions::image_generation) panel_id: Option<String>,
    pub(in crate::actions::image_generation) metadata_json: Option<Value>,
    pub(in crate::actions::image_generation) created_at: chrono::NaiveDateTime,
    pub(in crate::actions::image_generation) workflow_run_id: Option<i64>,
}
