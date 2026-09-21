#[test]
fn parse_chapter_analysis_stage_json_rejects_fenced_json() {
    let err = crate::structured_llm::parse_strict_json_response::<serde_json::Value>(
        "```json\n{}\n```",
        "chapter_fact_extraction",
    )
    .unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("严格 JSON 对象"))
    );
}
