use super::*;

#[test]
fn outline_blueprint_context_is_compact_and_keeps_story_fields() {
    let now = fixed_time(1);
    let blueprint = novel_blueprint::Model {
        id: 1,
        project_id: 9,
        title: "星潮档案".to_string(),
        genre: "科幻".to_string(),
        style: Some("冷峻".to_string()),
        tone: Some("悬疑".to_string()),
        target_audience: Some("成人".to_string()),
        one_sentence_summary: Some("调查星潮真相".to_string()),
        full_synopsis: Some("完整主线".to_string()),
        world_setting: Some(json!({"premise": "星潮复苏"})),
        current_version: "1.0.0".to_string(),
        needs_part_outlines: true,
        total_chapters: Some(80),
        chapters_per_part: Some(20),
        avatar_svg: Some("<svg />".to_string()),
        avatar_animal: None,
        created_at: now,
        updated_at: now,
    };
    let character = blueprint_character::Model {
        id: 2,
        project_id: 9,
        name: "林烬".to_string(),
        character_type: "protagonist".to_string(),
        gender: None,
        age: None,
        appearance: Some("灰色制服".to_string()),
        personality: Some("克制".to_string()),
        background: None,
        motivation: Some("查明真相".to_string()),
        arc_summary: None,
        relationship_to_protagonist: None,
        arc_stages: None,
        created_at: now,
        updated_at: now,
    };
    let relationship = blueprint_relationship::Model {
        id: 3,
        project_id: 9,
        relationship_type: "rival".to_string(),
        character_a: "林烬".to_string(),
        character_b: "沈澜".to_string(),
        description: Some("互相试探".to_string()),
        strength: Some(7),
        evolution: None,
        created_at: now,
    };

    let context =
        outline_blueprint_context_json(&blueprint, &[character], &[relationship], Some(25));
    let text = serde_json::to_string(&context).expect("context should serialize");

    assert_eq!(context["title"], json!("星潮档案"));
    assert!(context.get("total_chapters").is_none());
    assert_eq!(context["characters"][0]["motivation"], json!("查明真相"));
    assert_eq!(context["relationships"][0]["strength"], json!(7));
    assert!(!text.contains("\"project_id\""));
    assert!(!text.contains("\"created_at\""));
    assert!(!text.contains("\"updated_at\""));
    assert!(!text.contains("\"avatar_svg\""));
}

#[test]
fn chapter_outline_context_includes_foreshadowing_fields() {
    let now = fixed_time(1);
    let outline = chapter_outline::Model {
        id: 1,
        project_id: 9,
        part_id: Some(2),
        chapter_number: 3,
        title: Some("暗线".to_string()),
        summary: Some("主角发现线索".to_string()),
        beats: Some(json!(["发现"])),
        key_scenes: Some(json!(["密室"])),
        pacing_score: Some(0.7),
        foreshadowing_setup: Some(json!(["旧信封"])),
        foreshadowing_payoff: Some(json!(["第1章脚印"])),
        characters_involved: Some(json!(["林烬"])),
        word_count_target: Some(4500),
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    };

    let context = chapter_outline_context_json(&outline);

    assert_eq!(context["foreshadowing_setup"], json!(["旧信封"]));
    assert_eq!(context["foreshadowing_payoff"], json!(["第1章脚印"]));
    assert!(context.get("status").is_none());
    assert!(context.get("updated_at").is_none());
}
