use a4n_orchestrator::video::PromptOutputLanguage;

use super::{
    MIN_PROVIDER_VISUAL_PROMPT_CHARS, ProviderPromptTextViolation, provider_prompt_text_violation,
};

#[test]
fn hard_prompt_text_violations_take_priority_over_length() {
    let dictionary_violation = provider_prompt_text_violation(
        r#"{"visual_prompt":"test"}"#,
        "visual_prompt",
        PromptOutputLanguage::ZhCn,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        false,
    );
    assert_eq!(
        dictionary_violation,
        Some(ProviderPromptTextViolation::DictionaryLike)
    );

    let upload_status_violation = provider_prompt_text_violation(
        "没有输入首帧",
        "visual_prompt",
        PromptOutputLanguage::ZhCn,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        false,
    );
    assert_eq!(
        upload_status_violation,
        Some(ProviderPromptTextViolation::UploadStatusTerm(
            "没有输入首帧"
        ))
    );

    let short_natural_violation = provider_prompt_text_violation(
        "镜头缓慢推进",
        "visual_prompt",
        PromptOutputLanguage::ZhCn,
        MIN_PROVIDER_VISUAL_PROMPT_CHARS,
        false,
    );
    assert!(matches!(
        short_natural_violation,
        Some(ProviderPromptTextViolation::TooShort { .. })
    ));
}
