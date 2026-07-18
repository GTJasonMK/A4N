use a4n_core::prompt_language::PromptOutputLanguage;

use super::*;

fn options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: " 电影感写实 ".to_string(),
        prompt_output_language: PromptOutputLanguage::ZhCn,
        prompt_batch_size: 2,
        duration_secs: 8,
        duration_secs_options: vec![4, 8],
        aspect_ratio: " 16:9 ".to_string(),
        reference_image_ids: vec![6, 1, 2],
        video_library_image_ids: vec![9, 4],
        duration_capability: None,
        regenerate_existing: false,
        workflow_run_id: None,
    }
}

#[test]
fn validate_prompt_options_preserves_reference_id_order() {
    let validated = validate_prompt_options(options()).expect("valid options");

    assert_eq!(validated.reference_image_ids, vec![6, 1, 2]);
    assert_eq!(validated.video_library_image_ids, vec![9, 4]);
}

#[test]
fn validate_prompt_options_normalizes_duration_options() {
    let mut input = options();
    input.duration_secs = 8;
    input.duration_secs_options = vec![10, 4, 10];

    let validated = validate_prompt_options(input).expect("valid options");

    assert_eq!(validated.duration_secs_options, vec![4, 8, 10]);
}

#[test]
fn validate_prompt_options_allows_large_batch_size_but_rejects_zero() {
    let mut input = options();
    input.prompt_batch_size = 24;

    let validated = validate_prompt_options(input).expect("batch size follows caller context");

    assert_eq!(validated.prompt_batch_size, 24);

    let mut invalid = options();
    invalid.prompt_batch_size = 0;

    let err = validate_prompt_options(invalid).expect_err("zero batch size is invalid");

    assert!(err.to_string().contains("prompt_batch_size 必须大于等于 1"));
}

#[test]
fn validate_prompt_options_rejects_duplicate_reference_ids() {
    let mut input = options();
    input.reference_image_ids = vec![6, 1, 6];

    let err = validate_prompt_options(input).expect_err("duplicate ids are invalid");

    assert!(
        err.to_string()
            .contains("reference_image_ids 包含重复 ID：6")
    );
}

#[test]
fn validate_prompt_options_rejects_duplicate_video_library_ids() {
    let mut input = options();
    input.video_library_image_ids = vec![9, 4, 9];

    let err = validate_prompt_options(input).expect_err("duplicate ids are invalid");

    assert!(
        err.to_string()
            .contains("video_library_image_ids 包含重复 ID：9")
    );
}

#[test]
fn validate_prompt_options_rejects_unsupported_aspect_ratio() {
    let mut input = options();
    input.aspect_ratio = "2:1".to_string();

    let err = validate_prompt_options(input).expect_err("aspect ratio should be limited");

    assert!(err.to_string().contains("aspect_ratio 无效: 2:1"));
    assert!(err.to_string().contains("21:9、16:9、4:3、1:1、3:4、9:16"));
}
