use a4n_core::models::{
    chapter::chapter,
    character::{
        attribute_event::{self, SOURCE_KIND_AI_SYNC},
        character_state_index, protagonist_behavior_record,
    },
    generation::generation_run_target,
};
use a4n_db::{
    character::{
        CharacterAttributeValueChangeInput, CharacterDbError,
        CharacterSyncAttributeEventRecordInput, CharacterSyncChapterStateRecordInput,
        CharacterSyncPersistenceInput, CharacterSyncWriteStepRecordInput,
        InsertBehaviorRecordInput, add_profile_attribute, apply_character_sync_persistence_record,
        create_profile, create_profile_with_generation_run, profile_by_id, update_initial_profile,
    },
    generation::RecordCompletedGenerationRunInput,
    project::{
        ChapterSourceSnapshot, CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG,
        create_project_record,
    },
};
use chrono::Duration;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde_json::json;

async fn insert_chapter(db: &sea_orm::DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("林岚推开旧仓库的门".to_string()),
        word_count: Set(10),
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

fn generation_run(project_id: i64) -> RecordCompletedGenerationRunInput {
    RecordCompletedGenerationRunInput {
        project_id,
        workflow: "character_profile_sync".to_string(),
        step_key: "sync_character_profile_from_chapters".to_string(),
        title: "角色同步".to_string(),
        model_kind: "llm".to_string(),
        provider: None,
        model: None,
        prompt_template_names_json: None,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: None,
        input_variables_json: None,
        source_refs_json: None,
        media_refs_json: None,
        request_json: None,
        response_summary_json: None,
        targets: vec![],
    }
}

#[tokio::test]
async fn character_sync_preserves_user_attribute_and_commits_other_steps() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "角色冲突保留".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let chapter = insert_chapter(&db, project.id).await;
    let base = json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let created_profile = create_profile(&db, project.id, "林岚", base.clone())
        .await
        .expect("manual profile must be created");
    add_profile_attribute(
        &db,
        CharacterAttributeValueChangeInput {
            profile_id: created_profile.id,
            chapter_number: 1,
            category: "social".to_string(),
            key: "沈默".to_string(),
            value: json!("旧友"),
            change_description: Some("用户确认关系".to_string()),
            cause: "用户设定".to_string(),
            evidence: "用户手工录入".to_string(),
        },
    )
    .await
    .expect("user attribute must be recorded");
    let profile = profile_by_id(&db, created_profile.id)
        .await
        .expect("profile reload must succeed")
        .expect("profile must remain");
    let behavior = json!({
        "chapter_number": 1,
        "behavior_type": "decision",
        "behavior_description": "进入旧仓库",
        "original_text": "林岚推开旧仓库的门",
        "behavior_tags": ["调查"],
        "classification_results": {},
        "related_characters": [],
        "impact_analysis": "开始调查"
    });

    let applied = apply_character_sync_persistence_record(
        &db,
        CharacterSyncPersistenceInput {
            project_id: project.id,
            profile_id: profile.id,
            profile_name: profile.name.clone(),
            expected_profile_updated_at: profile.updated_at,
            expected_profile_last_synced_chapter: profile.last_synced_chapter,
            expected_profile_base_attributes: profile.base_attributes.clone(),
            expected_profile_current_state: profile.current_state.clone(),
            start_chapter: 1,
            target_chapter: 1,
            clear_to_chapter: Some(1),
            selected_chapter_numbers: vec![1],
            source_chapters: vec![ChapterSourceSnapshot::from(&chapter)],
            baseline_state: base,
            write_steps: vec![
                CharacterSyncWriteStepRecordInput::AttributeEvent(
                    CharacterSyncAttributeEventRecordInput {
                        operation: "modify".to_string(),
                        change: CharacterAttributeValueChangeInput {
                            profile_id: profile.id,
                            chapter_number: 1,
                            category: "social".to_string(),
                            key: "沈默".to_string(),
                            value: json!("敌人"),
                            change_description: Some("模型误判关系".to_string()),
                            cause: "章节分析".to_string(),
                            evidence: "冲突证据".to_string(),
                        },
                        from_classification: false,
                    },
                ),
                CharacterSyncWriteStepRecordInput::AttributeEvent(
                    CharacterSyncAttributeEventRecordInput {
                        operation: "add".to_string(),
                        change: CharacterAttributeValueChangeInput {
                            profile_id: profile.id,
                            chapter_number: 1,
                            category: "explicit".to_string(),
                            key: "location".to_string(),
                            value: json!("旧仓库"),
                            change_description: Some("记录当前位置".to_string()),
                            cause: "章节分析".to_string(),
                            evidence: "进入旧仓库".to_string(),
                        },
                        from_classification: false,
                    },
                ),
                CharacterSyncWriteStepRecordInput::Behavior(InsertBehaviorRecordInput {
                    profile_id: profile.id,
                    chapter_number: 1,
                    source_kind: SOURCE_KIND_AI_SYNC.to_string(),
                    behavior_type: "decision".to_string(),
                    behavior_description: "进入旧仓库".to_string(),
                    original_text: "林岚推开旧仓库的门".to_string(),
                    behavior_tags: json!(["调查"]),
                    classification_results: json!({}),
                    related_characters: Some(json!([])),
                    impact_analysis: Some("开始调查".to_string()),
                }),
            ],
            behaviors: vec![behavior],
            classification_suggestions: vec![],
            deletion_candidates: vec![],
            chapter_states: vec![CharacterSyncChapterStateRecordInput {
                chapter_number: 1,
                state_summary: "林岚进入旧仓库调查".to_string(),
                current_location: Some("旧仓库".to_string()),
                current_goal: Some("调查线索".to_string()),
                emotional_state: Some("警觉".to_string()),
            }],
            implicit_review_decisions: json!([]),
            generation_run: generation_run(project.id),
        },
    )
    .await
    .expect("protected conflict must not abort the other writes");

    assert_eq!(applied.write_summary.protected_attributes.len(), 1);
    assert_eq!(applied.write_summary.protected_attributes[0].key, "沈默");
    assert_eq!(applied.write_summary.changes_applied, 1);
    assert_eq!(applied.write_summary.behaviors_recorded, 1);

    let stored = profile_by_id(&db, profile.id)
        .await
        .expect("profile lookup must succeed")
        .expect("profile must remain");
    assert_eq!(
        stored.current_state["social_attributes"]["沈默"],
        json!("旧友")
    );
    assert_eq!(
        stored.current_state["explicit_attributes"]["location"],
        json!("旧仓库")
    );
    let events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .all(&db)
        .await
        .expect("attribute events must load");
    assert_eq!(events.len(), 2);
    let user_event = events
        .iter()
        .find(|event| event.key == "沈默")
        .expect("user relationship event must remain");
    assert_eq!(user_event.source_kind, "user");
    let ai_event = events
        .iter()
        .find(|event| event.key == "location")
        .expect("non-conflicting AI event must be committed");
    assert_eq!(ai_event.source_kind, SOURCE_KIND_AI_SYNC);
}

#[tokio::test]
async fn stale_chapter_source_rejects_entire_character_sync_transaction() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "角色同步正文来源复验".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let chapter = insert_chapter(&db, project.id).await;
    let source = ChapterSourceSnapshot::from(&chapter);
    let rewritten_at = source.updated_at + Duration::seconds(1);
    let base = json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let profile = create_profile(&db, project.id, "林岚", base.clone())
        .await
        .expect("profile must be created");
    let input = CharacterSyncPersistenceInput {
        project_id: project.id,
        profile_id: profile.id,
        profile_name: profile.name.clone(),
        expected_profile_updated_at: profile.updated_at,
        expected_profile_last_synced_chapter: profile.last_synced_chapter,
        expected_profile_base_attributes: profile.base_attributes.clone(),
        expected_profile_current_state: profile.current_state.clone(),
        start_chapter: 1,
        target_chapter: 1,
        clear_to_chapter: Some(1),
        selected_chapter_numbers: vec![1],
        source_chapters: vec![source],
        baseline_state: base,
        write_steps: vec![
            CharacterSyncWriteStepRecordInput::AttributeEvent(
                CharacterSyncAttributeEventRecordInput {
                    operation: "add".to_string(),
                    change: CharacterAttributeValueChangeInput {
                        profile_id: profile.id,
                        chapter_number: 1,
                        category: "explicit".to_string(),
                        key: "location".to_string(),
                        value: json!("旧仓库"),
                        change_description: Some("记录当前位置".to_string()),
                        cause: "章节分析".to_string(),
                        evidence: "旧正文证据".to_string(),
                    },
                    from_classification: false,
                },
            ),
            CharacterSyncWriteStepRecordInput::Behavior(InsertBehaviorRecordInput {
                profile_id: profile.id,
                chapter_number: 1,
                source_kind: SOURCE_KIND_AI_SYNC.to_string(),
                behavior_type: "decision".to_string(),
                behavior_description: "进入旧仓库".to_string(),
                original_text: "林岚推开旧仓库的门".to_string(),
                behavior_tags: json!(["调查"]),
                classification_results: json!({}),
                related_characters: Some(json!([])),
                impact_analysis: Some("开始调查".to_string()),
            }),
        ],
        behaviors: vec![],
        classification_suggestions: vec![],
        deletion_candidates: vec![],
        chapter_states: vec![CharacterSyncChapterStateRecordInput {
            chapter_number: 1,
            state_summary: "林岚进入旧仓库调查".to_string(),
            current_location: Some("旧仓库".to_string()),
            current_goal: Some("调查线索".to_string()),
            emotional_state: Some("警觉".to_string()),
        }],
        implicit_review_decisions: json!([]),
        generation_run: generation_run(project.id),
    };

    let mut changed: chapter::ActiveModel = chapter.into();
    changed.content = Set("用户已经重写了这一章".to_string());
    changed.updated_at = Set(rewritten_at);
    changed
        .update(&db)
        .await
        .expect("chapter rewrite must succeed");

    let error = apply_character_sync_persistence_record(&db, input)
        .await
        .expect_err("stale character sync result must be rejected");
    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, .. }
            if current == "chapter_source_changed"
    ));

    let stored = profile_by_id(&db, profile.id)
        .await
        .expect("profile lookup must succeed")
        .expect("profile must remain");
    assert_eq!(stored.current_state, profile.current_state);
    assert_eq!(stored.last_synced_chapter, profile.last_synced_chapter);
    assert_eq!(stored.updated_at, profile.updated_at);
    assert_eq!(
        attribute_event::Entity::find()
            .filter(attribute_event::Column::ProfileId.eq(profile.id))
            .count(&db)
            .await
            .expect("attribute event count must load"),
        0
    );
    assert_eq!(
        protagonist_behavior_record::Entity::find()
            .filter(protagonist_behavior_record::Column::ProfileId.eq(profile.id))
            .count(&db)
            .await
            .expect("behavior count must load"),
        0
    );
    assert_eq!(
        character_state_index::Entity::find()
            .filter(character_state_index::Column::ProfileId.eq(profile.id))
            .count(&db)
            .await
            .expect("state index count must load"),
        0
    );
}

#[tokio::test]
async fn initial_profile_update_replaces_baseline_and_claims_user_ownership() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "初始角色档案用户接管".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let original = json!({
        "explicit_attributes": {"occupation": "记者"},
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let mut discovery_run = generation_run(project.id);
    discovery_run.workflow = "character_discovery".to_string();
    discovery_run.step_key = "discover_character_profiles".to_string();
    let profile = create_profile_with_generation_run(
        &db,
        project.id,
        "林岚",
        original.clone(),
        discovery_run,
    )
    .await
    .expect("discovered profile must be created");
    assert_eq!(
        generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq("protagonist_profile"))
            .filter(generation_run_target::Column::TargetId.eq(profile.id))
            .count(&db)
            .await
            .expect("generation ownership must load"),
        1
    );

    let replacement = json!({
        "explicit_attributes": {
            "occupation": "调查记者",
            "age": "二十七"
        },
        "implicit_attributes": {"personality": "谨慎"},
        "social_attributes": {"沈默": "旧友"}
    });
    let updated = update_initial_profile(
        &db,
        project.id,
        profile.id,
        original.clone(),
        replacement.clone(),
    )
    .await
    .expect("initial profile must update");

    assert_eq!(updated.base_attributes, replacement);
    assert_eq!(updated.current_state, updated.base_attributes);
    assert_eq!(updated.age.as_deref(), Some("二十七"));
    assert_eq!(updated.personality.as_deref(), Some("谨慎"));
    assert_eq!(
        generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq("protagonist_profile"))
            .filter(generation_run_target::Column::TargetId.eq(profile.id))
            .count(&db)
            .await
            .expect("generation ownership must load"),
        0
    );

    let error = update_initial_profile(
        &db,
        project.id,
        profile.id,
        original,
        updated.base_attributes.clone(),
    )
    .await
    .expect_err("stale initial form must be rejected");
    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, .. }
            if current == "character_profile_source_changed"
    ));
}

#[tokio::test]
async fn initial_profile_update_rejects_real_timeline_when_sync_marker_is_zero() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "初始角色档案时间线门禁".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let base = json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {}
    });
    let profile = create_profile(&db, project.id, "林岚", base)
        .await
        .expect("profile must be created");
    add_profile_attribute(
        &db,
        CharacterAttributeValueChangeInput {
            profile_id: profile.id,
            chapter_number: 1,
            category: "social".to_string(),
            key: "沈默".to_string(),
            value: json!("旧友"),
            change_description: Some("用户记录关系变化".to_string()),
            cause: "正文推进".to_string(),
            evidence: "第一章".to_string(),
        },
    )
    .await
    .expect("timeline event must be recorded");
    let current = profile_by_id(&db, profile.id)
        .await
        .expect("profile lookup must succeed")
        .expect("profile must remain");
    assert_eq!(current.last_synced_chapter, 0);

    let error = update_initial_profile(
        &db,
        project.id,
        profile.id,
        current.current_state.clone(),
        current.current_state,
    )
    .await
    .expect_err("timeline profile must not be rewritten as an initial profile");
    assert!(matches!(
        error,
        CharacterDbError::InvalidState { current, .. }
            if current == "character_profile_timeline_started"
    ));
}
