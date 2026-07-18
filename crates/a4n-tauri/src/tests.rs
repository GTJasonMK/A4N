use crate::environment::validate_env_value;

#[test]
fn early_log_dir_env_value_must_not_be_empty() {
    let err = validate_env_value("A4N_LOG_DIR", "  ".into()).unwrap_err();
    assert!(err.contains("A4N_LOG_DIR 不能为空"));
}

#[test]
fn early_log_dir_env_value_is_trimmed() {
    let value = validate_env_value("A4N_LOG_DIR", " /tmp/a4n-logs ".into()).unwrap();
    assert_eq!(value, "/tmp/a4n-logs");
}
