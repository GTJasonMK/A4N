use serde_json::Value;

use super::super::super::{
    ApiError, i64_arg, optional_json_i64_vec, optional_string_field, optional_string_vec_field,
    portrait_generation_actions, string_arg,
};

pub(super) fn generate_portrait_input(
    args: &Value,
    command: &str,
) -> Result<portrait_generation_actions::GeneratePortraitInput, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(portrait_generation_actions::GeneratePortraitInput {
        project_id,
        character_name: string_arg(args, "characterName")?,
        style: optional_string_field(args, "style")?,
        expressions: optional_string_vec_field(args, "expressions")?,
        pose: optional_string_field(args, "pose")?,
        reference_image_ids: optional_json_i64_vec(args, "referenceImageIds", command)?,
    })
}

pub(super) fn regenerate_portrait_input(
    args: &Value,
    command: &str,
) -> Result<portrait_generation_actions::RegeneratePortraitInput, ApiError> {
    Ok(portrait_generation_actions::RegeneratePortraitInput {
        portrait_id: i64_arg(args, "portraitId")?,
        reference_image_ids: optional_json_i64_vec(args, "referenceImageIds", command)?,
    })
}

pub(super) fn age_variant_input(
    args: &Value,
    command: &str,
) -> Result<portrait_generation_actions::GenerateAgeVariantInput, ApiError> {
    Ok(portrait_generation_actions::GenerateAgeVariantInput {
        portrait_id: i64_arg(args, "portraitId")?,
        age: string_arg(args, "age")?,
        reference_image_ids: optional_json_i64_vec(args, "referenceImageIds", command)?,
    })
}

pub(super) fn missing_portraits_input(
    args: &Value,
) -> Result<portrait_generation_actions::GenerateMissingPortraitsInput, ApiError> {
    Ok(portrait_generation_actions::GenerateMissingPortraitsInput {
        project_id: i64_arg(args, "projectId")?,
    })
}
