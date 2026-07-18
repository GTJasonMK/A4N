#[test]
fn normalize_analysis_rejects_non_object_payload() {
    let err = normalize_analysis(json!([]), &sample_chapter()).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("JSON 对象")));
}

#[test]
fn normalize_analysis_rejects_missing_required_field() {
    let mut payload = sample_analysis();
    payload.as_object_mut().unwrap().remove("character_states");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("character_states"))
    );
}

#[test]
fn normalize_analysis_rejects_unknown_top_level_field() {
    let mut payload = sample_analysis();
    payload["legacy_summary"] = json!("旧字段");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 legacy_summary"))
    );
}

#[test]
fn normalize_analysis_rejects_unknown_nested_field() {
    let mut payload = sample_analysis();
    payload["character_states"]["林岚"]["emotion"] = json!("旧情绪字段");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(
        matches!(err, AppError::InvalidArgument(message) if message.contains("未知字段 emotion"))
    );
}

#[test]
fn normalize_analysis_rejects_default_placeholder_string() {
    let mut payload = sample_analysis();
    payload["metadata"]["tone"] = json!("");

    let err = normalize_analysis(payload, &sample_chapter()).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("非空字符串")));
}
