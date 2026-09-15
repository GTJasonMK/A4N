use self::variant::select_variant;
use super::super::super::contracts::{
    LlmContextPackRef, LlmContextPackRequirement, PlannedContextPack,
};
use super::rules::{checked_context_token_add, remaining_required_min_token_suffix};
use a4n_error::AppError;

mod variant;

pub(super) struct RequirementSelection {
    pub(super) selected_refs: Vec<LlmContextPackRef>,
    pub(super) selected_packs: Vec<PlannedContextPack>,
    pub(super) dropped: Vec<LlmContextPackRef>,
    pub(super) stable_prefix_tokens: usize,
    pub(super) used: usize,
}

pub(super) fn select_requirement_variants(
    requirements: &[LlmContextPackRequirement],
    available_input_tokens: usize,
    dynamic_delta_tokens: usize,
) -> Result<RequirementSelection, AppError> {
    let mut selected_refs = Vec::new();
    let mut selected_packs = Vec::new();
    let mut dropped = Vec::new();
    let mut stable_prefix_tokens = 0_usize;
    let mut used = dynamic_delta_tokens;
    let remaining_required_min_tokens = remaining_required_min_token_suffix(requirements)?;
    for requirement_index in 0..requirements.len() {
        let requirement = &requirements[requirement_index];
        if requirement.variants.is_empty() {
            if requirement.required {
                return Err(AppError::InvalidArgument(format!(
                    "必需上下文包 {} 没有可用压缩变体",
                    requirement.pack_kind
                )));
            }
            continue;
        }
        let later_required_min_tokens = remaining_required_min_tokens[requirement_index + 1];
        let (selected_variant, variants) = select_variant(
            &requirement.variants,
            used,
            later_required_min_tokens,
            available_input_tokens,
        );
        match selected_variant {
            Some(pack_ref) => {
                used = checked_context_token_add(
                    used,
                    pack_ref.token_estimate,
                    "已用 token 与选中上下文包之和",
                )?;
                if requirement.stable_prefix {
                    stable_prefix_tokens = checked_context_token_add(
                        stable_prefix_tokens,
                        pack_ref.token_estimate,
                        "稳定前缀 token 之和",
                    )?;
                }
                selected_refs.push(pack_ref.clone());
                selected_packs.push(PlannedContextPack {
                    pack_ref: pack_ref.clone(),
                    stable_prefix: requirement.stable_prefix,
                    canonical_content_text: None,
                });
            }
            None if requirement.required => {
                let smallest = variants
                    .iter()
                    .min_by_key(|variant| variant.token_estimate)
                    .expect("variants checked non-empty");
                let required_with_variant = checked_context_token_add(
                    used,
                    smallest.token_estimate,
                    "当前已用 token 与必需上下文最小变体之和",
                )?;
                let required_total = checked_context_token_add(
                    required_with_variant,
                    later_required_min_tokens,
                    "当前必需上下文与后续最小 token 之和",
                )?;
                return Err(AppError::InvalidArgument(format!(
                    "必需上下文包 {} 无法放入输入窗口：当前已用约 {} tokens，最小压缩变体约 {} tokens，后续必需上下文至少 {} tokens，所需合计约 {} tokens，可用输入窗口 {}",
                    requirement.pack_kind,
                    used,
                    smallest.token_estimate,
                    later_required_min_tokens,
                    required_total,
                    available_input_tokens,
                )));
            }
            None => dropped.extend(variants),
        }
    }
    Ok(RequirementSelection {
        selected_refs,
        selected_packs,
        dropped,
        stable_prefix_tokens,
        used,
    })
}
