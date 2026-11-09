use serde_json::json;

use super::*;

pub(in crate::manga) fn page_planning_fixture() -> (
    crate::media_authoring::PreparedAdaptation,
    Vec<a4n_domain::source_spans::SourceSpan>,
    serde_json::Value,
) {
    use a4n_domain::media_authoring::*;
    let spans = a4n_domain::source_spans::source_spans_for_content(
        9,
        1,
        "林岚发现密信。\n\n周衡关上机房门。",
    );
    assert_eq!(spans.len(), 2);
    let units = spans.iter().enumerate().map(|(index, span)| json!({
        "id":format!("unit-{index}"), "origin":"source", "title":"追查线索", "location":null, "participants":["林岚"],
        "dramatic_question":"信件来自何处", "objective":"查证", "obstacle":"无法解释时间", "entry_state":"疑惑", "exit_state":"发现线索", "source_span_ids":[span.id],
        "beats":[{"id":"clue", "kind":"action", "is_climax":false, "origin":"source", "importance":"required", "action":if index==0 {"检查信件"} else {"本页以外的改编动作"},
        "reaction":null,"dialogue_intent":null,"entry_state":"疑惑","exit_state":"发现线索","source_span_ids":[span.id]}]
    })).collect::<Vec<_>>();
    let plan = json!({"schema_version":"adaptation_plan_v1","target":"manga","page_count":2,"intention":"保留因果","decisions":[],"units":units});
    let record = AuthoringDocumentRecord {
        id: 1,
        project_id: 2,
        source_snapshot_id: 3,
        source_chapter_id: Some(9),
        kind: AuthoringDocumentKind::AdaptationPlan,
        version: 1,
        revision: 1,
        status: AuthoringDocumentStatus::Adopted,
        inputs: AuthoringDocumentInputs {
            sources: vec![],
            video_reference: None,
            dependencies: vec![],
            preprocessing_fingerprint: None,
        },
        document: plan.clone(),
        sha256: "plan-sha".into(),
    };
    let outline = json!({"schema_version":"manga_page_outline_v1","opening_state":"进入通道","pages":[
        {"page_number":1,"goal":"查看信件","source_span_ids":[spans[0].id],"beat_refs":[{"unit_id":"unit-0","beat_id":"clue"}],"exit_state":"取得线索"},
        {"page_number":2,"goal":"继续查证","source_span_ids":[spans[1].id],"beat_refs":[{"unit_id":"unit-1","beat_id":"clue"}],"exit_state":"离开通道"}
    ]});
    (
        crate::media_authoring::PreparedAdaptation {
            record,
            plan: serde_json::from_value(plan).unwrap(),
        },
        spans,
        outline,
    )
}

#[test]
fn page_outline_rejects_wrong_count_unknown_refs_and_missing_required_beats() {
    let (adaptation, spans, valid) = page_planning_fixture();
    parse_manga_page_outline(&valid, &adaptation, &spans).unwrap();
    let mut wrong_count = valid.clone();
    wrong_count["pages"].as_array_mut().unwrap().pop();
    assert!(
        parse_manga_page_outline(&wrong_count, &adaptation, &spans)
            .unwrap_err()
            .to_string()
            .contains("必须生成 2 页")
    );
    let mut unknown_source = valid.clone();
    unknown_source["pages"][0]["source_span_ids"][0] = json!("other-chapter");
    assert!(
        parse_manga_page_outline(&unknown_source, &adaptation, &spans)
            .unwrap_err()
            .to_string()
            .contains("不存在的 source_span_id")
    );
    let mut missing_beat = valid.clone();
    missing_beat["pages"][1]["beat_refs"] = missing_beat["pages"][0]["beat_refs"].clone();
    assert!(
        parse_manga_page_outline(&missing_beat, &adaptation, &spans)
            .unwrap_err()
            .to_string()
            .contains("遗漏关键改编节拍")
    );
    let mut unknown_beat = valid;
    unknown_beat["pages"][0]["beat_refs"][0]["beat_id"] = json!("missing");
    assert!(
        parse_manga_page_outline(&unknown_beat, &adaptation, &spans)
            .unwrap_err()
            .to_string()
            .contains("不存在的改编节拍")
    );
}

#[test]
fn single_page_input_contains_only_allocated_source_and_beats() {
    use crate::manga::pipeline::prompt::MangaPageContentMessages;
    let (adaptation, spans, raw) = page_planning_fixture();
    let outline = parse_manga_page_outline(&raw, &adaptation, &spans).unwrap();
    let markers = crate::source_markers::SourceMarkerMap::from_source_spans(&spans).unwrap();
    let extraction = json!({"source_events":{"events":[
        {"event_id":"first","description":"检查信件","source_span_ids":[spans[0].id]},
        {"event_id":"second","description":"本页以外的事件","source_span_ids":[spans[1].id]}
    ]}, "speech":{"dialogues":[{"source_event_id":"first","content":"查看密信"},{"source_event_id":"second","content":"本页以外的对白"}],"narrations":[]}, "comic_synthesis":{"chapter_summary":"通道中的调查"}});
    let message = MangaPageContentMessages {
        outline: &outline,
        extraction_summary: &extraction,
        adaptation: &adaptation,
        source_spans: &spans,
        chapter_content: "林岚发现密信。\n\n周衡关上机房门。",
        source_markers: &markers,
    }
    .build(0)
    .unwrap();
    let text = serde_json::to_string(&message).unwrap();
    for required in [
        "林岚发现密信",
        "检查信件",
        "查看密信",
        "进入通道",
        "取得线索",
    ] {
        assert!(text.contains(required), "{required}");
    }
    for excluded in [
        "周衡关上机房门",
        "本页以外的事件",
        "本页以外的对白",
        "本页以外的改编动作",
    ] {
        assert!(!text.contains(excluded), "{excluded}");
    }
}

#[test]
fn complete_slots_reject_mismatched_states_without_overwriting_model_data() {
    let (adaptation, spans, raw) = page_planning_fixture();
    let outline = parse_manga_page_outline(&raw, &adaptation, &spans).unwrap();
    let drafts = (1..=2).map(|page| serde_json::from_value(json!({"page_number":page,"summary":"查看信件","action":"调查","reaction":null,"dialogue_intent":null})).unwrap()).collect();
    let mut slots = serde_json::to_value(outline.assemble(drafts).unwrap()).unwrap();
    slots["pages"][1]["content"]["entry_state"] = json!("错误的地点");
    assert!(
        parse_manga_page_slots(&slots, &adaptation, &spans, "test")
            .unwrap_err()
            .to_string()
            .contains("上一页")
    );
    assert_eq!(slots["pages"][1]["content"]["entry_state"], "错误的地点");
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

    assert_eq!(page.page_number, 3);
    assert_eq!(page.image_prompt, "第 3 页完整漫画页");
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
