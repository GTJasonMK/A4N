use super::steps::{chat_preview_step, portrait_preview, provider_preview_step};
use crate::{
    actions::{
        access, image_generation::resolve_image_config, request_preview::ModelRequestPreview,
    },
    state::AppState,
};
use a4n_api_contract::portrait::{
    GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
    GeneratePortraitPromptInput, PortraitSummary, RegeneratePortraitInput,
};
use a4n_domain::image::validate_reference_image_ids;
use a4n_prompt::variables::parse_template_variables;

use super::super::host_port::{
    build_portrait_image_request_preview, build_portrait_image_request_preview_with_config,
};

pub async fn preview_generate_portrait_prompt_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(input.variables.as_ref())?;
    let prompt_renderer = state;
    let request = state
        .portrait_generation_application()
        .prepare_portrait_prompt_request(&input, prompt_renderer, &variables)
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

pub async fn preview_generate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    validate_reference_image_ids(&input.reference_image_ids)?;
    let prompt_renderer = state;
    let request = state
        .portrait_generation_application()
        .prepare_portrait_request(&input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    portrait_preview(
        "portrait_generation",
        "人物三视图拆解生成请求预览",
        vec![
            chat_preview_step(
                state,
                "portrait_prompt",
                "生成图片提示词的 LLM 请求",
                Some("character_portrait_generation"),
                request,
            )
            .await?,
        ],
    )
}

pub async fn preview_regenerate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    let existing = required_portrait(state, user_id, input.portrait_id).await?;
    let context = build_portrait_image_request_preview(
        state,
        user_id,
        existing.project_id,
        &existing.prompt,
        input.width,
        input.height,
        &input.reference_image_ids,
    )
    .await?;
    portrait_preview(
        "portrait_regeneration",
        "立绘重绘请求预览",
        vec![
            provider_preview_step(
                state,
                existing.project_id,
                "portrait_image",
                "立绘图片 provider 请求",
                &context.config,
                context.request,
            )
            .await?,
        ],
    )
}

pub async fn preview_generate_age_variant_request(
    state: &AppState,
    user_id: i64,
    input: GenerateAgeVariantInput,
) -> Result<ModelRequestPreview, String> {
    let existing = required_portrait(state, user_id, input.portrait_id).await?;
    let prompt = format!("{}，age: {}", existing.prompt, input.age);
    let context = build_portrait_image_request_preview(
        state,
        user_id,
        existing.project_id,
        &prompt,
        input.width,
        input.height,
        &input.reference_image_ids,
    )
    .await?;
    portrait_preview(
        "portrait_age_variant_generation",
        "立绘年龄变体请求预览",
        vec![
            provider_preview_step(
                state,
                existing.project_id,
                "portrait_age_variant_image",
                "年龄变体图片 provider 请求",
                &context.config,
                context.request,
            )
            .await?,
        ],
    )
}

pub async fn preview_generate_missing_portraits_request(
    state: &AppState,
    user_id: i64,
    input: GenerateMissingPortraitsInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let missing_profiles = state
        .portrait_record_application()
        .missing_generation_seeds(user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let config = resolve_image_config(state, None).await?;
    let mut steps = Vec::with_capacity(missing_profiles.len());
    for (index, profile) in missing_profiles.iter().enumerate() {
        let prompt = &profile.prompt;
        let context = build_portrait_image_request_preview_with_config(
            state,
            user_id,
            input.project_id,
            &prompt,
            input.width,
            input.height,
            &[],
            &config,
        )
        .await?;
        steps.push(
            provider_preview_step(
                state,
                input.project_id,
                &format!("missing_portrait_{}", index + 1),
                &format!(
                    "补齐角色「{}」人物三视图拆解 provider 请求",
                    profile.profile_name
                ),
                &context.config,
                context.request,
            )
            .await?,
        );
    }
    portrait_preview(
        "missing_portrait_generation",
        "补齐人物三视图拆解请求预览",
        steps,
    )
}

async fn required_portrait(
    state: &AppState,
    user_id: i64,
    portrait_id: i64,
) -> Result<PortraitSummary, String> {
    state
        .portrait_record_application()
        .by_id(user_id, portrait_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "立绘不存在".to_string())
}
