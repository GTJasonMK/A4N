use a4n_error::AppError;
use serde_json::{Map, Value};

use super::super::contracts::VideoPromptContractMode;
use super::source::rule_text;

pub(super) fn video_prompt_duration_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
) -> Result<String, AppError> {
    let key = match mode {
        VideoPromptContractMode::Api => "duration_rule_api_draft",
        VideoPromptContractMode::Chat => "duration_rule_chat_draft",
    };
    rule_text(rules, key)
}

pub(super) fn video_prompt_shot_number_rule(
    rules: &Map<String, Value>,
) -> Result<String, AppError> {
    rule_text(rules, "shot_number_rule_draft")
}

pub(super) fn video_prompt_dialogue_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
) -> Result<String, AppError> {
    let key = match mode {
        VideoPromptContractMode::Api => "dialogue_rule_api_draft",
        VideoPromptContractMode::Chat => "dialogue_rule_chat_draft",
    };
    rule_text(rules, key)
}

pub(super) fn video_prompt_temporal_rule(
    rules: &Map<String, Value>,
    mode: VideoPromptContractMode,
) -> Result<String, AppError> {
    let key = match mode {
        VideoPromptContractMode::Api => "temporal_beat_rule_api",
        VideoPromptContractMode::Chat => "temporal_beat_rule_chat",
    };
    rule_text(rules, key)
}
