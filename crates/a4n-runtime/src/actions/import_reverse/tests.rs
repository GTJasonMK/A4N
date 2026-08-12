use super::ReverseEngineeringTarget;
use a4n_core::config::app_config::llm_token_profiles;
use serde_json::json;

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

    assert_eq!(target.initial_message(), "正在准备章节大纲候选");
    assert_eq!(target.success_message(), "章节大纲候选已生成");
    assert_eq!(target.cancelled_message(), "反推章节大纲已停止");
}

#[test]
fn material_sync_warnings_extend_the_committed_response() {
    let mut response = json!({"warnings": ["项目状态更新失败"]});

    super::api::append_response_warnings(&mut response, vec!["资料检索索引未完全更新".to_string()])
        .unwrap();

    assert_eq!(
        response["warnings"],
        json!(["项目状态更新失败", "资料检索索引未完全更新"])
    );
}
