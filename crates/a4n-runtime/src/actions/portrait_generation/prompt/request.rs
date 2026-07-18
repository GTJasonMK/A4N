use std::collections::HashMap;

use a4n_core::models::character::protagonist_profile;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::portrait::{PortraitPromptRequestInput, PortraitPromptSpec};

use crate::AppState;
use crate::prompt_renderer::RuntimePromptRenderer;

use super::rules::{
    first_expression_or_default, portrait_profile_context, portrait_style_with_pose,
};

pub(in crate::actions::portrait_generation) struct GeneratePortraitPromptRequestContext {
    pub(in crate::actions::portrait_generation) profile: protagonist_profile::Model,
    pub(in crate::actions::portrait_generation) existing_count: u64,
    pub(in crate::actions::portrait_generation) expression: String,
    pub(in crate::actions::portrait_generation) request: ChatRequest,
}

pub(in crate::actions::portrait_generation) async fn build_generate_portrait_prompt_request(
    state: &AppState,
    project_id: i64,
    character_name: &str,
    style: Option<&str>,
    expressions: Option<&[String]>,
    pose: Option<&str>,
) -> Result<GeneratePortraitPromptRequestContext, String> {
    let rows = state
        .orchestrator
        .portrait_named_prompt_profile_rows(project_id, character_name)
        .await
        .map_err(|err| err.to_string())?;
    let profile = rows.profile;
    let existing_count = rows.existing_portrait_count;
    let profile_context = portrait_profile_context(&profile);
    let project_context = rows.project_context;
    let expression = first_expression_or_default(expressions);
    let style_with_pose = portrait_style_with_pose(style, pose);
    let variables = HashMap::new();
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: "full_body",
        art_style: style_with_pose.as_deref(),
        expression: Some(&expression),
        existing_portrait_count: Some(existing_count),
    };
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(GeneratePortraitPromptRequestContext {
        profile,
        existing_count,
        expression,
        request,
    })
}
