#[test]
fn chapter_state_drafts_cover_every_selected_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"] = json!([
        {
            "chapter_number": 2,
            "state_summary": "抵达镇外",
            "current_location": "镇外",
            "current_goal": "入镇调查",
            "emotional_state": "警惕"
        },
        {
            "chapter_number": 1,
            "state_summary": "离开山门",
            "current_location": "山门",
            "current_goal": "赶往落霞镇",
            "emotional_state": "坚定"
        }
    ]);

    let drafts =
        chapter_state_drafts(&value, &[1, 2]).expect("states should be ordered by input chapters");
    assert_eq!(drafts[0].chapter_number, 1);
    assert_eq!(drafts[0].state_summary, "离开山门");
    assert_eq!(drafts[1].chapter_number, 2);
    assert_eq!(drafts[1].current_location.as_deref(), Some("镇外"));
}

#[test]
fn chapter_state_drafts_rejects_missing_chapter() {
    let value = canonical_character_sync_value();

    let err = chapter_state_drafts(&value, &[1, 2])
        .expect_err("missing selected chapter state must fail");
    assert!(err.contains("缺少章节状态"));
    assert!(err.contains("第 2 章"));
}

#[test]
fn chapter_state_drafts_rejects_duplicate_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"] = json!([
        {
            "chapter_number": 1,
            "state_summary": "第一次状态",
            "current_location": null,
            "current_goal": null,
            "emotional_state": null
        },
        {
            "chapter_number": 1,
            "state_summary": "重复状态",
            "current_location": null,
            "current_goal": null,
            "emotional_state": null
        }
    ]);

    let err = chapter_state_drafts(&value, &[1]).expect_err("duplicate chapter state must fail");
    assert!(err.contains("重复章节"));
}

#[test]
fn chapter_state_drafts_rejects_out_of_range_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"][0]["chapter_number"] = json!(3);

    let err = chapter_state_drafts(&value, &[1, 2]).expect_err("out-of-range chapter must fail");
    assert!(err.contains("超出本次同步章节"));
    assert!(err.contains("第 3 章"));
}
