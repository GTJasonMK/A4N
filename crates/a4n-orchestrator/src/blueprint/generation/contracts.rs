use a4n_error::AppError;
use serde_json::Value;

mod stage_specs;
pub(in crate::blueprint) use stage_specs::BLUEPRINT_GENERATION_STAGES;

pub(in crate::blueprint) struct BlueprintStageSpec {
    pub(in crate::blueprint) key: &'static str,
    pub(in crate::blueprint) title: &'static str,
    pub(in crate::blueprint) validate: fn(&Value) -> Result<(), AppError>,
    pub(in crate::blueprint) progress_current: u8,
    pub(in crate::blueprint) progress_message: &'static str,
    pub(in crate::blueprint) restored_message: &'static str,
    pub(in crate::blueprint) output_schema: &'static str,
}
