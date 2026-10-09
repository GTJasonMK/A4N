use super::*;
use a4n_domain::manga::MangaPageV4;

fn manga_page_with_participants(participants: &[&str]) -> MangaPageV4 {
    serde_json::from_value(json!({
        "page_number": 2,
        "beat_refs": [],
        "role": "setup",
        "pacing": "medium",
        "importance": "normal",
        "reading_flow": "top_to_bottom_left_to_right",
        "page_goal": "推进场景",
        "visual_thesis": "角色对峙",
        "entry_state": "平静",
        "exit_state": "紧张",
        "dominant_panel_number": 1,
        "panels": [{
            "panel_number": 1,
            "beat_refs": [],
            "function": "dialogue",
            "size": "dominant",
            "shot_scale": "medium",
            "camera_angle": "eye_level",
            "participants": participants,
            "composition": "双人构图",
            "observable_action": "两人交谈",
            "reaction": null,
            "spatial_relationship": "面对面",
            "continuity_from_previous": null,
            "text_layers": []
        }]
    }))
    .expect("valid manga page fixture")
}

#[test]
fn manga_page_slots_input_uses_preprocessing_events() {
    let page_slots = manga_page_slots_input(
        json!({
            "characters": {"陈恪": {"role": "主角"}},
            "source_events": {"events": [{"event_id": "event-1", "description": "陈恪发现线索"}]},
            "events": [{"index": 0, "description": "暗棋初动"}],
            "reference_assets": {"character_portraits": [{"prompt": "重复立绘"}]},
            "speech": {"dialogues": [{"content": "是谁？"}]},
            "visual_anchors": {"scenes": [{"location": "雨夜长街"}]},
            "items": [{"name": "密信"}],
            "comic_synthesis": {
                "chapter_summary": "陈恪发现线索。",
                "mood_progression": ["疑点浮现", "压力加深", "线索揭开"]
            }
        }),
        json!({"page_count": 8, "intention": "保留关键因果", "units": []}),
    )
    .expect("valid page slots input");

    assert!(page_slots.get("chapter").is_none());
    let page_slots_text = serde_json::to_string(&page_slots).unwrap();
    assert!(!page_slots_text.contains("暗棋初动"));
    assert!(!page_slots_text.contains("重复立绘"));
    assert!(page_slots_text.contains("陈恪发现线索"));
    assert!(page_slots_text.contains("雨夜长街"));
    assert!(page_slots_text.contains("密信"));
}

#[test]
fn manga_page_drawing_prompt_page_input_keeps_only_page_scope_and_refs() {
    let page_plan = manga_page_with_participants(&["陈恪", "小宦官"]);
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
            "mood_progression": ["迷茫", "震惊", "寻找答案"]
        }
    });
    let reference_assets = json!({
        "character_portraits": [
            {"character_name": "陈恪", "prompt": "青年书吏"},
            {"character_name": "李建成", "prompt": "太子冠服"}
        ]
    });

    let input = manga_page_drawing_prompt_page_input(
        &page_plan,
        &extraction_summary,
        &reference_assets,
        &[],
        &[],
        &[],
    )
    .expect("page context should build");
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

#[test]
fn manga_page_drawing_context_rejects_malformed_reference_metadata() {
    let page_plan = manga_page_with_participants(&["陈恪"]);
    let extraction_summary = json!({
        "characters": {"陈恪": {"name": "陈恪"}},
    });
    let reference_assets = json!({
        "character_portraits": [{"character_name": "陈恪"}]
    });

    let error = manga_page_drawing_prompt_page_input(
        &page_plan,
        &extraction_summary,
        &reference_assets,
        &[],
        &[],
        &[],
    )
    .expect_err("missing portrait prompt must fail");

    assert!(error.to_string().contains("character_portraits[0].prompt"));
}
