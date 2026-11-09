use std::collections::HashMap;

use a4n_domain::manga::{
    MangaPageAdaptationV1, MangaPageContentV1, MangaPreprocessingFacts, MangaPreprocessingSpeech,
    MangaPreprocessingStep, MangaPreprocessingVisuals,
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::Value;

use super::*;
use crate::manga::pipeline::prompt::{
    MangaLlmCallInput, MangaPreprocessingMessages, manga_page_storyboard_user_message,
    prepare_manga_llm_call,
};
use crate::source_markers::SourceMarkerMap;
use a4n_llm_context::{
    LlmContextManifest, context_aware_system_user_request, preview_context_aware_chat_request,
};

fn render_manifest(manifest: LlmContextManifest) -> String {
    let mut request = context_aware_system_user_request("system", manifest, None, None).unwrap();
    preview_context_aware_chat_request(&mut request, 1_000_000, 8192).unwrap();
    request.messages[1].content.clone()
}

#[test]
fn preprocessing_summary_preserves_sources_and_has_no_legacy_event_alias() {
    let mut preprocessing: a4n_domain::manga::MangaPreprocessingV3 =
        serde_json::from_value(json!({
            "schema_version": "manga_preprocessing_v3",
            "source_events": {"events": [{
                "event_id": "event-1", "type": "action", "description": "推门",
                "participants": [], "source_span_ids": ["span-1"]
            }]},
            "characters": {}, "speech": {"dialogues": [], "narrations": []},
            "visual_anchors": {"scenes": []}, "items": {"items": []},
            "comic_synthesis": {"chapter_summary": "人物推门进入。", "mood_progression": ["犹豫", "靠近", "进入"]}
        }))
        .unwrap();
    preprocessing.validate().unwrap();
    let summary =
        crate::manga::pipeline::prompt::manga_preprocessing_summary(&preprocessing).unwrap();
    assert_eq!(summary.as_object().unwrap().len(), 6);
    for key in [
        "source_events",
        "characters",
        "speech",
        "visual_anchors",
        "comic_synthesis",
    ] {
        assert_eq!(
            summary[key],
            serde_json::to_value(&preprocessing).unwrap()[key]
        );
    }
    assert_eq!(summary["items"], json!([]));
    assert!(summary.get("events").is_none());
    assert!(summary.get("schema_version").is_none());
    preprocessing.items = json!({});
    assert!(crate::manga::pipeline::prompt::manga_preprocessing_summary(&preprocessing).is_err());
    preprocessing.items = json!({"items": null});
    assert!(crate::manga::pipeline::prompt::manga_preprocessing_summary(&preprocessing).is_err());
}

#[test]
fn preprocessing_requests_share_contracts_and_only_carry_their_real_dependencies() {
    let source = "原文独有标记：她推开木门。";
    let spans = a4n_domain::source_spans::source_spans_for_content(7, 1, source);
    let markers = SourceMarkerMap::from_source_spans(&spans).unwrap();
    let assets = json!({"reference_only_marker": "参考人物资料"});
    let messages = MangaPreprocessingMessages {
        chapter_content: source,
        source_spans: &spans,
        source_markers: &markers,
        reference_assets: &assets,
    };
    let facts = MangaPreprocessingFacts {
        source_events: json!({"events":[{
            "event_id":"event-1", "type":"action", "description":"推门",
            "participants":[], "source_span_ids":[spans[0].id]
        }]}),
        characters: json!({"未具名主角":{
            "name":"未具名主角", "appearance":"外观独有标记", "personality":"原文未说明",
            "role":"protagonist", "gender":"female", "age_description":"原文未说明",
            "source_span_ids":[spans[0].id]
        }}),
    };
    facts.validate().unwrap();
    let speech = MangaPreprocessingSpeech {
        speech: json!({"dialogues":[],"narrations":[]}),
    };
    let visuals = MangaPreprocessingVisuals {
        visual_anchors: json!({"scenes":[]}),
        items: json!({"items":[]}),
    };
    for step in MangaPreprocessingStep::ALL {
        let manifest = messages
            .build(step, Some(&facts), Some(&speech), Some(&visuals))
            .unwrap();
        let contract = manifest
            .packs
            .iter()
            .find_map(|pack| {
                pack.variants[0]
                    .canonical_content_json
                    .get("output_contract")
            })
            .unwrap();
        assert_eq!(contract["output_schema"], step.output_schema());
        assert_eq!(
            crate::manga::pipeline::json::output_schema(step.prompt_name()).unwrap(),
            step.output_schema()
        );
        let mut request =
            context_aware_system_user_request("system", manifest, None, None).unwrap();
        assert!(
            request.max_tokens.is_none(),
            "business steps must inherit the configured output budget"
        );
        preview_context_aware_chat_request(&mut request, 1_000_000, 20_000).unwrap();
        assert_eq!(request.max_tokens, Some(20_000));
        let sent = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            sent.matches("原文独有标记").count(),
            usize::from(step != MangaPreprocessingStep::Synthesis)
        );
        assert_eq!(
            sent.contains("reference_only_marker"),
            step == MangaPreprocessingStep::Facts
        );
        if step == MangaPreprocessingStep::Synthesis {
            assert!(sent.contains("event-1"));
            assert!(sent.contains("未具名主角"));
            assert!(!sent.contains("外观独有标记"));
        }
        assert!(
            !sent.contains(&spans[0].id),
            "source ids must stay encoded as short markers"
        );
    }
    messages
        .build(MangaPreprocessingStep::Facts, None, None, None)
        .unwrap();
    for step in [
        MangaPreprocessingStep::Speech,
        MangaPreprocessingStep::Visuals,
        MangaPreprocessingStep::Synthesis,
    ] {
        assert!(messages.build(step, None, None, None).is_err());
    }
    assert!(
        messages
            .build(
                MangaPreprocessingStep::Synthesis,
                Some(&facts),
                Some(&speech),
                None
            )
            .is_err()
    );
    assert!(
        messages
            .build(
                MangaPreprocessingStep::Synthesis,
                Some(&facts),
                None,
                Some(&visuals)
            )
            .is_err()
    );
}

#[test]
fn manga_pages_reuse_shared_input_without_losing_page_or_output_requirements() {
    let build = |page| {
        build_manga_step_user_message(
        "manga_page_drawing_prompt", &format!("只生成第 {page} 页"),
        json!({"current_page": {"page_number": page}, "page_context": {"cast": ["林晓雨"]}, "reference_assets": ["portrait:1"]}),
        json!({"format": "json_object", "required_fields": ["panels"], "page_number": page}),
    ).unwrap()
    };
    let first = build(1);
    let second = build(2);
    assert_eq!(first.packs, second.packs);
    let rendered = render_manifest(second.clone());
    for required in ["林晓雨", "portrait:1", "panels", "只生成第 2 页"] {
        assert!(rendered.contains(required));
    }
    let payload: Value = serde_json::from_str(
        a4n_prompt::request::json_user_message_payload(&second.dynamic_user_content).unwrap(),
    )
    .unwrap();
    assert_eq!(payload["input"]["current_page"]["page_number"], 2);
    assert_eq!(payload["output_contract"]["page_number"], 2);
    assert!(!second.packs.iter().any(|pack| {
        pack.variants[0]
            .canonical_content_json
            .to_string()
            .contains("current_page")
    }));
}

#[test]
fn manga_page_storyboard_message_requires_null_speaker_for_non_dialogue_layers() {
    let beat_ref = json!({"unit_id": "unit-1", "beat_id": "beat-1"});
    let content: MangaPageContentV1 = serde_json::from_value(json!({
        "summary": "角色发现异常",
        "source_span_ids": [],
        "beat_refs": [beat_ref.clone()],
        "action": "角色停下脚步",
        "reaction": null,
        "dialogue_intent": null,
        "entry_state": "平静",
        "exit_state": "紧张"
    }))
    .expect("page content fixture");
    let adaptation: MangaPageAdaptationV1 = serde_json::from_value(json!({
        "visual_intent": "用留白放大异常感",
        "continuity_requirements": [],
        "retained_details": [],
        "omitted_details": [],
        "beat_refs": [beat_ref]
    }))
    .expect("page adaptation fixture");
    let source_markers = SourceMarkerMap::from_source_spans(&[]).expect("source markers");
    let message = manga_page_storyboard_user_message(
        1,
        1,
        &content,
        &adaptation,
        None,
        None,
        json!({"units": [{"participants": ["林晓雨"]}]}),
        &json!({
            "characters": {"林晓雨": {"name": "林晓雨"}},
            "source_events": {"events": []},
            "speech": {"dialogues": [], "narrations": []}
        }),
        &json!({"character_portraits": []}),
        &[],
        "",
        &source_markers,
    )
    .expect("storyboard message");
    let contract = message
        .packs
        .iter()
        .find_map(|pack| {
            pack.variants[0]
                .canonical_content_json
                .get("output_contract")
        })
        .expect("storyboard output contract");
    assert_eq!(
        contract["text_layer_speaker_rule"],
        "text_layers.kind 为 dialogue 或 thought 时 speaker 必须是 characters 中的稳定角色名；kind 为 narration 或 sfx 时 speaker 必须严格为 JSON null"
    );
    assert_eq!(
        contract["text_layer_placement_rule"],
        "placement 必须严格使用 top_left、top_right、upper_center、center_left、center_right、bottom_left、bottom_right、lower_center；upper_right 等别名无效"
    );
}

struct TestPromptRenderer;

#[test]
fn manga_storyboard_includes_only_current_page_original_speech() {
    let raw_source = "把灯熄了。\n房间暗了下来。\n其他页面的对白";
    let spans = a4n_domain::source_spans::source_spans_for_content(7, 1, raw_source);
    assert_eq!(spans.len(), 3);
    let content: MangaPageContentV1 = serde_json::from_value(json!({
        "summary": "林晓雨要求熄灯", "source_span_ids": [spans[0].id, spans[1].id],
        "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
        "action": "林晓雨指向灯", "reaction": null, "dialogue_intent": "要求熄灯",
        "entry_state": "灯亮着", "exit_state": "灯熄灭"
    }))
    .unwrap();
    let adaptation: MangaPageAdaptationV1 = serde_json::from_value(json!({
        "visual_intent": "熄灯后的停顿", "continuity_requirements": [],
        "retained_details": [], "omitted_details": [], "beat_refs": content.beat_refs
    }))
    .unwrap();
    let extraction = json!({
        "characters": {"林晓雨": {"name": "林晓雨"}},
        "source_events": {"events": [
            {"event_id": "event-1", "source_span_ids": [spans[0].id, spans[1].id]},
            {"event_id": "event-2", "source_span_ids": [spans[2].id]}
        ]},
        "speech": {
            "dialogues": [
                {"source_event_id": "event-1", "content": "把灯熄了。"},
                {"source_event_id": "event-2", "content": "其他页面的对白"}
            ],
            "narrations": [{"source_event_id": "event-1", "content": "房间暗了下来。"}]
        }
    });
    let markers = SourceMarkerMap::from_source_spans(&spans).unwrap();
    let manifest = manga_page_storyboard_user_message(
        1,
        2,
        &content,
        &adaptation,
        None,
        None,
        json!({"units": [{"participants": ["林晓雨"], "beats": [
            {"id": "beat-1", "source_span_ids": [spans[0].id]}
        ]}]}),
        &extraction,
        &json!({"character_portraits": []}),
        &spans,
        raw_source,
        &markers,
    )
    .unwrap();
    let source = manifest
        .packs
        .iter()
        .find_map(|pack| {
            pack.variants[0]
                .canonical_content_json
                .pointer("/input/source_context")
        })
        .expect("page source must be a model context pack");
    assert_eq!(source["speech"]["dialogues"][0]["content"], "把灯熄了。");
    assert!(
        source["source"]
            .as_str()
            .unwrap()
            .contains("#0 把灯熄了。\n#1 房间暗了下来。")
    );
    let sent = render_manifest(manifest);
    assert!(sent.contains("把灯熄了。"));
    assert!(sent.contains("房间暗了下来。"));
    assert!(!sent.contains("其他页面的对白"));
    assert!(!sent.contains("event-2"));

    let mut invalid_content = content.clone();
    invalid_content
        .source_span_ids
        .push("outside-source".into());
    let error = crate::manga::pipeline::prompt::manga_page_storyboard_source_input(
        &invalid_content,
        &extraction,
        &spans,
        raw_source,
        &markers,
    )
    .unwrap_err();
    assert!(error.to_string().contains("来源不在当前章节"));
}

#[async_trait]
impl PromptRenderer for TestPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        _variables: &HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        assert_eq!(template_name, "manga_preprocessing_facts");
        Ok("system prompt".to_string())
    }

    async fn render_prompt_for_call(
        &self,
        _: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.render_prompt(template_name, variables, project_id)
            .await
    }
}

#[tokio::test]
async fn manga_llm_request_sends_dynamic_payload_once() {
    let marker = "unique-manga-payload-marker";
    let variables = HashMap::new();
    let mut prepared = prepare_manga_llm_call(MangaLlmCallInput {
        prompt_renderer: &TestPromptRenderer,
        prompt_name: "manga_preprocessing_facts",
        project_id: 7,
        workflow_run_id: None,
        call_key: "manga:test".to_string(),
        call_order: 1,
        variables: &variables,
        user_content: build_manga_step_user_message(
            "manga_preprocessing_facts",
            "绑定角色",
            json!({"chapter": marker}),
            json!({"format": "json_object"}),
        )
        .unwrap(),
        temperature: 0.3,
    })
    .await
    .expect("manga request should build");
    preview_context_aware_chat_request(&mut prepared.request, 1_000_000, 8192).unwrap();

    let sent = prepared
        .request
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(sent.matches(marker).count(), 1);
    assert!(prepared.request.context_manifest_json.is_some());
}

#[test]
fn build_manga_step_user_message_uses_json_payload() {
    let message = build_manga_step_user_message(
        "manga_preprocessing_speech",
        "选择漫画对白与旁白",
        json!({
            "chapter": {"content": "章节正文"}
        }),
        json!({
            "format": "json_object",
            "required_fields": ["speeches"]
        }),
    )
    .expect("manga step message should build");
    let payload: Value = serde_json::from_str(
        a4n_prompt::request::json_user_message_payload(&message.dynamic_user_content).unwrap(),
    )
    .unwrap();
    assert_eq!(payload["task"]["prompt_name"], "manga_preprocessing_speech");
    assert!(payload["input"].is_object());
    assert!(payload["output_contract"].is_object());
    assert!(render_manifest(message).contains("章节正文"));
}
