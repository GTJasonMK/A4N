use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    character::protagonist_profile,
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project,
    },
};
use a4n_db::{
    blueprint::{
        SaveManualBlueprintInput, UpdateBlueprintWorldSettingInput,
        save_manual_blueprint_records_transaction,
        update_blueprint_world_setting_records_transaction,
    },
    character::sync_character_profiles_from_blueprint,
    import_reverse::{
        ReverseBlueprintApplyDraft, ReversePartOutlineApplyRow,
        apply_reverse_blueprint_records_transaction,
        apply_reverse_part_outline_records_transaction, reverse_context_rows,
    },
    project::{
        BlueprintCharacterDraft, BlueprintRelationshipDraft, CreateProjectRecordInput,
        PROJECT_TYPE_NOVEL_LONG, blueprint_sources_need_review, create_project_record,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

async fn create_project(db: &sea_orm::DatabaseConnection, description: &str) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "钟楼旧稿".to_string(),
            description: description.to_string(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn blueprint_context_allows_formal_materials_without_chapter_body() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "从废弃钟楼和旧钥匙继续创作").await;

    let rows = reverse_context_rows(&db, project_id, true).await.unwrap();

    assert!(rows.chapters.is_empty());
    assert_eq!(rows.project.description, "从废弃钟楼和旧钥匙继续创作");
    assert!(rows.blueprint_context.is_some());
    let shared_rows = reverse_context_rows(&db, project_id, false).await.unwrap();
    assert!(shared_rows.chapters.is_empty());
    assert!(shared_rows.blueprint_context.is_none());
}

#[tokio::test]
async fn reverse_blueprint_preserves_omitted_materials_and_records_version() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有设定").await;
    let now = Utc::now().naive_utc();
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧蓝图".to_string()),
        genre: Set("悬疑".to_string()),
        style: Set(Some("冷峻".to_string())),
        tone: Set(Some("克制".to_string())),
        target_audience: Set(Some("成人".to_string())),
        one_sentence_summary: Set(Some("旧概要".to_string())),
        full_synopsis: Set(Some("旧梗概".to_string())),
        world_setting: Set(Some(json!({"premise": "旧城钟声倒流"}))),
        current_version: Set("1.0".to_string()),
        needs_part_outlines: Set(false),
        total_chapters: Set(Some(30)),
        chapters_per_part: Set(None),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
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
    .insert(&db)
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
    .insert(&db)
    .await
    .unwrap();

    let result = apply_reverse_blueprint_records_transaction(
        &db,
        project_id,
        ReverseBlueprintApplyDraft {
            title: "钟楼回声".to_string(),
            genre: "悬疑".to_string(),
            style: "冷峻".to_string(),
            tone: "克制".to_string(),
            target_audience: "成人".to_string(),
            one_sentence_summary: "追查倒走钟声背后的失踪案。".to_string(),
            full_synopsis: "林岚依据旧钥匙和钟楼记录追查母亲失踪真相。".to_string(),
            world_setting: json!({"premise": "旧城钟声会倒走"}),
            incoming_total_chapters: Some(30),
            incoming_needs_part_outlines: Some(false),
            characters: vec![BlueprintCharacterDraft {
                name: "林岚".to_string(),
                character_type: "protagonist".to_string(),
                gender: Some("女".to_string()),
                age: Some("25".to_string()),
                appearance: None,
                personality: Some("AI 试图覆盖".to_string()),
                background: Some("AI 试图覆盖".to_string()),
                motivation: Some("查明钟声真相".to_string()),
                arc_summary: None,
                relationship_to_protagonist: None,
                arc_stages: None,
            }],
            relationships: vec![BlueprintRelationshipDraft {
                relationship_type: "friend".to_string(),
                character_a: "林岚".to_string(),
                character_b: "陈恪".to_string(),
                description: Some("AI 试图覆盖关系".to_string()),
                strength: Some(9),
                evolution: Some(json!(["共同调查"])),
            }],
        },
    )
    .await
    .unwrap();

    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    let version = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq("1.1"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let project = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq("林岚"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(result.character_count, 1);
    assert_eq!(result.relationship_count, 1);
    assert_eq!(result.version, "1.1");
    assert_eq!(characters[0].background.as_deref(), Some("用户手工录入"));
    assert_eq!(characters[0].personality.as_deref(), Some("谨慎"));
    assert_eq!(characters[0].age.as_deref(), Some("25"));
    assert_eq!(characters[0].motivation.as_deref(), Some("查明钟声真相"));
    assert_eq!(
        relationships[0].description.as_deref(),
        Some("用户确认的旧友关系")
    );
    assert_eq!(relationships[0].strength, Some(7));
    assert_eq!(relationships[0].evolution, Some(json!(["共同调查"])));
    assert_eq!(version.blueprint_data["characters"][0]["name"], "林岚");
    assert_eq!(
        version.blueprint_data["relationships"][0]["character_b"],
        "陈恪"
    );
    assert_eq!(project.title, "钟楼回声");
    assert_eq!(project.target_chapters, 30);
    assert_eq!(project.total_chapters, Some(30));
    assert_eq!(profile.background.as_deref(), Some("用户手工录入"));
    let raw_blueprint: serde_json::Value =
        serde_json::from_str(project.raw_blueprint.as_deref().unwrap()).unwrap();
    assert_eq!(
        raw_blueprint["full_synopsis"],
        "林岚依据旧钥匙和钟楼记录追查母亲失踪真相。"
    );
}

#[tokio::test]
async fn reverse_blueprint_rejects_total_before_existing_materials_without_partial_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有正文，稍后归纳蓝图").await;
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(20),
        title: Set(Some("第二十章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set("imported".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let error = apply_reverse_blueprint_records_transaction(
        &db,
        project_id,
        ReverseBlueprintApplyDraft {
            title: "钟楼回声".to_string(),
            genre: "悬疑".to_string(),
            style: "冷峻".to_string(),
            tone: "克制".to_string(),
            target_audience: "成人".to_string(),
            one_sentence_summary: "追查钟楼。".to_string(),
            full_synopsis: "林岚追查钟楼失踪案。".to_string(),
            world_setting: json!({"premise": "钟声倒流"}),
            incoming_total_chapters: Some(19),
            incoming_needs_part_outlines: Some(false),
            characters: Vec::new(),
            relationships: Vec::new(),
        },
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("不能小于现有正式资料覆盖的第 20 章")
    );
    assert!(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn part_outline_aggregation_uses_existing_outlines_and_rebinds_them() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有章纲，尚未写正文").await;
    let now = Utc::now().naive_utc();
    chapter_outline::ActiveModel {
        project_id: Set(project_id),
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
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let rows = reverse_context_rows(&db, project_id, false).await.unwrap();
    assert!(rows.chapters.is_empty());
    assert_eq!(rows.chapter_outlines.len(), 1);

    apply_reverse_part_outline_records_transaction(
        &db,
        project_id,
        &[ReversePartOutlineApplyRow {
            part_number: 1,
            title: "旧钟初鸣".to_string(),
            start_chapter: 1,
            end_chapter: 12,
            summary: "主角开始调查钟楼。".to_string(),
            theme: "启程".to_string(),
            key_events: json!(["收到来信", "进入钟楼", "发现倒流钟声"]),
            character_focus: json!({"林岚": "从逃避转为主动调查"}),
        }],
    )
    .await
    .unwrap();

    let part = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(12))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outline.part_id, Some(part.id));
}

#[tokio::test]
async fn part_outline_aggregation_rejects_overlap_without_partial_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有后续分部").await;
    let now = Utc::now().naive_utc();
    part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(2),
        title: Set("旧钟回响".to_string()),
        start_chapter: Set(13),
        end_chapter: Set(24),
        summary: Set(Some("调查继续深入。".to_string())),
        theme: Set(Some("追查".to_string())),
        key_events: Set(Some(json!(["进入地宫"]))),
        character_focus: Set(Some(json!({"林岚": "承担代价"}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let error = apply_reverse_part_outline_records_transaction(
        &db,
        project_id,
        &[ReversePartOutlineApplyRow {
            part_number: 1,
            title: "旧钟初鸣".to_string(),
            start_chapter: 1,
            end_chapter: 15,
            summary: "主角开始调查钟楼。".to_string(),
            theme: "启程".to_string(),
            key_events: json!(["收到来信", "进入钟楼", "发现倒流钟声"]),
            character_focus: json!({"林岚": "从逃避转为主动调查"}),
        }],
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("章节范围重叠"));
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].part_number, 2);
}

#[tokio::test]
async fn manual_blueprint_save_preserves_owned_materials_and_creates_restorable_version() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有手工世界观和角色资料").await;
    let now = Utc::now().naive_utc();
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("钟楼旧稿".to_string()),
        genre: Set("悬疑".to_string()),
        style: Set(Some("旧风格".to_string())),
        tone: Set(Some("旧基调".to_string())),
        target_audience: Set(None),
        one_sentence_summary: Set(None),
        full_synopsis: Set(None),
        world_setting: Set(Some(json!({"premise": "钟声会倒流"}))),
        current_version: Set("1.0".to_string()),
        needs_part_outlines: Set(false),
        total_chapters: Set(Some(30)),
        chapters_per_part: Set(None),
        avatar_svg: Set(Some("<svg/>".to_string())),
        avatar_animal: Set(Some("owl".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
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
    .insert(&db)
    .await
    .unwrap();

    let result = save_manual_blueprint_records_transaction(
        &db,
        SaveManualBlueprintInput {
            project_id,
            genre: "  都市悬疑  ".to_string(),
            style: Some("  冷峻  ".to_string()),
            tone: None,
            target_audience: None,
            one_sentence_summary: Some("追查倒走钟声。".to_string()),
            full_synopsis: None,
            total_chapters: 1,
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap();

    let project = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();
    let version = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq("1.1"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(result.blueprint.genre, "都市悬疑");
    assert_eq!(result.blueprint.total_chapters, Some(1));
    assert_eq!(
        result.blueprint.world_setting,
        Some(json!({"premise": "钟声会倒流"}))
    );
    assert_eq!(result.blueprint.avatar_animal.as_deref(), Some("owl"));
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].background.as_deref(), Some("用户手工录入"));
    assert_eq!(version.blueprint_data["characters"][0]["name"], "林岚");
    assert_eq!(
        version.blueprint_data["world_setting"]["premise"],
        "钟声会倒流"
    );
    assert_eq!(project.status, "blueprint_ready");
    assert_eq!(project.target_chapters, 1);
    assert_eq!(project.total_chapters, Some(1));
    assert!(
        !blueprint_sources_need_review(&db, project_id, result.blueprint.updated_at)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn blueprint_source_review_tracks_changes_after_last_blueprint_save() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "检查正式资料变化").await;
    let saved = save_manual_blueprint_records_transaction(
        &db,
        SaveManualBlueprintInput {
            project_id,
            genre: "悬疑".to_string(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: None,
            total_chapters: 1,
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap();
    let chapter_changed_at = saved.blueprint.updated_at + chrono::Duration::seconds(1);
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("钟楼来信".to_string())),
        content: Set("新增正文".to_string()),
        word_count: Set(4),
        status: Set("imported".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(chapter_changed_at),
        updated_at: Set(chapter_changed_at),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    assert!(
        blueprint_sources_need_review(&db, project_id, saved.blueprint.updated_at)
            .await
            .unwrap()
    );

    let reviewed_at = chapter_changed_at + chrono::Duration::seconds(1);
    let mut blueprint: novel_blueprint::ActiveModel = saved.blueprint.into();
    blueprint.updated_at = Set(reviewed_at);
    blueprint.update(&db).await.unwrap();
    assert!(
        !blueprint_sources_need_review(&db, project_id, reviewed_at)
            .await
            .unwrap()
    );

    let project_changed_at = reviewed_at + chrono::Duration::seconds(1);
    let project = novel_project::Entity::find_by_id(project_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let mut project: novel_project::ActiveModel = project.into();
    project.description = Set("用户补充了项目核心设定".to_string());
    project.updated_at = Set(project_changed_at);
    project.update(&db).await.unwrap();
    assert!(
        blueprint_sources_need_review(&db, project_id, reviewed_at)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn manual_blueprint_save_rejects_total_before_existing_materials_without_partial_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有正文、章纲和分部").await;
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(12),
        title: Set(Some("钟楼来信".to_string())),
        content: Set("已有正文".to_string()),
        word_count: Set(4),
        status: Set("imported".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(14),
        title: Set(Some("地宫回声".to_string())),
        summary: Set(None),
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
    part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("旧钟初鸣".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(20),
        summary: Set(None),
        theme: Set(None),
        key_events: Set(None),
        character_focus: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let error = save_manual_blueprint_records_transaction(
        &db,
        SaveManualBlueprintInput {
            project_id,
            genre: "悬疑".to_string(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: None,
            total_chapters: 19,
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("不能小于现有正式资料覆盖的第 20 章")
    );
    assert!(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .all(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn world_setting_update_creates_version_without_replacing_other_blueprint_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "更新世界观").await;

    save_manual_blueprint_records_transaction(
        &db,
        SaveManualBlueprintInput {
            project_id,
            genre: "悬疑".to_string(),
            style: Some("冷峻".to_string()),
            tone: None,
            target_audience: None,
            one_sentence_summary: Some("追查钟楼。".to_string()),
            full_synopsis: Some("林岚追查钟楼失踪案。".to_string()),
            total_chapters: 12,
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap();

    let result = update_blueprint_world_setting_records_transaction(
        &db,
        UpdateBlueprintWorldSettingInput {
            project_id,
            world_setting: json!({"premise": "午夜钟声会让时间倒退"}),
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap();
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(result.blueprint.current_version, "1.1");
    assert_eq!(result.blueprint.style.as_deref(), Some("冷峻"));
    assert_eq!(
        result.blueprint.full_synopsis.as_deref(),
        Some("林岚追查钟楼失踪案。")
    );
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().any(|version| {
        version.version == "1.1"
            && version.blueprint_data["world_setting"]["premise"] == "午夜钟声会让时间倒退"
    }));
}

#[tokio::test]
async fn blueprint_profile_seed_only_fills_missing_manual_profile_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "已有用户角色档案").await;
    save_manual_blueprint_records_transaction(
        &db,
        SaveManualBlueprintInput {
            project_id,
            genre: "悬疑".to_string(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: None,
            total_chapters: 1,
            created_by: "user:1".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林岚".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(Some("男".to_string())),
        age: Set(Some("20".to_string())),
        appearance: Set(None),
        personality: Set(Some("冷静".to_string())),
        background: Set(Some("AI 蓝图背景".to_string())),
        motivation: Set(None),
        arc_summary: Set(Some("AI 角色弧光".to_string())),
        relationship_to_protagonist: Set(None),
        arc_stages: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let manual_attributes = json!({
        "explicit_attributes": {
            "gender": "女",
            "background": "用户手工背景"
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    protagonist_profile::ActiveModel {
        project_id: Set(project_id),
        name: Set("林岚".to_string()),
        gender: Set(Some("女".to_string())),
        age: Set(None),
        base_attributes: Set(manual_attributes.clone()),
        current_state: Set(manual_attributes),
        character_arc: Set(Some(json!({"summary": "用户手工弧光"}))),
        voice_fingerprint: Set(None),
        appearance: Set(None),
        background: Set(Some("用户手工背景".to_string())),
        personality: Set(None),
        last_synced_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    sync_character_profiles_from_blueprint(&db, project_id, now)
        .await
        .unwrap();

    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq("林岚"))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(profile.gender.as_deref(), Some("女"));
    assert_eq!(profile.age.as_deref(), Some("20"));
    assert_eq!(profile.background.as_deref(), Some("用户手工背景"));
    assert_eq!(profile.personality.as_deref(), Some("冷静"));
    assert_eq!(
        profile.character_arc,
        Some(json!({"summary": "用户手工弧光"}))
    );
}
