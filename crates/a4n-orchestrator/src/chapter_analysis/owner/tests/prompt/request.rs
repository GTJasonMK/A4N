use std::collections::HashMap;

use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;

use crate::chapter_analysis::owner::requests::{
    build_chapter_character_state_request, build_chapter_fact_extraction_request,
    build_chapter_foreshadowing_events_request, build_chapter_summary_synthesis_request,
};

struct ChapterSessionPromptRenderer;

#[async_trait]
impl PromptRenderer for ChapterSessionPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        _variables: &HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        Ok(format!("rendered prompt for {template_name}"))
    }
}

#[test]
fn chapter_analysis_stage_user_message_preserves_full_content() {
    let mut content = "甲".repeat(12_050);
    content.push_str("尾部关键线索");

    let message = build_chapter_analysis_stage_user_message(
        "chapter_fact_extraction",
        "抽取事实",
        json!({
            "chapter_title": "第一章",
            "content": content,
            "source_spans": []
        }),
        json!({
            "format": "json_object",
            "required_fields": ["characters"]
        }),
    )
    .expect("message");

    assert!(message.contains("尾部关键线索"));
}

#[test]
fn chapter_fact_prompt_input_omits_system_derived_source_span_fields() {
    let mut ch = sample_chapter();
    ch.content = "林岚走进旧城，墙上刻着隐藏标记。".to_string();
    let spans = source_spans_for_chapters(std::slice::from_ref(&ch));
    let message = build_chapter_analysis_stage_user_message(
        "chapter_fact_extraction",
        "抽取事实",
        json!({
            "chapter_title": "第三章",
            "content": ch.content,
            "source_spans": source_span_prompt_values(&spans)
        }),
        json!({
            "format": "json_object",
            "required_fields": ["characters"]
        }),
    )
    .expect("message");

    assert!(!message.contains(r#""chapter_id""#));
    assert!(!message.contains(r#""chapter_number""#));
    assert!(!message.contains(r#""char_start""#));
    assert!(!message.contains(r#""char_end""#));
    assert!(!message.contains(r#""long_text""#));
    assert!(message.contains(r#""id": "ch3:s1:f1""#));
    assert!(message.contains(r#""text": "林岚走进旧城""#));
}

#[tokio::test]
async fn chapter_analysis_stages_share_one_source_prefix_without_losing_stage_prompts() {
    let chapter = sample_chapter();
    let spans = source_spans_for_chapters(std::slice::from_ref(&chapter));
    let facts = json!({
        "characters": [],
        "event_candidates": [],
        "foreshadowing_candidates": []
    });
    let requests = vec![
        build_chapter_fact_extraction_request(&chapter, &ChapterSessionPromptRenderer, &spans)
            .await
            .expect("facts request"),
        build_chapter_summary_synthesis_request(&chapter, &ChapterSessionPromptRenderer, &facts)
            .await
            .expect("summary request"),
        build_chapter_character_state_request(&chapter, &ChapterSessionPromptRenderer, &facts)
            .await
            .expect("character state request"),
        build_chapter_foreshadowing_events_request(&chapter, &ChapterSessionPromptRenderer, &facts)
            .await
            .expect("foreshadowing request"),
    ];
    let first_system = &requests[0].messages[0].content;
    let first_pack = &requests[0].context_manifest_json.as_ref().unwrap()["packs"][0];

    for request in &requests[1..] {
        assert_eq!(&request.messages[0].content, first_system);
        let pack = &request.context_manifest_json.as_ref().unwrap()["packs"][0];
        assert_eq!(pack["config_hash"], first_pack["config_hash"]);
        assert_eq!(pack["variants"], first_pack["variants"]);
    }
    for stage in [
        "chapter_fact_extraction",
        "chapter_summary_synthesis",
        "chapter_character_state",
        "chapter_foreshadowing_events",
    ] {
        assert!(
            requests
                .iter()
                .any(|request| request.messages[1].content.contains(stage)),
            "missing rendered stage prompt for {stage}"
        );
    }
}
