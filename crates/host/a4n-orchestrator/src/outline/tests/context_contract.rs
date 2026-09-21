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

    let project = novel_project::Model {
        id: 9,
        user_id: 1,
        title: "星潮项目".to_string(),
        description: "项目简介".to_string(),
        project_type: "novel_long".to_string(),
        project_kind: "text".to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 80,
        current_chapter: 0,
        total_chapters: Some(80),
        chapters_per_part: Some(25),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let context = outline_blueprint_context_json(
        &project,
        Some(&blueprint),
        &[character],
        &[relationship],
        json!({
            "character_profiles": [],
            "objects": [],
            "scenes": [],
            "foreshadowing": [],
        }),
        Some(25),
    );
    let text = serde_json::to_string(&context).expect("context should serialize");

    assert_eq!(context["title"], json!("星潮档案"));
    assert_eq!(context["blueprint_available"], json!(true));
    assert!(context.get("formal_materials").is_some());
    assert!(context.get("total_chapters").is_none());
    assert_eq!(context["characters"][0]["motivation"], json!("查明真相"));
    assert_eq!(context["relationships"][0]["strength"], json!(7));
    assert!(!text.contains("\"project_id\""));
    assert!(!text.contains("\"created_at\""));
    assert!(!text.contains("\"updated_at\""));
    assert!(!text.contains("\"avatar_svg\""));
}

#[test]
fn outline_context_without_blueprint_uses_sparse_formal_materials_without_fake_chapter_facts() {
    let now = fixed_time(1);
    let project = novel_project::Model {
        id: 9,
        user_id: 1,
        title: "钟楼旧稿".to_string(),
        description: "从废弃钟楼和一把旧钥匙开始续写".to_string(),
        project_type: "novel_long".to_string(),
        project_kind: "text".to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 30,
        current_chapter: 0,
        total_chapters: None,
        chapters_per_part: Some(10),
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    };
    let profile = protagonist_profile::Model {
        id: 1,
        project_id: 9,
        name: "林岚".to_string(),
        gender: None,
        age: None,
        base_attributes: json!({"social_attributes": {"陈恪": "旧友"}}),
        current_state: json!({"social_attributes": {"陈恪": "旧友"}}),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: Some("一直在寻找失踪的母亲".to_string()),
        personality: None,
        last_synced_chapter: 0,
        created_at: now,
        updated_at: now,
    };
    let object = object_profile::Model {
        id: 2,
        project_id: 9,
        ownership_kind: "user".to_string(),
        name: "旧钥匙".to_string(),
        normalized_name: "旧钥匙".to_string(),
        aliases_json: json!([]),
        object_type: String::new(),
        importance: String::new(),
        visual_description: String::new(),
        material_and_state: String::new(),
        story_usage: "母亲留下的唯一线索".to_string(),
        visual_usage: String::new(),
        generation_usage: String::new(),
        location_context: None,
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!([]),
        image_prompt_seed: String::new(),
        created_at: now,
        updated_at: now,
    };
    let scene = scene_profile::Model {
        id: 3,
        project_id: 9,
        ownership_kind: "user".to_string(),
        name: "废弃钟楼".to_string(),
        normalized_name: "废弃钟楼".to_string(),
        aliases_json: json!([]),
        scene_type: String::new(),
        importance: String::new(),
        parent_scene_id: None,
        base_description: "城北多年封闭的钟楼".to_string(),
        visual_description: String::new(),
        spatial_layout: String::new(),
        atmosphere: String::new(),
        entry_points_json: json!([]),
        rules_json: json!({}),
        current_state_json: json!({}),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!([]),
        image_prompt_seed: String::new(),
        created_at: now,
        updated_at: now,
    };
    let foreshadowing = foreshadowing_index::Model {
        id: 4,
        project_id: 9,
        description: "旧钟会在午夜倒走".to_string(),
        setup_chapter: 2,
        payoff_chapter: None,
        status: "pending".to_string(),
        related_characters: Some(json!(["林岚"])),
        importance: 4,
        created_at: now,
        updated_at: now,
    };
    let formal_materials =
        outline_formal_materials_context_json(&[profile], &[object], &[scene], &[foreshadowing]);
    let context = outline_blueprint_context_json(
        &project,
        None,
        &[],
        &[],
        formal_materials.clone(),
        Some(10),
    );

    assert_eq!(context["blueprint_available"], json!(false));
    assert_eq!(context["project_description"], json!(project.description));
    assert_eq!(
        context["formal_materials"]["character_profiles"][0]["current_state"]["social_attributes"]
            ["陈恪"],
        json!("旧友")
    );
    assert_eq!(
        context["formal_materials"]["objects"][0]["name"],
        json!("旧钥匙")
    );
    assert!(
        context["formal_materials"]["objects"][0]
            .get("observed_chapter_range")
            .is_none()
    );
    assert!(
        context["formal_materials"]["scenes"][0]
            .get("observed_chapter_range")
            .is_none()
    );
    assert!(has_outline_generation_source(
        false,
        "",
        &formal_materials,
        false
    ));
    assert_eq!(
        minimal_blueprint_context(&context)["formal_materials"]["objects"][0]["name"],
        json!("旧钥匙")
    );
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
