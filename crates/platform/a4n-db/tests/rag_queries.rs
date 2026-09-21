use a4n_db::{
    project::{CreateProjectRecordInput, create_project_record},
    rag::{
        chapter_outlines_for_rag_ingest, foreshadowing_items_for_rag_ingest,
        pending_foreshadowing_search_rows, scene_occurrences_for_rag_ingest,
        scene_profiles_for_rag_ingest, scene_transitions_for_rag_ingest,
    },
};
use a4n_db_models::models::{
    chapter::{chapter, chapter_outline},
    character::foreshadowing_index,
    scene::{scene_occurrence, scene_profile, scene_transition},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

#[tokio::test]
async fn chapter_outlines_for_rag_ingest_returns_only_record_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("RAG 章纲查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let outline = chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(Some(1)),
        chapter_number: Set(3),
        title: Set(Some("入城".to_string())),
        summary: Set(Some("主角进入边境城。".to_string())),
        beats: Set(Some(json!(["抵达"]))),
        key_scenes: Set(Some(json!(["城门"]))),
        pacing_score: Set(Some(0.8)),
        foreshadowing_setup: Set(Some(json!({"large": "omitted"}))),
        foreshadowing_payoff: Set(Some(json!({"large": "omitted"}))),
        characters_involved: Set(Some(json!(["林舟"]))),
        word_count_target: Set(Some(3000)),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test outline must be created");

    let rows = chapter_outlines_for_rag_ingest(&db, project.id)
        .await
        .expect("RAG outline projection must load");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, outline.id);
    assert_eq!(rows[0].chapter_number, 3);
    assert_eq!(rows[0].title.as_deref(), Some("入城"));
    assert_eq!(rows[0].summary.as_deref(), Some("主角进入边境城。"));
    assert_eq!(rows[0].beats, Some(json!(["抵达"])));
    assert_eq!(rows[0].key_scenes, Some(json!(["城门"])));
}

#[tokio::test]
async fn scene_profiles_for_rag_ingest_returns_record_fields_in_order() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("RAG 场景查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let profile = scene_profile::ActiveModel {
        project_id: Set(project.id),
        ownership_kind: Set("user".to_string()),
        name: Set("旧书房".to_string()),
        normalized_name: Set("旧书房".to_string()),
        aliases_json: Set(json!(["书房"])),
        scene_type: Set("interior".to_string()),
        importance: Set("primary".to_string()),
        parent_scene_id: Set(None),
        base_description: Set("一间旧书房".to_string()),
        visual_description: Set("落满灰尘".to_string()),
        spatial_layout: Set("书架环绕".to_string()),
        atmosphere: Set("安静".to_string()),
        entry_points_json: Set(json!(["东门"])),
        rules_json: Set(json!(["禁止喧哗"])),
        current_state_json: Set(json!({"灯": "亮"})),
        first_seen_chapter: Set(1),
        last_seen_chapter: Set(3),
        evidence_json: Set(json!([{"chapter": 1}])),
        image_prompt_seed: Set("旧书房".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test scene profile must be created");

    let rows = scene_profiles_for_rag_ingest(&db, project.id)
        .await
        .expect("RAG scene profile projection must load");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, profile.id);
    assert_eq!(rows[0].name, "旧书房");
    assert_eq!(rows[0].aliases_json, json!(["书房"]));
    assert_eq!(rows[0].scene_type, "interior");
    assert_eq!(rows[0].first_seen_chapter, 1);
    assert_eq!(rows[0].last_seen_chapter, 3);
    assert_eq!(rows[0].evidence_json, json!([{"chapter": 1}]));

    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("进入旧书房".to_string()),
        word_count: Set(5),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        summary: Set(Some("进入场景".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test chapter must be created");
    let occurrence = scene_occurrence::ActiveModel {
        project_id: Set(project.id),
        scene_profile_id: Set(profile.id),
        chapter_id: Set(chapter.id),
        chapter_number: Set(1),
        occurrence_order: Set(1),
        timeline_position: Set(Some("夜晚".to_string())),
        summary: Set("主角进入旧书房".to_string()),
        mood: Set(Some("紧张".to_string())),
        weather_or_lighting: Set(Some("烛光".to_string())),
        source_span_ids_json: Set(json!([1])),
        evidence_json: Set(json!([{"chapter": 1}])),
        characters_json: Set(json!(["林舟"])),
        objects_json: Set(json!(["钥匙"])),
        key_events_json: Set(json!(["发现密室"])),
        entry_state_json: Set(json!({"门": "关闭"})),
        exit_state_json: Set(json!({"门": "打开"})),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test scene occurrence must be created");
    let occurrence_rows = scene_occurrences_for_rag_ingest(&db, project.id)
        .await
        .expect("RAG scene occurrence projection must load");
    assert_eq!(occurrence_rows.len(), 1);
    assert_eq!(occurrence_rows[0].id, occurrence.id);
    assert_eq!(occurrence_rows[0].scene_profile_id, profile.id);
    assert_eq!(occurrence_rows[0].summary, "主角进入旧书房");
    assert_eq!(occurrence_rows[0].characters_json, json!(["林舟"]));

    let transition = scene_transition::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        chapter_number: Set(1),
        from_scene_profile_id: Set(Some(profile.id)),
        to_scene_profile_id: Set(profile.id),
        occurrence_id: Set(Some(occurrence.id)),
        transition_order: Set(1),
        movement_type: Set("进入".to_string()),
        route_description: Set("从门口走到书桌".to_string()),
        time_cost: Set(Some("一分钟".to_string())),
        distance_hint: Set(Some("五米".to_string())),
        participants_json: Set(json!(["林舟"])),
        object_refs_json: Set(json!(["钥匙"])),
        source_span_ids_json: Set(json!([1])),
        evidence_json: Set(json!([{"chapter": 1}])),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test scene transition must be created");
    let transition_rows = scene_transitions_for_rag_ingest(&db, project.id)
        .await
        .expect("RAG scene transition projection must load");
    assert_eq!(transition_rows.len(), 1);
    assert_eq!(transition_rows[0].id, transition.id);
    assert_eq!(transition_rows[0].from_scene_profile_id, Some(profile.id));
    assert_eq!(transition_rows[0].to_scene_profile_id, profile.id);
    assert_eq!(transition_rows[0].route_description, "从门口走到书桌");
}

#[tokio::test]
async fn pending_foreshadowing_search_rows_keep_filter_order_and_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("RAG 伏笔查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    for (description, setup_chapter, status, importance, payoff_chapter) in [
        ("早期重要伏笔", 2, "pending", 5, None),
        ("晚于边界", 8, "pending", 5, None),
        ("已回收", 1, "resolved", 5, Some(3)),
        ("早期普通伏笔", 1, "pending", 2, None),
    ] {
        foreshadowing_index::ActiveModel {
            project_id: Set(project.id),
            description: Set(description.to_string()),
            setup_chapter: Set(setup_chapter),
            payoff_chapter: Set(payoff_chapter),
            status: Set(status.to_string()),
            related_characters: Set(Some(json!(["林岚"]))),
            importance: Set(importance),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("foreshadowing must be inserted");
    }

    let rows = pending_foreshadowing_search_rows(&db, project.id, Some(5), 10)
        .await
        .expect("search projection must load");
    assert_eq!(
        rows.iter()
            .map(|row| row.description.as_str())
            .collect::<Vec<_>>(),
        ["早期重要伏笔", "早期普通伏笔"]
    );
    assert_eq!(rows[0].importance, 5);
    assert_eq!(rows[0].related_characters, Some(json!(["林岚"])));
}

#[tokio::test]
async fn foreshadowing_items_for_rag_ingest_returns_only_record_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("RAG 伏笔窄投影 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let row = foreshadowing_index::ActiveModel {
        project_id: Set(project.id),
        description: Set("门后的脚步声".to_string()),
        setup_chapter: Set(2),
        payoff_chapter: Set(Some(8)),
        status: Set("resolved".to_string()),
        related_characters: Set(Some(json!(["林岚"]))),
        importance: Set(4),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("foreshadowing must be inserted");

    let rows = foreshadowing_items_for_rag_ingest(&db, project.id)
        .await
        .expect("RAG foreshadowing projection must load");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, row.id);
    assert_eq!(rows[0].description, "门后的脚步声");
    assert_eq!(rows[0].setup_chapter, 2);
    assert_eq!(rows[0].payoff_chapter, Some(8));
    assert_eq!(rows[0].status, "resolved");
    assert_eq!(rows[0].related_characters, Some(json!(["林岚"])));
    assert_eq!(rows[0].importance, 4);
}
