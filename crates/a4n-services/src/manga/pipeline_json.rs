mod fields;
mod page_plan;
mod page_prompts;

pub(crate) use fields::{
    optional_value_field, parse_json_object, required_array_field, required_object_field,
};
pub(crate) use page_plan::{page_plan_page_number, page_plan_pages};
pub(crate) use page_prompts::{
    PageDrawingPrompt, page_drawing_prompt_from_response, validate_page_drawing_prompts,
};

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::error::AppError;
    use serde_json::json;

    #[test]
    fn parses_json_object() {
        let value = parse_json_object(r#"{"pages":[]}"#, "manga_page_drawing_prompt").unwrap();

        assert!(value.is_object());
    }

    #[test]
    fn rejects_markdown_fence() {
        let err = parse_json_object("```json\n{}\n```", "manga_page_drawing_prompt").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn reports_truncated_json() {
        let err = parse_json_object(
            r#"{"dialogues":[{"content":"未闭合"}]"#,
            "manga_speech_selection",
        )
        .unwrap_err();

        match err {
            AppError::InvalidArgument(message) => assert!(message.contains("不完整 JSON")),
            other => panic!("expected invalid argument, got {other:?}"),
        }
    }

    #[test]
    fn reports_empty_json_response() {
        let err = parse_json_object("", "manga_page_planning").unwrap_err();

        match err {
            AppError::InvalidArgument(message) => assert!(message.contains("返回空内容")),
            other => panic!("expected invalid argument, got {other:?}"),
        }
    }

    #[test]
    fn reports_non_json_response_prefix() {
        let err = parse_json_object("我无法生成完整分页", "manga_page_planning").unwrap_err();

        match err {
            AppError::InvalidArgument(message) => assert!(message.contains("不是 JSON 对象")),
            other => panic!("expected invalid argument, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_object_json() {
        let err = parse_json_object("[]", "manga_page_drawing_prompt").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_array_value_when_object_required() {
        let err = fields::require_object_value(json!([]), "manga_page_planning").unwrap_err();

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn validates_page_drawing_prompts() {
        let json = json!({
            "pages": [
                {
                    "page_number": 2,
                    "image_prompt": "完整漫画页提示词",
                    "dialogue_text": "关键对白"
                }
            ]
        });

        let pages = validate_page_drawing_prompts(&json, "manga_page_drawing_prompt").unwrap();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 2);
        assert_eq!(pages[0].image_prompt, "完整漫画页提示词");
        assert_eq!(pages[0].dialogue_text, Some("关键对白"));
    }

    #[test]
    fn rejects_page_plan_duplicate_page_numbers() {
        let err = page_plan_pages(
            &json!({
                "pages": [
                    {"page_number": 1, "event_indices": [0], "content_summary": "第一页"},
                    {"page_number": 1, "event_indices": [1], "content_summary": "重复第一页"}
                ]
            }),
            "manga_page_planning",
        )
        .expect_err("duplicate page numbers should fail");

        assert!(err.to_string().contains("page_number 重复：1"));
    }

    #[test]
    fn rejects_page_plan_invalid_event_indices() {
        let err = page_plan_pages(
            &json!({
                "pages": [
                    {"page_number": 1, "event_indices": ["0"], "content_summary": "字符串事件索引"}
                ]
            }),
            "manga_page_planning",
        )
        .expect_err("event indices must be numeric");

        assert!(
            err.to_string()
                .contains("manga_page_planning.pages[0].event_indices[0]")
        );
        assert!(err.to_string().contains("非负整数"));
    }

    #[test]
    fn rejects_page_plan_derived_root_fields() {
        let err = page_plan_pages(
            &json!({
                "pages": [
                    {"page_number": 1, "event_indices": [0], "content_summary": "第一页"}
                ],
                "total_pages": 1
            }),
            "manga_page_planning",
        )
        .expect_err("derived root fields must fail");

        assert!(err.to_string().contains("未知字段 total_pages"));
    }

    #[test]
    fn extracts_single_page_drawing_prompt() {
        let page = page_drawing_prompt_from_response(
            &json!({
                "pages": [{
                    "page_number": 3,
                    "image_prompt": "第 3 页完整漫画页"
                }]
            }),
            3,
            "manga_page_drawing_prompt.page_3",
        )
        .unwrap();

        assert_eq!(page["page_number"], json!(3));
    }

    #[test]
    fn rejects_page_drawing_prompt_auxiliary_fields() {
        let err = page_drawing_prompt_from_response(
            &json!({
                "pages": [{
                    "page_number": 3,
                    "image_prompt": "第 3 页完整漫画页",
                    "layout": {"panels": 4}
                }]
            }),
            3,
            "manga_page_drawing_prompt.page_3",
        )
        .expect_err("auxiliary fields must stay out of drawing prompt output");

        assert!(err.to_string().contains("未知字段 layout"));
    }
}
