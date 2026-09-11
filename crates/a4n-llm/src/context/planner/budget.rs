mod capacity;
mod config;
mod rules;
mod selection;

use super::super::contracts::{COMPRESSION_MINIMAL, LlmContextPackRequirement, LlmContextPlan};
use a4n_error::AppError;

use capacity::calculate_input_capacity;
use rules::compression_rank;
use selection::select_requirement_variants;

pub use config::ContextBudgetPlanner;

impl ContextBudgetPlanner {
    pub fn plan_requirements(
        &self,
        model_context_tokens: usize,
        output_token_reserve: usize,
        requirements: Vec<LlmContextPackRequirement>,
        dynamic_delta_tokens: usize,
    ) -> Result<LlmContextPlan, AppError> {
        let capacity = calculate_input_capacity(
            model_context_tokens,
            output_token_reserve,
            self.safety_margin_tokens,
            self.input_utilization_percent,
            dynamic_delta_tokens,
        )?;
        let raw_available_input_tokens = capacity.raw_available_input_tokens;
        let available_input_tokens = capacity.available_input_tokens;
        let selection = select_requirement_variants(
            &requirements,
            available_input_tokens,
            dynamic_delta_tokens,
        )?;
        let selected_compression_level = selection
            .selected_refs
            .iter()
            .map(|pack| pack.compression_level.as_str())
            .max_by_key(|level| compression_rank(level))
            .unwrap_or(COMPRESSION_MINIMAL)
            .to_string();
        Ok(LlmContextPlan {
            model_context_tokens,
            output_token_reserve,
            safety_margin_tokens: self.safety_margin_tokens,
            input_utilization_percent: self.input_utilization_percent,
            raw_available_input_tokens,
            available_input_tokens,
            stable_prefix_tokens: selection.stable_prefix_tokens,
            dynamic_delta_tokens,
            selected_compression_level,
            selected_pack_refs: selection.selected_refs,
            selected_packs: selection.selected_packs,
            dropped_optional_pack_refs: selection.dropped,
            estimated_input_tokens: selection.used,
        })
    }
}
