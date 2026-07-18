use super::super::generation::age_variant_prompt;
use super::super::image_pipeline::{
    build_portrait_image_request, character_portrait_by_id, validate_reference_image_ids,
};
use super::super::prompt::{build_generate_portrait_prompt_request, profile_portrait_seed_prompt};
use super::super::types::{
    GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
    RegeneratePortraitInput,
};
use super::steps::{
    blocked_preview_step, chat_preview_step, portrait_preview, provider_preview_step,
};
use crate::actions::access;
use crate::actions::portrait_records::portrait_project_id_for_user;
use crate::actions::request_preview::ModelRequestPreview;
use crate::state::AppState;

pub async fn preview_generate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let context = build_generate_portrait_prompt_request(
        state,
        input.project_id,
        &input.character_name,
        input.style.as_deref(),
        input.expressions.as_deref(),
        input.pose.as_deref(),
    )
    .await?;
    let reference_count = validate_reference_image_ids(&input.reference_image_ids)?.len();
    let image_step_message = if reference_count > 0 {
        format!(
            "该图片请求依赖上一步模型返回的 image prompt，生成前无法伪造完整 provider request；真实生成会附带 {} 张参考图。",
            reference_count
        )
    } else {
        "该图片请求依赖上一步模型返回的 image prompt，生成前无法伪造完整 provider request"
            .to_string()
    };
    portrait_preview(
        "portrait_generation",
        "人物三视图拆解生成请求预览",
        vec![
            chat_preview_step(
                state,
                "portrait_prompt",
                "生成图片提示词的 LLM 请求",
                Some("character_portrait_generation"),
                context.request,
            )
            .await?,
            blocked_preview_step(
                "portrait_image",
                "人物三视图拆解图片 provider 请求",
                "image_generation",
                &image_step_message,
            ),
        ],
    )
}

pub async fn preview_regenerate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    let context = build_portrait_image_request(
        state,
        existing.project_id,
        &existing.prompt,
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
    portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    let context = build_portrait_image_request(
        state,
        existing.project_id,
        &age_variant_prompt(&existing.prompt, &input.age),
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
        .orchestrator
        .missing_portrait_profiles(input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let mut steps = Vec::with_capacity(missing_profiles.len());
    for (index, profile) in missing_profiles.iter().enumerate() {
        let prompt = profile_portrait_seed_prompt(profile)?;
        let context = build_portrait_image_request(state, input.project_id, &prompt, &[]).await?;
        steps.push(
            provider_preview_step(
                state,
                input.project_id,
                &format!("missing_portrait_{}", index + 1),
                &format!("补齐角色「{}」人物三视图拆解 provider 请求", profile.name),
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
