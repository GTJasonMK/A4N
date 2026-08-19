use a4n_core::error::AppError;
use serde_json::Value;

mod stage_specs;
pub use stage_specs::BLUEPRINT_GENERATION_STAGES;

pub struct BlueprintStageSpec {
    pub key: &'static str,
    pub title: &'static str,
    pub validate: fn(&Value) -> Result<(), AppError>,
    pub progress_current: u8,
    pub progress_message: &'static str,
    pub restored_message: &'static str,
    pub output_schema: &'static str,
}
