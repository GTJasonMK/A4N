use super::super::{
    AppState, GeneratePortraitPromptInput, ModelRequestPreview, PortraitPromptRequestInput,
    PortraitPromptSpec, RuntimePromptRenderer, access, chat_preview_step, parse_template_variables,
    portrait_preview,
};
use super::rules::portrait_profile_context;

pub async fn preview_generate_portrait_prompt_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(input.variables.as_ref())?;
    let rows = state
        .orchestrator
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await
        .map_err(|err| err.to_string())?;
    let profile = rows.profile;
    let profile_context = portrait_profile_context(&profile);
    let project_context = rows.project_context;
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: &input.portrait_type,
        art_style: None,
        expression: None,
        existing_portrait_count: None,
    };
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
        })
        .await
        .map_err(|error| error.to_string())?;
    portrait_preview(
        "portrait_prompt_generation",
        "人物三视图拆解提示词 LLM 请求预览",
        vec![
            chat_preview_step(
                state,
                "portrait_prompt",
                "人物三视图拆解提示词 LLM 请求",
                Some("character_portrait_generation"),
                request,
            )
            .await?,
        ],
    )
}
