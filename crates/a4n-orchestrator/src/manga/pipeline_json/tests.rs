use serde_json::json;

use super::*;

#[test]
fn validates_page_drawing_prompts() {
    let json = json!({
        "pages": [
            {
                "schema_version": "manga_page_drawing_prompt_v2",
                "page_number": 2,
                "image_prompt": "完整漫画页提示词"
            }
        ]
    });

    let pages = validate_page_drawing_prompts(&json, "manga_page_drawing_prompt").unwrap();

    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 2);
    assert_eq!(pages[0].image_prompt, "完整漫画页提示词");
}

#[test]
fn extracts_single_page_drawing_prompt() {
    let page = page_drawing_prompt_from_response(
        &json!({
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": 3,
            "image_prompt": "第 3 页完整漫画页"
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
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": 3,
            "image_prompt": "第 3 页完整漫画页",
            "layout": {"panels": 4}
        }),
        3,
        "manga_page_drawing_prompt.page_3",
    )
    .expect_err("auxiliary fields must stay out of drawing prompt output");

    assert!(err.to_string().contains("layout"));
}
