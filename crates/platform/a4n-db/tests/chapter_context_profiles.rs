use a4n_db::asset::create_object_profile_record;
use a4n_db::fact_feedback::{FactChangeRecordInput, record_fact_change};
use a4n_db::project::{
    ChapterObjectProfileContext, ChapterSceneProfileContext, CreateProjectRecordInput,
    create_project_record, object_profiles_before_chapter, scene_profiles_before_chapter,
};
use a4n_db::scene_profile::create_scene_profile_record;
use a4n_db::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, merge_profile_evidence_records_batch,
    rebuild_chapter_content_anchor_records,
};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::chapter,
    scene::{scene_occurrence, scene_profile},
};
use a4n_domain::object_profile::ObjectProfileRecordInput;
use a4n_domain::scene_profile::SceneProfileRecordInput;
use a4n_domain::{
    fact_feedback::FactSourceRevision,
    text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;

async fn chapter(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    number: i32,
    content: &str,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        chapter_number: Set(number),
        title: Set(Some(format!("第{number}章"))),
        content: Set(content.to_string()),
        word_count: Set(content.chars().count() as i32),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("chapter must be inserted")
}

fn object_input(name: &str) -> ObjectProfileRecordInput {
    ObjectProfileRecordInput {
        name: name.to_string(),
        normalized_name: name.to_string(),
        aliases_json: json!([]),
        object_type: "prop".to_string(),
        importance: "primary".to_string(),
        visual_description: "旧物件".to_string(),
        material_and_state: "第八章后已损毁".to_string(),
        story_usage: "关键道具".to_string(),
        visual_usage: "特写".to_string(),
        generation_usage: "保持一致".to_string(),
        location_context: None,
        first_seen_chapter: 1,
        last_seen_chapter: 8,
        evidence_json: json!(["作者设定"]),
        image_prompt_seed: "旧物件".to_string(),
    }
}

fn scene_input(name: &str) -> SceneProfileRecordInput {
    SceneProfileRecordInput {
        name: name.to_string(),
        normalized_name: name.to_string(),
        aliases_json: json!([]),
        scene_type: "room".to_string(),
        importance: "primary".to_string(),
        parent_scene_id: None,
        base_description: "一间房间".to_string(),
        visual_description: "木门和书桌".to_string(),
        spatial_layout: "门在南侧".to_string(),
        atmosphere: "安静".to_string(),
        entry_points_json: json!([]),
        rules_json: json!([]),
        first_seen_chapter: 1,
        last_seen_chapter: 8,
        evidence_json: json!(["作者设定"]),
        image_prompt_seed: "房间".to_string(),
    }
}

async fn occurrence(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    scene_profile_id: i64,
    chapter: &chapter::Model,
    summary: &str,
) {
    scene_occurrence::ActiveModel {
        project_id: Set(project_id),
        scene_profile_id: Set(scene_profile_id),
        chapter_id: Set(chapter.id),
        chapter_number: Set(chapter.chapter_number),
        occurrence_order: Set(1),
        timeline_position: Set(None),
        summary: Set(summary.to_string()),
        mood: Set(Some("紧张".to_string())),
        weather_or_lighting: Set(None),
        source_span_ids_json: Set(json!([])),
        evidence_json: Set(json!([])),
        characters_json: Set(json!([])),
        objects_json: Set(json!([])),
        key_events_json: Set(json!([])),
        entry_state_json: Set(json!({})),
        exit_state_json: Set(json!({"状态": summary})),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("scene occurrence must be inserted");
}

#[tokio::test]
async fn chapter_context_uses_only_historical_evidence_and_occurrences() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("chapter context {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("project must be created");
    let first = chapter(&db, project.id, 1, "钥匙出现，主角进入房间").await;
    let third = chapter(&db, project.id, 3, "中间事件").await;
    let eighth = chapter(&db, project.id, 8, "钥匙损毁，房间起火").await;
    let first_anchor = rebuild_chapter_content_anchor_records(&db, project.id, first.id)
        .await
        .expect("first anchors must be created")
        .remove(0);
    let eighth_anchor = rebuild_chapter_content_anchor_records(&db, project.id, eighth.id)
        .await
        .expect("eighth anchors must be created")
        .remove(0);

    let observed_object = create_object_profile_record(&db, project.id, object_input("钥匙"))
        .await
        .expect("object profile must be created");
    merge_profile_evidence_records_batch(
        &db,
        project.id,
        None,
        TextAnchorProfileKind::Object,
        vec![(
            observed_object.id,
            vec![
                TextAnchorEvidenceInput {
                    text_anchor_id: first_anchor.id,
                    evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                },
                TextAnchorEvidenceInput {
                    text_anchor_id: eighth_anchor.id,
                    evidence_role: "state_change".to_string(),
                },
            ],
        )],
    )
    .await
    .expect("object evidence must be bound");
    let authored_object = create_object_profile_record(&db, project.id, object_input("作者道具"))
        .await
        .expect("authored object must be created");
    let mut authored_object_range: object_profile::ActiveModel = authored_object.clone().into();
    authored_object_range.last_seen_chapter = Set(1);
    authored_object_range.update(&db).await.unwrap();
    record_fact_change(
        &db,
        FactChangeRecordInput {
            project_id: project.id,
            workflow_run_id: None,
            material_kind: "object_profile".to_string(),
            target_id: Some(authored_object.id),
            field_path: String::new(),
            operation: "replace".to_string(),
            before: json!({"material_and_state": "铜钥匙完好"}),
            after: json!({"material_and_state": "第八章后已损毁"}),
            sources: vec![FactSourceRevision {
                chapter_id: eighth.id,
                chapter_number: eighth.chapter_number,
                content_revision: "chapter-8-revision".to_string(),
                analysis_hash: "chapter-8-analysis".to_string(),
            }],
            evidence: json!([{"chapter_number": 8, "quote": "钥匙损毁"}]),
            reason: "第八章正文更新物体状态".to_string(),
            status: "applied".to_string(),
        },
    )
    .await
    .expect("object state fact must be recorded");
    let mut future_object_input = object_input("未来道具");
    future_object_input.first_seen_chapter = 8;
    future_object_input.last_seen_chapter = 8;
    let _future_object = create_object_profile_record(&db, project.id, future_object_input)
        .await
        .expect("future authored object must be created");
    let mut ai_only: object_profile::ActiveModel = authored_object.clone().into();
    ai_only.id = sea_orm::NotSet;
    ai_only.name = Set("无历史物体".to_string());
    ai_only.normalized_name = Set("无历史物体".to_string());
    ai_only.ownership_kind = Set("ai_sync".to_string());
    ai_only.evidence_json = Set(json!([]));
    let _ai_only = ai_only
        .insert(&db)
        .await
        .expect("ai object must be inserted");

    let scene = create_scene_profile_record(&db, project.id, scene_input("房间"), "user")
        .await
        .expect("scene profile must be created");
    occurrence(&db, project.id, scene.id, &first, "门口安静").await;
    occurrence(&db, project.id, scene.id, &eighth, "房间起火").await;
    let authored_scene =
        create_scene_profile_record(&db, project.id, scene_input("作者场景"), "user")
            .await
            .expect("authored scene must be created");
    let mut authored_scene_range: scene_profile::ActiveModel =
        scene_profile::Entity::find_by_id(authored_scene.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .into();
    authored_scene_range.last_seen_chapter = Set(1);
    authored_scene_range.update(&db).await.unwrap();
    let mut changed_scene: scene_profile::ActiveModel =
        scene_profile::Entity::find_by_id(authored_scene.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .into();
    changed_scene.visual_description = Set("火灾后的坍塌房间".to_string());
    changed_scene.update(&db).await.unwrap();
    record_fact_change(
        &db,
        FactChangeRecordInput {
            project_id: project.id,
            workflow_run_id: None,
            material_kind: "scene_profile".to_string(),
            target_id: Some(authored_scene.id),
            field_path: String::new(),
            operation: "replace".to_string(),
            before: json!({"visual_description": "木门和书桌"}),
            after: json!({"visual_description": "火灾后的坍塌房间"}),
            sources: vec![FactSourceRevision {
                chapter_id: eighth.id,
                chapter_number: eighth.chapter_number,
                content_revision: "chapter-8-revision".to_string(),
                analysis_hash: "chapter-8-analysis".to_string(),
            }],
            evidence: json!([{"chapter_number": 8, "quote": "房间起火"}]),
            reason: "第八章正文更新场景".to_string(),
            status: "applied".to_string(),
        },
    )
    .await
    .expect("scene state fact must be recorded");
    let mut future_scene_input = scene_input("未来场景");
    future_scene_input.first_seen_chapter = 8;
    future_scene_input.last_seen_chapter = 8;
    let _future_scene = create_scene_profile_record(&db, project.id, future_scene_input, "user")
        .await
        .expect("future authored scene must be created");
    let mut ai_scene: scene_profile::ActiveModel =
        scene_profile::Entity::find_by_id(authored_scene.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .into();
    ai_scene.id = sea_orm::NotSet;
    ai_scene.name = Set("无历史场景".to_string());
    ai_scene.normalized_name = Set("无历史场景".to_string());
    ai_scene.ownership_kind = Set("ai_sync".to_string());
    ai_scene.evidence_json = Set(json!([]));
    ai_scene.current_state_json = Set(json!({}));
    let _ai_scene = ai_scene
        .insert(&db)
        .await
        .expect("ai scene must be inserted");

    let objects_before_third = object_profiles_before_chapter(&db, project.id, 3)
        .await
        .expect("object context must load");
    let observed_before_third = objects_before_third
        .iter()
        .find_map(|context| match context {
            ChapterObjectProfileContext::Observed { name, evidence, .. } if name == "钥匙" => {
                Some(evidence)
            }
            _ => None,
        })
        .expect("historical object evidence must be present");
    assert_eq!(observed_before_third.len(), 1);
    assert_eq!(observed_before_third[0].chapter_number, 1);
    assert!(objects_before_third.iter().all(|context| {
        !matches!(context, ChapterObjectProfileContext::Observed { name, .. } if name == "无历史物体")
    }));
    assert!(objects_before_third.iter().any(|context| {
        matches!(context, ChapterObjectProfileContext::Authored(profile) if profile.name == "作者道具")
    }));
    assert!(objects_before_third.iter().any(|context| {
        matches!(context, ChapterObjectProfileContext::Authored(profile) if profile.name == "作者道具" && profile.material_and_state == "铜钥匙完好")
    }));
    assert!(objects_before_third.iter().all(|context| {
        !matches!(context, ChapterObjectProfileContext::Authored(profile) if profile.name == "未来道具")
    }));

    let objects_before_ninth = object_profiles_before_chapter(&db, project.id, 9)
        .await
        .expect("late object context must load");
    let late_evidence = objects_before_ninth
        .iter()
        .find_map(|context| match context {
            ChapterObjectProfileContext::Observed { name, evidence, .. } if name == "钥匙" => {
                Some(evidence)
            }
            _ => None,
        })
        .expect("late object evidence must be present");
    assert_eq!(late_evidence.last().unwrap().chapter_number, 8);

    let scenes_before_third = scene_profiles_before_chapter(&db, project.id, 3)
        .await
        .expect("scene context must load");
    assert!(scenes_before_third.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Observed { occurrence, .. } if occurrence.chapter_number == 1)
    }));
    assert!(scenes_before_third.iter().all(|context| {
        !matches!(context, ChapterSceneProfileContext::Observed { name, .. } if name == "无历史场景")
    }));
    assert!(scenes_before_third.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Authored(profile) if profile.name == "作者场景")
    }));
    assert!(scenes_before_third.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Authored(profile) if profile.name == "作者场景" && profile.visual_description == "木门和书桌")
    }));
    assert!(scenes_before_third.iter().all(|context| {
        !matches!(context, ChapterSceneProfileContext::Authored(profile) if profile.name == "未来场景")
    }));

    let scenes_before_ninth = scene_profiles_before_chapter(&db, project.id, 9)
        .await
        .expect("late scene context must load");
    assert!(scenes_before_ninth.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Observed { occurrence, .. } if occurrence.chapter_number == 8)
    }));
    assert!(objects_before_ninth.iter().any(|context| {
        matches!(context, ChapterObjectProfileContext::Authored(profile) if profile.name == "未来道具")
    }));
    assert!(objects_before_ninth.iter().any(|context| {
        matches!(context, ChapterObjectProfileContext::Authored(profile) if profile.name == "作者道具" && profile.material_and_state == "第八章后已损毁")
    }));
    assert!(scenes_before_ninth.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Authored(profile) if profile.name == "未来场景")
    }));
    assert!(scenes_before_ninth.iter().any(|context| {
        matches!(context, ChapterSceneProfileContext::Authored(profile) if profile.name == "作者场景" && profile.visual_description == "火灾后的坍塌房间")
    }));

    let _ = third;
}
