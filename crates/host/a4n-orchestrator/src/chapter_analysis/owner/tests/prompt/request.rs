use std::collections::HashMap;

use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;

use crate::chapter_analysis::owner::requests::{
    build_chapter_analysis_initial_request, build_chapter_character_state_request,
    build_chapter_fact_extraction_request, build_chapter_foreshadowing_events_request,
    build_chapter_summary_synthesis_request,
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

#[tokio::test]
async fn chapter_analysis_preview_builds_only_the_exact_initial_request() {
    let step =
        build_chapter_analysis_initial_request(&sample_chapter(), &ChapterSessionPromptRenderer)
            .await
            .expect("initial request");

    assert_eq!(step.key, "chapter_fact_extraction");
    assert_eq!(step.prompt_name, "chapter_fact_extraction");
    assert!(!step.request.messages[1].content.contains("preview_note"));
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
    let payload: serde_json::Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&message).unwrap())
            .unwrap();
    assert_eq!(payload["input"]["source_spans"][0]["id"], "ch3:s1:f1");
    assert_eq!(payload["input"]["source_spans"][0]["text"], "林岚走进旧城");
}

#[tokio::test]
async fn chapter_analysis_stages_share_one_source_prefix_without_losing_stage_prompts() {
    let chapter = sample_chapter();
    let spans = source_spans_for_chapters(std::slice::from_ref(&chapter));
    let facts = json!({
        "characters": [],
        "event_candidates": [],
        "foreshadowing_candidates": [],
        "world_facts": []
    });
    let pending_foreshadowing = json!([{
        "id": "7",
        "description": "门后的脚步声",
        "setup_chapter": 1,
        "payoff_chapter": null,
        "status": "pending",
        "importance": 3,
        "related_characters": []
    }]);
    let requests = [
        build_chapter_fact_extraction_request(&chapter, &ChapterSessionPromptRenderer, &spans)
            .await
            .expect("facts request"),
        build_chapter_summary_synthesis_request(&chapter, &ChapterSessionPromptRenderer, &facts)
            .await
            .expect("summary request"),
        build_chapter_character_state_request(&chapter, &ChapterSessionPromptRenderer, &facts)
            .await
            .expect("character state request"),
        build_chapter_foreshadowing_events_request(
            &chapter,
            &ChapterSessionPromptRenderer,
            &facts,
            &pending_foreshadowing,
        )
        .await
        .expect("foreshadowing request"),
    ];
    let first_system = &requests[0].messages[0].content;
    let first_pack = &requests[0].context_manifest_json.as_ref().unwrap()["packs"][0];
    let variants = first_pack["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 1, "不能选择无正文的压缩变体");
    let source = &variants[0]["canonical_content_json"];
    assert!(source.get("source_spans").is_none(), "正文只发送一份");
    let mut restored = source["content"].as_str().unwrap().to_string();
    for span in &spans {
        restored = restored.replace(&format!("⟦{}⟧", span.id), "");
    }
    assert_eq!(restored, chapter.content);

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
    assert!(
        requests[3].messages[1]
            .content
            .contains("pending_foreshadowing")
    );
    assert!(requests[3].messages[1].content.contains("门后的脚步声"));
}
