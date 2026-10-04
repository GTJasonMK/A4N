use std::collections::HashMap;

use a4n_api_contract::portrait::{
    GeneratePortraitInput, GeneratePortraitPromptInput, PortraitType,
};
use a4n_domain::character::CharacterProfileRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    portrait::{
        PortraitPromptRequestInput, PortraitPromptSpec, expression_or_default,
        portrait_profile_context, portrait_style_with_pose,
    },
};

pub(super) struct PreparedNamedPortraitRequest {
    pub(super) profile: CharacterProfileRecord,
    pub(super) expression: String,
    pub(super) request: ChatRequest,
}

pub(super) struct PreparedProfilePortraitRequest {
    pub(super) profile: CharacterProfileRecord,
    pub(super) profile_context: Value,
    pub(super) project_context: Option<Value>,
    pub(super) request: ChatRequest,
}

pub(super) async fn prepare_named_portrait_request(
    context: &OrchestratorContext,
    input: &GeneratePortraitInput,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<PreparedNamedPortraitRequest, AppError> {
    let rows = context
        .portrait_named_prompt_profile_rows(input.project_id, &input.character_name)
        .await?;
    let profile = rows.profile;
    let existing_count = rows.existing_portrait_count;
    let profile_context = portrait_profile_context(&profile);
    let expression = expression_or_default(input.expression.as_deref());
    let style_with_pose = portrait_style_with_pose(input.style.as_deref(), input.pose.as_deref());
    let variables = HashMap::new();
    let request = context
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec: PortraitPromptSpec {
                profile_name: &profile.name,
                appearance: profile.appearance.as_deref(),
                gender: profile.gender.as_deref(),
                age: profile.age.as_deref(),
                personality: profile.personality.as_deref(),
                background: profile.background.as_deref(),
                profile_context: Some(&profile_context),
                project_context: rows.project_context.as_ref(),
                portrait_type: PortraitType::FullBody,
                art_style: style_with_pose.as_deref(),
                expression: Some(&expression),
                existing_portrait_count: Some(existing_count),
            },
            prompt_renderer,
            variables: &variables,
        })
        .await?;
    Ok(PreparedNamedPortraitRequest {
        profile,
        expression,
        request,
    })
}

pub(super) async fn prepare_profile_portrait_request(
    context: &OrchestratorContext,
    input: &GeneratePortraitPromptInput,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<PreparedProfilePortraitRequest, AppError> {
    let rows = context
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await?;
    let profile = rows.profile;
    let profile_context = portrait_profile_context(&profile);
    let project_context = rows.project_context;
    let request = context
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec: PortraitPromptSpec {
                profile_name: &profile.name,
                appearance: profile.appearance.as_deref(),
                gender: profile.gender.as_deref(),
                age: profile.age.as_deref(),
                personality: profile.personality.as_deref(),
                background: profile.background.as_deref(),
                profile_context: Some(&profile_context),
                project_context: project_context.as_ref(),
                portrait_type: input.portrait_type,
                art_style: None,
                expression: None,
                existing_portrait_count: None,
            },
            prompt_renderer,
            variables,
        })
        .await?;
    Ok(PreparedProfilePortraitRequest {
        profile,
        profile_context,
        project_context,
        request,
    })
}
