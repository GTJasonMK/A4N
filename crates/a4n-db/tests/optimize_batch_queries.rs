use a4n_db::optimize::{
    OptimizationContextReadInput, OptimizationPatchCreateInput, OptimizationPatchSetCreateInput,
    OptimizationSessionCreateInput, OptimizationTurnCreateInput, OptimizationTurnEventAppendInput,
    append_optimization_turn_event, create_optimization_patch_set, create_optimization_session,
    create_optimization_turn, optimization_context_rows, optimization_patch_set_patches_for_sets,
    optimization_patch_sets, optimization_turn_events_for_turns,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{
    chapter::{chapter, chapter_outline},
    character::foreshadowing_index,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

#[tokio::test]
async fn optimization_context_reads_outline_projection_without_contract_loss() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("窄大纲查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let outline = chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章大纲".to_string())),
        summary: Set(Some("大纲摘要".to_string())),
        beats: Set(Some(json!(["beat"]))),
        key_scenes: Set(Some(json!(["scene"]))),
        pacing_score: Set(Some(8.5)),
        foreshadowing_setup: Set(Some(json!(["setup"]))),
        foreshadowing_payoff: Set(Some(json!(["payoff"]))),
        characters_involved: Set(Some(json!(["主角"]))),
        word_count_target: Set(Some(1200)),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test outline must be created");
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(Some(outline.id)),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test chapter must be created");
    foreshadowing_index::ActiveModel {
        project_id: Set(project.id),
        description: Set("伏笔描述".to_string()),
        setup_chapter: Set(1),
        payoff_chapter: Set(None),
        status: Set("pending".to_string()),
        related_characters: Set(Some(json!(["主角"]))),
        importance: Set(4),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test foreshadowing must be created");

    let rows = optimization_context_rows(
        &db,
        OptimizationContextReadInput {
            project_id: project.id,
            chapter_id: chapter.id,
            read_project_detail: false,
            read_blueprint: false,
            read_outlines: true,
            read_character_profiles: false,
            read_foreshadowing: true,
        },
    )
    .await
    .expect("optimization context query must succeed")
    .expect("chapter must exist");

    let current = rows
        .current_chapter_outline
        .expect("current outline must be loaded");
    assert_eq!(current.chapter_number, 1);
    assert_eq!(current.beats, Some(json!(["beat"])));
    assert_eq!(rows.chapter_outlines.len(), 1);
    assert_eq!(rows.chapter_outlines[0].word_count_target, Some(1200));
    assert_eq!(rows.foreshadowing_index.len(), 1);
    assert_eq!(rows.foreshadowing_index[0].description, "伏笔描述");
    assert_eq!(rows.foreshadowing_index[0].importance, 4);
}

#[tokio::test]
async fn optimization_turn_events_for_turns_returns_grouped_ordered_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: format!("批量事件查询 {}", uuid::Uuid::new_v4()),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created");
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("test chapter must be created");
    let session = create_optimization_session(
        &db,
        OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("批量事件".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: None,
        },
    )
    .await
    .expect("test session must be created");
    let first = create_optimization_turn(
        &db,
        OptimizationTurnCreateInput {
            session_id: session.id,
            user_text: "第一轮".to_string(),
            status: "completed".to_string(),
            permissions_snapshot: None,
        },
    )
    .await
    .expect("first turn must be created");
    let second = create_optimization_turn(
        &db,
        OptimizationTurnCreateInput {
            session_id: session.id,
            user_text: "第二轮".to_string(),
            status: "completed".to_string(),
            permissions_snapshot: None,
        },
    )
    .await
    .expect("second turn must be created");

    let event_input = |turn_id, seq| OptimizationTurnEventAppendInput {
        turn_id,
        kind: format!("kind-{seq}"),
        status: "completed".to_string(),
        public_text: format!("event-{seq}"),
        payload: None,
    };
    let first_event = append_optimization_turn_event(&db, event_input(first.id, 1))
        .await
        .expect("first event must be created");
    let second_first_event = append_optimization_turn_event(&db, event_input(second.id, 1))
        .await
        .expect("second turn event must be created");
    let second_event = append_optimization_turn_event(&db, event_input(first.id, 2))
        .await
        .expect("second first-turn event must be created");

    let rows = optimization_turn_events_for_turns(&db, &[second.id, first.id, second.id])
        .await
        .expect("batch event query must succeed");

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, first_event.id);
    assert_eq!(rows[1].id, second_event.id);
    assert_eq!(rows[2].id, second_first_event.id);
    assert_eq!(rows[0].seq, 1);
    assert_eq!(rows[1].seq, 2);
    assert_eq!(rows[2].seq, 1);

    let patch_set_input = |turn_id, patch_id| OptimizationPatchSetCreateInput {
        turn_id,
        session_id: session.id,
        chapter_id: chapter.id,
        base_content_hash: format!("hash-{patch_id}"),
        status: "pending".to_string(),
        summary: format!("summary-{patch_id}"),
        patch_decision: "pending".to_string(),
        patch_apply_status: "pending".to_string(),
        patches: vec![OptimizationPatchCreateInput {
            patch_id: format!("patch-{patch_id}"),
            issue_id: None,
            operation: "replace".to_string(),
            old_text: "old".to_string(),
            new_text: "new".to_string(),
            reason: "test".to_string(),
            risk: None,
        }],
    };
    let first_patch_set = create_optimization_patch_set(&db, patch_set_input(first.id, "first"))
        .await
        .expect("first patch set must be created")
        .expect("non-empty patch set must be persisted");
    let second_patch_set = create_optimization_patch_set(&db, patch_set_input(second.id, "second"))
        .await
        .expect("second patch set must be created")
        .expect("non-empty patch set must be persisted");
    let patch_sets = optimization_patch_sets(&db, session.id)
        .await
        .expect("patch sets must be listed");
    let patch_set_ids = patch_sets
        .iter()
        .map(|patch_set| patch_set.id)
        .collect::<Vec<_>>();
    let patches = optimization_patch_set_patches_for_sets(&db, &patch_set_ids)
        .await
        .expect("batch patch query must succeed");

    assert_eq!(patch_sets.len(), 2);
    assert!(
        patch_sets
            .iter()
            .any(|patch_set| patch_set.id == first_patch_set.id)
    );
    assert!(
        patch_sets
            .iter()
            .any(|patch_set| patch_set.id == second_patch_set.id)
    );
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].patch_set_id, first_patch_set.id);
    assert_eq!(patches[1].patch_set_id, second_patch_set.id);
}
