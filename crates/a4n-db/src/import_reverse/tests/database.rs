use a4n_core::models::{
    chapter::{chapter_outline, chapter_outline_provenance, part_outline},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project,
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::json;

use crate::import_reverse::{
    ReverseBlueprintApplyDraft, ReverseOutlineApplyRow, ReversePartOutlineApplyRow,
    apply_reverse_blueprint_records_transaction, apply_reverse_outline_records_transaction,
    apply_reverse_part_outline_records_transaction,
};
use crate::project::{CHAPTERS_PER_PART, PROJECT_KIND_TEXT, SOURCE_MODE_SELF, SOURCE_SYNC_CURRENT};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

async fn insert_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    let now = fixed_time();
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set(title.to_string()),
        description: Set("反推 apply DB 测试".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(PROJECT_KIND_TEXT.to_string()),
        source_mode: Set(SOURCE_MODE_SELF.to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        derived_project_status: Set(None),
        status: Set("draft".to_string()),
        target_chapters: Set(100),
        current_chapter: Set(0),
        total_chapters: Set(Some(100)),
        chapters_per_part: Set(Some(CHAPTERS_PER_PART)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_existing_blueprint(
    db: &DatabaseConnection,
    project_id: i64,
    total_chapters: i32,
    needs_part_outlines: bool,
) {
    let now = fixed_time();
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧蓝图".to_string()),
        genre: Set("科幻".to_string()),
        style: Set(Some("冷峻".to_string())),
        tone: Set(Some("悬疑".to_string())),
        target_audience: Set(Some("成人".to_string())),
        one_sentence_summary: Set(Some("旧概要".to_string())),
        full_synopsis: Set(Some("旧大纲".to_string())),
        world_setting: Set(Some(json!({"core_rules": ["旧规则"]}))),
        current_version: Set("1.0".to_string()),
        needs_part_outlines: Set(needs_part_outlines),
        total_chapters: Set(Some(total_chapters)),
        chapters_per_part: Set(needs_part_outlines.then_some(CHAPTERS_PER_PART)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn insert_existing_blueprint_materials(db: &DatabaseConnection, project_id: i64) {
    let now = fixed_time();
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林岚".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(None),
        age: Set(None),
        appearance: Set(None),
        personality: Set(Some("谨慎".to_string())),
        background: Set(Some("用户手工录入".to_string())),
        motivation: Set(None),
        arc_summary: Set(None),
        relationship_to_protagonist: Set(None),
        arc_stages: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("friend".to_string()),
        character_a: Set("林岚".to_string()),
        character_b: Set("陈恪".to_string()),
        description: Set(Some("用户确认的旧友关系".to_string())),
        strength: Set(Some(7)),
        evolution: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn blueprint_draft(
    total_chapters: Option<i32>,
    needs_part_outlines: Option<bool>,
) -> ReverseBlueprintApplyDraft {
    ReverseBlueprintApplyDraft {
        title: "星潮档案".to_string(),
        genre: "科幻".to_string(),
        style: "冷峻".to_string(),
        tone: "悬疑".to_string(),
        target_audience: "成人".to_string(),
        one_sentence_summary: "调查星潮真相。".to_string(),
        full_synopsis: "主角追查星潮背后的阴谋。".to_string(),
        world_setting: json!({"premise": "星潮改写记忆"}),
        incoming_total_chapters: total_chapters,
        incoming_needs_part_outlines: needs_part_outlines,
        characters: Vec::new(),
        relationships: Vec::new(),
    }
}

async fn blueprint_for_project(db: &DatabaseConnection, project_id: i64) -> novel_blueprint::Model {
    novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn reverse_blueprint_transaction_persists_derived_part_outline_threshold() {
    let db = setup_db().await;
    let short_project = insert_project(&db, "五十章项目").await;
    let long_project = insert_project(&db, "五十一章项目").await;

    apply_reverse_blueprint_records_transaction(
        &db,
        short_project.id,
        blueprint_draft(Some(50), Some(false)),
    )
    .await
    .unwrap();
    apply_reverse_blueprint_records_transaction(
        &db,
        long_project.id,
        blueprint_draft(Some(51), Some(true)),
    )
    .await
    .unwrap();

    let short = blueprint_for_project(&db, short_project.id).await;
    let long = blueprint_for_project(&db, long_project.id).await;
    let short_versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(short_project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(short.total_chapters, Some(50));
    assert!(!short.needs_part_outlines);
    assert_eq!(short.chapters_per_part, None);
    assert_eq!(long.total_chapters, Some(51));
    assert!(long.needs_part_outlines);
    assert_eq!(long.chapters_per_part, Some(CHAPTERS_PER_PART));
    assert_eq!(short.current_version, "1.0");
    assert_eq!(short_versions.len(), 1);
    assert_eq!(short_versions[0].version, "1.0");
}

#[tokio::test]
async fn reverse_blueprint_transaction_preserves_existing_chapter_plan_when_omitted() {
    let db = setup_db().await;
    let project = insert_project(&db, "既有五十一章项目").await;
    insert_existing_blueprint(&db, project.id, 51, true).await;
    insert_existing_blueprint_materials(&db, project.id).await;

    apply_reverse_blueprint_records_transaction(&db, project.id, blueprint_draft(None, None))
        .await
        .unwrap();

    let blueprint = blueprint_for_project(&db, project.id).await;
    let project = novel_project::Entity::find_by_id(project.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(blueprint.total_chapters, Some(51));
    assert!(blueprint.needs_part_outlines);
    assert_eq!(blueprint.chapters_per_part, Some(CHAPTERS_PER_PART));
    assert_eq!(project.total_chapters, Some(51));
    assert_eq!(project.target_chapters, 51);
    assert_eq!(project.chapters_per_part, Some(CHAPTERS_PER_PART));
    assert_eq!(blueprint.current_version, "1.1");
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    let version = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project.id))
        .filter(blueprint_version::Column::Version.eq("1.1"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].background.as_deref(), Some("用户手工录入"));
    assert_eq!(relationships.len(), 1);
    assert_eq!(
        relationships[0].description.as_deref(),
        Some("用户确认的旧友关系")
    );
    assert_eq!(version.blueprint_data["characters"][0]["name"], "林岚");
    assert_eq!(
        version.blueprint_data["relationships"][0]["character_b"],
        "陈恪"
    );
}

#[tokio::test]
async fn reverse_blueprint_transaction_rejects_conflict_without_mutation() {
    let db = setup_db().await;
    let project = insert_project(&db, "章节数冲突项目").await;
    insert_existing_blueprint(&db, project.id, 50, false).await;

    let error = apply_reverse_blueprint_records_transaction(
        &db,
        project.id,
        blueprint_draft(Some(51), None),
    )
    .await
    .expect_err("conflicting chapter count must fail");

    assert!(error.to_string().contains("total_chapters"));
    let blueprint = blueprint_for_project(&db, project.id).await;
    assert_eq!(blueprint.title, "旧蓝图");
    assert_eq!(blueprint.total_chapters, Some(50));
    assert!(!blueprint.needs_part_outlines);
}

#[tokio::test]
async fn reverse_blueprint_transaction_rejects_mismatched_flag_without_insert() {
    let db = setup_db().await;
    let project = insert_project(&db, "分部标记冲突项目").await;

    let error = apply_reverse_blueprint_records_transaction(
        &db,
        project.id,
        blueprint_draft(Some(100), Some(false)),
    )
    .await
    .expect_err("mismatched part outline flag must fail");

    assert!(error.to_string().contains("needs_part_outlines"));
    let count = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn reverse_outline_transaction_updates_existing_word_count_target() {
    let db = setup_db().await;
    let project = insert_project(&db, "章纲更新项目").await;
    let now = fixed_time();
    let existing = chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("旧标题".to_string())),
        summary: Set(Some("旧摘要".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(None),
        status: Set("draft".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    chapter_outline_provenance::ActiveModel {
        outline_id: Set(existing.id),
        source_kind: Set(
            chapter_outline_provenance::SOURCE_KIND_RETROSPECTIVE_ANALYSIS.to_string(),
        ),
        source_fingerprint_json: Set(json!({"source": "old chapter analysis"})),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .unwrap();

    let count = apply_reverse_outline_records_transaction(
        &db,
        project.id,
        &[ReverseOutlineApplyRow {
            chapter_number: 1,
            title: "新标题".to_string(),
            summary: "新摘要".to_string(),
            word_count_target: Some(3200),
        }],
    )
    .await
    .unwrap();

    let outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project.id))
        .filter(chapter_outline::Column::ChapterNumber.eq(1))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(outline.title.as_deref(), Some("新标题"));
    assert_eq!(outline.summary.as_deref(), Some("新摘要"));
    assert_eq!(outline.word_count_target, Some(3200));
    assert_eq!(outline.foreshadowing_setup, None);
    assert_eq!(outline.foreshadowing_payoff, None);
    assert!(outline.updated_at > existing.updated_at);
    assert!(
        chapter_outline_provenance::Entity::find_by_id(existing.id)
            .one(&db)
            .await
            .unwrap()
            .is_none(),
        "用户确认的反推章纲必须成为正式写作计划"
    );
}

#[tokio::test]
async fn reverse_part_outline_transaction_inserts_and_updates_canonical_row() {
    let db = setup_db().await;
    let project = insert_project(&db, "分部更新项目").await;
    chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(12),
        title: Set(Some("钟楼来信".to_string())),
        summary: Set(Some("林岚收到来自未来的信。".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(None),
        status: Set("draft".to_string()),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let first = ReversePartOutlineApplyRow {
        part_number: 1,
        title: "第一部".to_string(),
        start_chapter: 1,
        end_chapter: 25,
        summary: "初始摘要".to_string(),
        theme: "启程".to_string(),
        key_events: json!(["获得线索"]),
        character_focus: json!({"主角": "接受任务"}),
    };
    let updated = ReversePartOutlineApplyRow {
        title: "第一部·星潮".to_string(),
        end_chapter: 24,
        summary: "更新摘要".to_string(),
        theme: "代价".to_string(),
        key_events: json!(["旧敌回归", "盟友背叛"]),
        character_focus: json!({"主角": "主动追查"}),
        ..first.clone()
    };

    apply_reverse_part_outline_records_transaction(&db, project.id, &[first])
        .await
        .unwrap();
    apply_reverse_part_outline_records_transaction(&db, project.id, &[updated])
        .await
        .unwrap();

    let rows = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    let outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project.id))
        .filter(chapter_outline::Column::ChapterNumber.eq(12))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "第一部·星潮");
    assert_eq!(rows[0].end_chapter, 24);
    assert_eq!(rows[0].summary.as_deref(), Some("更新摘要"));
    assert_eq!(rows[0].theme.as_deref(), Some("代价"));
    assert_eq!(rows[0].key_events, Some(json!(["旧敌回归", "盟友背叛"])));
    assert_eq!(rows[0].character_focus, Some(json!({"主角": "主动追查"})));
    assert_eq!(outline.part_id, Some(rows[0].id));
}
