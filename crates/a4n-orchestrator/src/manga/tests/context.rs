use super::*;

#[test]
fn manga_late_pipeline_inputs_do_not_repeat_chapter_content() {
    let items_input = manga_item_extraction_input(
        json!({"dialogues": [{"content": "原文对白"}]}),
        json!({"scenes": [{"description": "雨夜"}]}),
    );
    let page_planning = manga_page_planning_input(
        json!({
            "characters": {"陈恪": {"role": "主角"}},
            "events": [{"index": 0, "description": "暗棋初动"}],
            "reference_assets": {"character_portraits": [{"prompt": "重复立绘"}]},
            "speech": {"dialogues": [{"content": "是谁？"}]},
            "visual_anchors": {"scenes": [{"location": "雨夜长街"}]},
            "items": [{"name": "密信"}],
            "comic_synthesis": {"chapter_summary": "陈恪发现线索"}
        }),
        json!({"shooting_script_id": 1, "shooting_script_revision": 1, "script": {}}),
    );

    assert!(items_input.get("chapter").is_none());
    assert!(page_planning.get("chapter").is_none());
    assert!(
        serde_json::to_string(&items_input)
            .unwrap()
            .contains("原文对白")
    );
    let page_planning_text = serde_json::to_string(&page_planning).unwrap();
    assert!(!page_planning_text.contains("暗棋初动"));
    assert!(!page_planning_text.contains("重复立绘"));
    assert!(page_planning_text.contains("陈恪发现线索"));
    assert!(page_planning_text.contains("雨夜长街"));
    assert!(page_planning_text.contains("密信"));
}

#[test]
fn manga_context_manifest_summary_redacts_canonical_content() {
    let manifest = json!({
        "schema_version": 1,
        "packs": [
            {
                "scope_kind": "chapter",
                "pack_kind": "manga_chapter_source",
                "source_refs_json": {"source_span_count": 1},
                "config_hash": "cfg",
                "required": true,
                "stable_prefix": true,
                "variants": [
                    {
                        "compression_level": "full_raw",
                        "canonical_content_json": {
                            "content": "这是一段不应进入调用摘要的完整章节原文"
                        }
                    }
                ]
            }
        ],
        "dynamic_user_content": "当前任务",
        "output_contract": null
    });

    let summary = manga_request_context_manifest_summary(Some(&manifest))
        .expect("summary should build")
        .expect("summary should exist");
    let summary_text = serde_json::to_string(&summary).expect("summary json");

    assert_eq!(summary["pack_count"], json!(1));
    assert_eq!(
        summary["packs"][0]["pack_kind"],
        json!("manga_chapter_source")
    );
    assert!(summary["packs"][0]["variants"][0]["canonical_content_chars"].is_number());
    assert!(
        summary["packs"][0]["variants"][0]["canonical_content_sha256"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(!summary_text.contains("这是一段不应进入调用摘要的完整章节原文"));
    assert!(!summary_text.contains("canonical_content_json"));
}

#[test]
fn manga_page_drawing_prompt_page_input_keeps_only_page_scope_and_refs() {
    let page_plan = json!({
        "page_number": 2,
        "panels": [
            {"panel_number": 1, "participants": ["陈恪", "小宦官"]}
        ]
    });
    let extraction_summary = json!({
        "characters": {
            "陈恪": {"name": "陈恪"},
            "小宦官": {"name": "小宦官"},
            "李建成": {"name": "李建成"}
        },
        "events": [
            {"index": 3, "participants": ["陈恪", "小宦官"]},
            {"index": 8, "participants": ["陈恪", "李建成"]}
        ],
        "reference_assets": {
            "character_portraits": [
                {"character_name": "陈恪", "prompt": "青年书吏"},
                {"character_name": "李建成", "prompt": "太子冠服"}
            ],
            "character_portrait_summary": "陈恪: 青年书吏\n李建成: 太子冠服"
        },
        "speech": {
            "dialogues": [
                {"beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 3}, "content": "今日是六月初三"},
                {"beat_ref": {"scene_index": 2, "shot_index": 1, "beat_index": 1}, "content": "你以为如何？"}
            ],
            "narrations": [
                {"beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 4}, "content": "武德九年六月初三"}
            ]
        },
        "visual_anchors": {
            "scenes": [
                {"beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 3}, {"scene_index": 1, "shot_index": 1, "beat_index": 4}], "location": "书房"},
                {"beat_refs": [{"scene_index": 2, "shot_index": 1, "beat_index": 1}], "location": "显德殿"}
            ]
        },
        "items": [
            {"first_appearance_beat_ref": {"scene_index": 1, "shot_index": 1, "beat_index": 4}, "name": "青铜镜"},
            {"first_appearance_beat_ref": {"scene_index": 2, "shot_index": 1, "beat_index": 1}, "name": "奏章"}
        ],
        "comic_synthesis": {
            "chapter_summary": "陈恪穿越到唐初。",
            "mood_progression": ["迷茫", "震惊"]
        }
    });

    let input = manga_page_drawing_prompt_page_input(&page_plan, &extraction_summary);
    let context = &input["page_context"];

    assert!(context["characters"].get("陈恪").is_some());
    assert!(context["characters"].get("小宦官").is_some());
    assert!(context["characters"].get("李建成").is_none());
    assert!(context.get("extraction_summary").is_none());
    let refs = input["reference_assets"]["character_portraits"]
        .as_array()
        .unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["character_name"], json!("陈恪"));
}
