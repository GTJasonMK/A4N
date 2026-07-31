use super::ReverseEngineeringTarget;
use a4n_core::config::app_config::llm_token_profiles;

#[test]
fn reverse_targets_keep_stable_prompt_names_and_token_profiles() {
    assert_eq!(
        ReverseEngineeringTarget::Blueprint.prompt_name(),
        "reverse_blueprint"
    );
    assert_eq!(
        ReverseEngineeringTarget::Blueprint.max_tokens_profile(),
        llm_token_profiles::BLUEPRINT
    );
    assert_eq!(
        ReverseEngineeringTarget::Outline.prompt_name(),
        "reverse_outline"
    );
    assert_eq!(
        ReverseEngineeringTarget::Outline.max_tokens_profile(),
        llm_token_profiles::OUTLINE
    );
    assert_eq!(
        ReverseEngineeringTarget::PartOutline.prompt_name(),
        "reverse_part_outline"
    );
}

#[test]
fn reverse_targets_distinguish_preview_and_apply_messages() {
    let target = ReverseEngineeringTarget::Outline;

    assert_eq!(target.initial_message(false), "正在准备反推章节大纲");
    assert_eq!(target.initial_message(true), "正在准备反推章节大纲并应用");
    assert_eq!(target.success_message(false), "反推章节大纲完成");
    assert_eq!(target.success_message(true), "反推章节大纲结果已应用");
    assert_eq!(target.cancelled_message(), "反推章节大纲已停止");
}
