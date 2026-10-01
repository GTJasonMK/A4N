use a4n_db::{
    asset::{
        AssetDbError, create_object_profile_record,
        upsert_object_profiles_with_generation_run_record,
    },
    chapter_content::{
        GeneratedChapterContentRecordInput, finish_generated_chapter_content_record_in_transaction,
        import_chapter_content_record,
    },
    character::{create_profile, create_profile_with_generation_run_in},
    outline::{
        OUTLINE_GENERATION_SOURCE_CHANGED, OutlineDbError, load_outline_generation_source_rows,
        lock_outline_generation_sources, outline_generation_source_snapshot,
        sync_chapter_outline_records_by_number,
    },
    project::{
        CreateProjectRecordInput, ProjectChapterOutlineUpdateInput,
        RetrospectiveChapterOutlineAction, RetrospectiveChapterOutlineUpsertInput,
        chapter_outline_by_id, character_sync_chapters, create_project_record,
        delete_automatic_chapter_outlines_for_source_change,
        formal_chapter_outline_by_project_number, update_chapter_outline_record,
        upsert_retrospective_chapter_outline_checked,
    },
    scene_profile::{SceneProfileDbError, replace_chapter_scene_sync_with_generation_run_records},
    text_anchor::{
        EVIDENCE_ROLE_FIRST_SEEN, delete_chapter_content_profile_evidence_records,
        merge_profile_evidence_records_batch, rebuild_chapter_content_anchor_records,
    },
};
use a4n_db_models::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, chapter_outline_provenance},
    character::protagonist_profile,
    generation::generation_run,
    project::{novel_project, profile_evidence, text_anchor},
    scene::scene_profile,
};
use a4n_domain::chapter::ChapterOutlineSyncDraft;
use a4n_domain::chapter::ImportChapterContentInput;
use a4n_domain::generation::ProjectLifecycleOperation;
use a4n_domain::generation::RecordCompletedGenerationRunInput;
use a4n_domain::object_profile::{ObjectProfileEvidenceRecordInput, ObjectProfileRecordInput};
use a4n_domain::scene_profile::{
    SceneProfileEvidenceRecordInput, SceneProfileRecordDraft, SceneProfileRecordInput,
    SceneSyncRecordInput,
};
use a4n_domain::text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set, TransactionTrait,
};
use serde_json::json;
use tokio::{
    sync::oneshot,
    time::{Duration as TokioDuration, timeout},
};

async fn create_project(db: &sea_orm::DatabaseConnection, title: &str) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

async fn insert_source_chapter(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    content: &str,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
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
    .expect("source chapter must be inserted")
}

fn object_generation_run(project_id: i64) -> RecordCompletedGenerationRunInput {
    RecordCompletedGenerationRunInput {
        project_id,
        workflow: "object_discovery".to_string(),
        step_key: "object_discovery.llm".to_string(),
        title: "物体档案发现".to_string(),
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

fn scene_generation_run(project_id: i64) -> RecordCompletedGenerationRunInput {
    let mut run = object_generation_run(project_id);
    run.workflow = "scene_profile_sync".to_string();
    run.step_key = "sync_scene_profiles_for_chapter".to_string();
    run.title = "场景档案同步".to_string();
    run
}

fn character_discovery_generation_run(project_id: i64) -> RecordCompletedGenerationRunInput {
    let mut run = object_generation_run(project_id);
    run.workflow = "character_discovery".to_string();
    run.step_key = "character_discovery.llm".to_string();
    run.title = "角色发现".to_string();
    run
}

fn discovered_object() -> ObjectProfileRecordInput {
    ObjectProfileRecordInput {
        name: "青铜钥匙".to_string(),
        normalized_name: "青铜钥匙".to_string(),
        aliases_json: json!([]),
        object_type: "token".to_string(),
        importance: "primary".to_string(),
        visual_description: "带有锈迹的青铜钥匙".to_string(),
        material_and_state: "青铜，轻微锈蚀".to_string(),
        story_usage: "开启石门".to_string(),
        visual_usage: "关键特写".to_string(),
        generation_usage: "保持锈迹和齿纹一致".to_string(),
        location_context: Some("主角口袋".to_string()),
        first_seen_chapter: 1,
        last_seen_chapter: 1,
        evidence_json: json!(["主角捡起青铜钥匙"]),
        image_prompt_seed: "锈蚀青铜钥匙".to_string(),
    }
}

fn discovered_scene() -> SceneSyncRecordInput {
    SceneSyncRecordInput {
        profiles: vec![SceneProfileRecordDraft {
            scene_ref: "old-study".to_string(),
            scene_profile_id: None,
            input: SceneProfileRecordInput {
                name: "旧书房".to_string(),
                normalized_name: "旧书房".to_string(),
                aliases_json: json!([]),
                scene_type: "room".to_string(),
                importance: "primary".to_string(),
                parent_scene_id: None,
                base_description: "一间封闭的旧书房".to_string(),
                visual_description: "木书桌靠窗".to_string(),
                spatial_layout: "门、书桌、窗户".to_string(),
                atmosphere: "昏暗安静".to_string(),
                entry_points_json: json!(["木门"]),
                rules_json: json!([]),
                first_seen_chapter: 1,
                last_seen_chapter: 1,
                evidence_json: json!(["主角进入旧书房"]),
                image_prompt_seed: "昏暗旧书房".to_string(),
            },
        }],
        occurrences: vec![],
        transitions: vec![],
        entity_states: vec![],
    }
}

async fn insert_user_outline(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> a4n_domain::chapter::ChapterOutlineRecord {
    let now = Utc::now().naive_utc();
    let outline = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        chapter_number: Set(1),
        title: Set(Some("用户章纲".to_string())),
        summary: Set(Some("用户计划".to_string())),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("user outline must be inserted");
    chapter_outline_by_id(db, outline.id)
        .await
        .expect("user outline lookup must succeed")
        .expect("user outline must exist")
}

fn generated_outline_draft(title: &str) -> ChapterOutlineSyncDraft {
    ChapterOutlineSyncDraft {
        chapter_number: 1,
        title: title.to_string(),
        summary: "模型生成摘要".to_string(),
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: Some(2_000),
    }
}

async fn outline_source_snapshot(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
) -> serde_json::Value {
    let rows = load_outline_generation_source_rows(db, project_id)
        .await
        .expect("outline generation sources must load")
        .expect("project must exist");
    outline_generation_source_snapshot(&rows)
}

fn assert_outline_source_changed(error: OutlineDbError) {
    assert!(matches!(
        error,
        OutlineDbError::InvalidState { current, .. }
            if current == OUTLINE_GENERATION_SOURCE_CHANGED
    ));
}

#[tokio::test]
async fn stale_outline_generation_cannot_overwrite_new_formal_outline() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "旧生成结果不得覆盖新章纲").await;
    let snapshot = outline_source_snapshot(&db, project_id).await;
    let user_outline = insert_user_outline(&db, project_id).await;

    let result: Result<(), OutlineDbError> = async {
        let tx = lock_outline_generation_sources(&db, project_id, &snapshot).await?;
        sync_chapter_outline_records_by_number(
            &tx,
            project_id,
            None,
            vec![generated_outline_draft("过期模型章纲")],
            Utc::now().naive_utc(),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    .await;

    assert_outline_source_changed(result.expect_err("stale generation must be rejected"));
    let stored = chapter_outline::Entity::find_by_id(user_outline.id)
        .one(&db)
        .await
        .expect("outline lookup must succeed")
        .expect("user outline must remain");
    assert_eq!(stored.title.as_deref(), Some("用户章纲"));
    assert_eq!(stored.summary.as_deref(), Some("用户计划"));
}

#[tokio::test]
async fn outline_generation_lock_orders_new_material_before_generated_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "新增资料与大纲写入有明确顺序").await;
    let snapshot = outline_source_snapshot(&db, project_id).await;
    let tx = lock_outline_generation_sources(&db, project_id, &snapshot)
        .await
        .expect("matching sources must lock");

    let insert_db = db.clone();
    let (attempting, attempted) = oneshot::channel();
    let mut insert = tokio::spawn(async move {
        let _ = attempting.send(());
        create_profile(
            &insert_db,
            project_id,
            "林岚",
            json!({
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        )
        .await
    });
    attempted.await.expect("insert task must start");
    assert!(
        timeout(TokioDuration::from_millis(200), &mut insert)
            .await
            .is_err(),
        "new formal material must wait for the outline source lock"
    );

    tx.commit().await.expect("source lock must commit");
    timeout(TokioDuration::from_secs(5), insert)
        .await
        .expect("profile insert must unblock")
        .expect("profile task must complete")
        .expect("profile insert must succeed");

    let error = lock_outline_generation_sources(&db, project_id, &snapshot)
        .await
        .expect_err("new character must invalidate the old source snapshot");
    assert_outline_source_changed(error);
}

#[tokio::test]
async fn later_outline_task_with_the_same_snapshot_cannot_overwrite_first_commit() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "并发大纲任务按正式资料顺序提交").await;
    let snapshot = outline_source_snapshot(&db, project_id).await;
    let first_tx = lock_outline_generation_sources(&db, project_id, &snapshot)
        .await
        .expect("first task must lock matching sources");

    let second_db = db.clone();
    let second_snapshot = snapshot.clone();
    let (attempting, attempted) = oneshot::channel();
    let mut second = tokio::spawn(async move {
        let _ = attempting.send(());
        lock_outline_generation_sources(&second_db, project_id, &second_snapshot)
            .await
            .map(|_| ())
    });
    attempted.await.expect("second task must start");
    assert!(
        timeout(TokioDuration::from_millis(200), &mut second)
            .await
            .is_err(),
        "second task must wait for the first formal write"
    );

    sync_chapter_outline_records_by_number(
        &first_tx,
        project_id,
        None,
        vec![generated_outline_draft("先提交的章纲")],
        Utc::now().naive_utc(),
    )
    .await
    .expect("first generated outline must persist");
    first_tx.commit().await.expect("first task must commit");

    let error = timeout(TokioDuration::from_secs(5), second)
        .await
        .expect("second task must unblock")
        .expect("second task must complete")
        .expect_err("second task must reject its stale snapshot");
    assert_outline_source_changed(error);
    let stored = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .one(&db)
        .await
        .expect("outline lookup must succeed")
        .expect("first outline must remain");
    assert_eq!(stored.title.as_deref(), Some("先提交的章纲"));
}

#[tokio::test]
async fn outline_generation_does_not_write_while_project_is_resetting() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "重置期间禁止大纲写入").await;
    let resetting_gate = ProjectLifecycleOperation::Reset.gate();
    let rows = load_outline_generation_source_rows(&db, project_id)
        .await
        .expect("outline generation sources must load")
        .expect("project must exist");
    let snapshot = outline_generation_source_snapshot(&rows);
    let mut project: novel_project::ActiveModel =
        novel_project::Entity::find_by_id(rows.project.id)
            .one(&db)
            .await
            .expect("project lookup must succeed")
            .expect("project must exist")
            .into();
    project.lifecycle_gate = Set(resetting_gate.to_string());
    project
        .update(&db)
        .await
        .expect("project lifecycle gate must update");

    let error = lock_outline_generation_sources(&db, project_id, &snapshot)
        .await
        .expect_err("resetting project must reject outline writes");
    assert!(matches!(
        error,
        OutlineDbError::InvalidState { current, .. }
            if current == resetting_gate
    ));
    assert!(
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn generated_chapter_does_not_require_or_create_an_outline() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "无章纲正文生成").await;
    let now = Utc::now().naive_utc();
    let content = "林岚推开旧仓库的门。";

    let chapter = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id,
            chapter_number: 1,
            existing_chapter_id: None,
            source_outline: None,
            title: "第一章".to_string(),
            summary: "林岚进入旧仓库".to_string(),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            now,
        },
    )
    .await
    .expect("chapter generation must persist without a formal outline");

    assert_eq!(chapter.outline_id, None);
    assert!(
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn generated_chapter_links_only_to_the_unchanged_outline_it_used() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let now = Utc::now().naive_utc();

    let unchanged_project = create_project(&db, "关联实际使用章纲").await;
    let unchanged_outline = insert_user_outline(&db, unchanged_project).await;
    let unchanged_chapter = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id: unchanged_project,
            chapter_number: 1,
            existing_chapter_id: None,
            source_outline: Some(unchanged_outline.clone()),
            title: "第一章".to_string(),
            summary: "摘要".to_string(),
            content: "正文".to_string(),
            word_count: 2,
            now,
        },
    )
    .await
    .expect("unchanged source outline must remain linked");
    assert_eq!(unchanged_chapter.outline_id, Some(unchanged_outline.id));

    let changed_project = create_project(&db, "不冒认已修改章纲").await;
    let source_outline = insert_user_outline(&db, changed_project).await;
    let mut changed: chapter_outline::ActiveModel =
        chapter_outline::Entity::find_by_id(source_outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .expect("source outline must exist")
            .into();
    changed.title = Set(Some("生成期间修改后的章纲".to_string()));
    changed.updated_at = Set(source_outline.updated_at + Duration::seconds(1));
    changed
        .update(&db)
        .await
        .expect("outline update must succeed");
    let changed_chapter = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id: changed_project,
            chapter_number: 1,
            existing_chapter_id: None,
            source_outline: Some(source_outline),
            title: "第一章".to_string(),
            summary: "摘要".to_string(),
            content: "正文".to_string(),
            word_count: 2,
            now,
        },
    )
    .await
    .expect("outline changes must not discard generated content");
    assert_eq!(changed_chapter.outline_id, None);

    let added_project = create_project(&db, "不冒认生成期间新增章纲").await;
    let added_outline = insert_user_outline(&db, added_project).await;
    let added_chapter = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id: added_project,
            chapter_number: 1,
            existing_chapter_id: None,
            source_outline: None,
            title: "第一章".to_string(),
            summary: "摘要".to_string(),
            content: "正文".to_string(),
            word_count: 2,
            now,
        },
    )
    .await
    .expect("a newly added outline must not block generated content");
    assert_eq!(added_chapter.outline_id, None);
    assert!(
        chapter_outline::Entity::find_by_id(added_outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_some()
    );
}

#[tokio::test]
async fn generated_chapter_rejects_an_outline_from_another_target() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let source_project = create_project(&db, "章纲来源项目").await;
    let target_project = create_project(&db, "正文目标项目").await;
    let source_outline = insert_user_outline(&db, source_project).await;

    let error = finish_generated_chapter_content_record_in_transaction(
        &db,
        GeneratedChapterContentRecordInput {
            project_id: target_project,
            chapter_number: 1,
            existing_chapter_id: None,
            source_outline: Some(source_outline),
            title: "第一章".to_string(),
            summary: "摘要".to_string(),
            content: "正文".to_string(),
            word_count: 2,
            now: Utc::now().naive_utc(),
        },
    )
    .await
    .expect_err("cross-project outline source must be rejected");

    assert!(error.to_string().contains("来源章纲与目标章节不一致"));
    assert!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(target_project))
            .one(&db)
            .await
            .expect("chapter lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn importing_content_preserves_existing_outline_and_does_not_create_placeholder() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "保留章纲").await;
    let outline = insert_user_outline(&db, project_id).await;
    let imported = import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id,
            chapter_number: 1,
            content: "已有正文".to_string(),
            title: Some("第一章".to_string()),
        },
        "导入章节正文",
    )
    .await
    .expect("content import must succeed");
    assert_eq!(imported.outline_id, Some(outline.id));
    assert_eq!(
        chapter_outline_by_id(&db, outline.id)
            .await
            .expect("outline lookup must succeed")
            .expect("outline must remain"),
        outline
    );

    let project_without_outline = create_project(&db, "不造占位章纲").await;
    let imported_gap = import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id: project_without_outline,
            chapter_number: 7,
            content: "另一份正文".to_string(),
            title: Some("第七章".to_string()),
        },
        "导入章节正文",
    )
    .await
    .expect("content import without outline must succeed");
    assert_eq!(imported_gap.chapter_number, 7);
    assert!(
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_without_outline))
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn importing_content_validates_and_derives_persisted_fields() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "导入正文写入合同").await;

    let invalid_number = import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id,
            chapter_number: 0,
            content: "正文".to_string(),
            title: None,
        },
        "导入章节正文",
    )
    .await
    .expect_err("non-positive chapter number must fail");
    assert!(matches!(
        invalid_number,
        a4n_db::chapter_content::ChapterContentImportError::InvalidArgument(message)
            if message == "章节序号必须大于 0"
    ));

    let blank_content = import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id,
            chapter_number: 1,
            content: " \n ".to_string(),
            title: None,
        },
        "导入章节正文",
    )
    .await
    .expect_err("blank content must fail");
    assert!(matches!(
        blank_content,
        a4n_db::chapter_content::ChapterContentImportError::InvalidArgument(message)
            if message == "章节正文不能为空"
    ));

    let imported = import_chapter_content_record(
        &db,
        ImportChapterContentInput {
            project_id,
            chapter_number: 1,
            content: "四个字符".to_string(),
            title: Some("  第一章  ".to_string()),
        },
        "导入章节正文",
    )
    .await
    .expect("valid content must import");
    assert_eq!(imported.title.as_deref(), Some("第一章"));
    assert_eq!(imported.word_count, 4);
}

#[tokio::test]
async fn character_sync_uses_only_available_finished_chapters() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "稀疏章节角色同步来源").await;
    let now = Utc::now().naive_utc();
    for (chapter_number, content, status) in [
        (1, "", chapter::STATUS_DRAFT),
        (2, "生成中的旧正文", chapter::STATUS_GENERATING),
        (3, "已删除正文", chapter::STATUS_DELETED),
        (7, "唯一可用正文", chapter::STATUS_IMPORTED),
    ] {
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(None),
            content: Set(content.to_string()),
            word_count: Set(content.chars().count() as i32),
            status: Set(status.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("test chapter must be inserted");
    }

    let chapters = character_sync_chapters(&db, project_id)
        .await
        .expect("character sync chapters must load");
    assert_eq!(
        chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .collect::<Vec<_>>(),
        vec![7]
    );
}

#[tokio::test]
async fn retrospective_outline_yields_to_manual_edit() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "用户接管章纲").await;
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be inserted");
    let input = |fingerprint: &str, summary: &str| RetrospectiveChapterOutlineUpsertInput {
        project_id,
        chapter_number: 1,
        title: "第一章".to_string(),
        summary: summary.to_string(),
        key_scenes: Some(json!(["转折"])),
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: Some(json!(["林岚"])),
        word_count_target: 2,
        source_fingerprint_json: json!({"fingerprint": fingerprint}),
    };

    let created = upsert_retrospective_chapter_outline_checked(
        &db,
        input("first", "自动摘要"),
        a4n_db::project::chapter_source_snapshot_from_model(&chapter),
    )
    .await
    .expect("retrospective outline must be created");
    assert_eq!(created.action, RetrospectiveChapterOutlineAction::Created);
    assert!(
        formal_chapter_outline_by_project_number(&db, project_id, 1)
            .await
            .expect("formal outline lookup must succeed")
            .is_none()
    );

    update_chapter_outline_record(
        &db,
        ProjectChapterOutlineUpdateInput {
            project_id,
            outline_id: created.outline.id,
            expected_updated_at: created.outline.updated_at.to_string(),
            title: Some("用户标题".to_string()),
            summary: Some("用户摘要".to_string()),
            beats: None,
            key_scenes: None,
        },
    )
    .await
    .expect("manual edit must succeed");
    assert!(
        chapter_outline_provenance::Entity::find_by_id(created.outline.id)
            .one(&db)
            .await
            .expect("provenance lookup must succeed")
            .is_none()
    );
    assert_eq!(
        formal_chapter_outline_by_project_number(&db, project_id, 1)
            .await
            .expect("formal outline lookup must succeed")
            .expect("manual edit must turn the outline into a formal plan")
            .id,
        created.outline.id
    );

    let preserved = upsert_retrospective_chapter_outline_checked(
        &db,
        input("second", "不应覆盖"),
        a4n_db::project::chapter_source_snapshot_from_model(&chapter),
    )
    .await
    .expect("user-owned outline must be preserved");
    assert_eq!(
        preserved.action,
        RetrospectiveChapterOutlineAction::Preserved
    );
    assert_eq!(preserved.outline.title.as_deref(), Some("用户标题"));
    assert_eq!(preserved.outline.summary.as_deref(), Some("用户摘要"));
    assert!(
        delete_automatic_chapter_outlines_for_source_change(&db, project_id, 1)
            .await
            .expect("source-change cleanup must succeed")
            .is_empty()
    );
    assert!(
        chapter_outline::Entity::find_by_id(created.outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_some()
    );
}

#[tokio::test]
async fn source_change_removes_only_analysis_owned_outline() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "清理旧自动章纲").await;
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("旧正文".to_string()),
        word_count: Set(3),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be inserted");
    let automatic = upsert_retrospective_chapter_outline_checked(
        &db,
        RetrospectiveChapterOutlineUpsertInput {
            project_id,
            chapter_number: 1,
            title: "第一章".to_string(),
            summary: "旧正文摘要".to_string(),
            key_scenes: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: 3,
            source_fingerprint_json: json!({"fingerprint": "old"}),
        },
        a4n_db::project::chapter_source_snapshot_from_model(&chapter),
    )
    .await
    .expect("automatic outline must be created");

    assert_eq!(
        delete_automatic_chapter_outlines_for_source_change(&db, project_id, 1)
            .await
            .expect("source-change cleanup must succeed"),
        vec![automatic.outline.id]
    );
    assert!(
        chapter_outline::Entity::find_by_id(automatic.outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
    assert_eq!(
        chapter::Entity::find_by_id(chapter.id)
            .one(&db)
            .await
            .expect("chapter lookup must succeed")
            .expect("chapter must remain")
            .outline_id,
        None
    );
}

#[tokio::test]
async fn stale_chapter_source_rejects_object_discovery_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "物体发现复验正文来源").await;
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("主角捡起青铜钥匙".to_string()),
        word_count: Set(9),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be inserted");
    let source = a4n_db::project::chapter_source_snapshot_from_model(&chapter);

    let mut changed: chapter::ActiveModel = chapter.into();
    changed.content = Set("用户改成了没有钥匙的版本".to_string());
    changed.updated_at = Set(now + Duration::seconds(1));
    changed
        .update(&db)
        .await
        .expect("chapter rewrite must succeed");

    let error = upsert_object_profiles_with_generation_run_record(
        &db,
        project_id,
        vec![discovered_object()],
        vec![source],
        vec![],
        object_generation_run(project_id),
    )
    .await
    .expect_err("stale object discovery result must be rejected");
    assert!(matches!(
        error,
        AssetDbError::InvalidState { current, .. }
            if current == "chapter_source_changed"
    ));
    assert!(
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .expect("object profile lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn stale_chapter_source_rejects_scene_sync_write() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "场景同步复验正文来源").await;
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("主角进入旧书房".to_string()),
        word_count: Set(8),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be inserted");
    let source = a4n_db::project::chapter_source_snapshot_from_model(&chapter);

    let mut changed: chapter::ActiveModel = chapter.clone().into();
    changed.content = Set("用户改成了露天广场".to_string());
    changed.updated_at = Set(now + Duration::seconds(1));
    changed
        .update(&db)
        .await
        .expect("chapter rewrite must succeed");

    let error = replace_chapter_scene_sync_with_generation_run_records(
        &db,
        project_id,
        chapter.id,
        chapter.chapter_number,
        source,
        discovered_scene(),
        vec![],
        scene_generation_run(project_id),
    )
    .await
    .expect_err("stale scene sync result must be rejected");
    assert!(matches!(
        error,
        SceneProfileDbError::InvalidState { current, .. }
            if current == "chapter_source_changed"
    ));
    assert!(
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .expect("scene profile lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn character_discovery_profile_run_and_evidence_roll_back_together() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "角色发现事务原子性").await;
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("林岚与沈默进入旧仓库".to_string()),
        word_count: Set(10),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("source chapter must be inserted");
    let anchor = rebuild_chapter_content_anchor_records(&db, project_id, chapter.id)
        .await
        .expect("chapter anchors must be rebuilt")
        .remove(0);
    let tx = db.begin().await.expect("transaction must begin");

    let first = create_profile_with_generation_run_in(
        &tx,
        project_id,
        "林岚",
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_discovery_generation_run(project_id),
    )
    .await
    .expect("first discovered profile must be staged");
    merge_profile_evidence_records_batch(
        &tx,
        project_id,
        None,
        TextAnchorProfileKind::Character,
        vec![(
            first.id,
            vec![TextAnchorEvidenceInput {
                text_anchor_id: anchor.id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            }],
        )],
    )
    .await
    .expect("first evidence must be staged");

    let second = create_profile_with_generation_run_in(
        &tx,
        project_id,
        "沈默",
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_discovery_generation_run(project_id),
    )
    .await
    .expect("second discovered profile must be staged");
    merge_profile_evidence_records_batch(
        &tx,
        project_id,
        None,
        TextAnchorProfileKind::Character,
        vec![(
            second.id,
            vec![TextAnchorEvidenceInput {
                text_anchor_id: i64::MAX,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            }],
        )],
    )
    .await
    .expect_err("invalid second evidence must abort the batch");
    tx.rollback().await.expect("failed batch must roll back");

    assert_eq!(
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("character profiles must be counted"),
        0
    );
    assert_eq!(
        generation_run::Entity::find()
            .filter(generation_run::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("generation runs must be counted"),
        0
    );
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::ProfileId.is_in([first.id, second.id]))
            .count(&db)
            .await
            .expect("profile evidence must be counted"),
        0
    );
}

#[tokio::test]
async fn object_discovery_evidence_failure_rolls_back_profile_and_generation() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "物体证据事务原子性").await;
    let chapter = insert_source_chapter(&db, project_id, "主角捡起青铜钥匙").await;
    let anchor = rebuild_chapter_content_anchor_records(&db, project_id, chapter.id)
        .await
        .expect("chapter anchors must be rebuilt")
        .remove(0);

    let error = upsert_object_profiles_with_generation_run_record(
        &db,
        project_id,
        vec![discovered_object()],
        vec![a4n_db::project::chapter_source_snapshot_from_model(
            &chapter,
        )],
        vec![ObjectProfileEvidenceRecordInput {
            normalized_name: "青铜钥匙".to_string(),
            text_anchor_ids: vec![i64::MAX],
        }],
        object_generation_run(project_id),
    )
    .await
    .expect_err("invalid evidence must abort object discovery persistence");
    assert!(matches!(error, AssetDbError::InvalidArgument(_)));
    assert_eq!(
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("object profiles must be counted"),
        0
    );
    assert_eq!(
        generation_run::Entity::find()
            .filter(generation_run::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("generation runs must be counted"),
        0
    );
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::TextAnchorId.eq(anchor.id))
            .count(&db)
            .await
            .expect("object evidence must be counted"),
        0
    );
}

#[tokio::test]
async fn scene_sync_evidence_failure_rolls_back_profile_and_generation() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "场景证据事务原子性").await;
    let chapter = insert_source_chapter(&db, project_id, "主角进入旧书房").await;
    let anchor = rebuild_chapter_content_anchor_records(&db, project_id, chapter.id)
        .await
        .expect("chapter anchors must be rebuilt")
        .remove(0);

    let error = replace_chapter_scene_sync_with_generation_run_records(
        &db,
        project_id,
        chapter.id,
        chapter.chapter_number,
        a4n_db::project::chapter_source_snapshot_from_model(&chapter),
        discovered_scene(),
        vec![SceneProfileEvidenceRecordInput {
            scene_ref: "old-study".to_string(),
            text_anchor_ids: vec![i64::MAX],
        }],
        scene_generation_run(project_id),
    )
    .await
    .expect_err("invalid evidence must abort scene sync persistence");
    assert!(matches!(error, SceneProfileDbError::InvalidArgument(_)));
    assert_eq!(
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("scene profiles must be counted"),
        0
    );
    assert_eq!(
        generation_run::Entity::find()
            .filter(generation_run::Column::ProjectId.eq(project_id))
            .count(&db)
            .await
            .expect("generation runs must be counted"),
        0
    );
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::TextAnchorId.eq(anchor.id))
            .count(&db)
            .await
            .expect("scene evidence must be counted"),
        0
    );
}

#[tokio::test]
async fn chapter_source_change_invalidates_old_profile_evidence_without_deleting_profile() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "正文变化立即失效档案证据").await;
    let chapter = insert_source_chapter(&db, project_id, "主角捡起青铜钥匙").await;
    let first_anchor = rebuild_chapter_content_anchor_records(&db, project_id, chapter.id)
        .await
        .expect("chapter anchors must be rebuilt")
        .remove(0);
    let object = create_object_profile_record(&db, project_id, discovered_object())
        .await
        .expect("object profile must be created");
    let bind = |anchor_id| TextAnchorEvidenceInput {
        text_anchor_id: anchor_id,
        evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
    };
    merge_profile_evidence_records_batch(
        &db,
        project_id,
        None,
        TextAnchorProfileKind::Object,
        vec![(object.id, vec![bind(first_anchor.id)])],
    )
    .await
    .expect("old evidence must be bound");

    assert_eq!(
        delete_chapter_content_profile_evidence_records(&db, project_id, chapter.id)
            .await
            .expect("chapter evidence cleanup must succeed"),
        1
    );
    assert!(
        object_profile::Entity::find_by_id(object.id)
            .one(&db)
            .await
            .expect("object profile lookup must succeed")
            .is_some()
    );
    assert!(
        text_anchor::Entity::find_by_id(first_anchor.id)
            .one(&db)
            .await
            .expect("old anchor lookup must succeed")
            .is_some()
    );

    merge_profile_evidence_records_batch(
        &db,
        project_id,
        None,
        TextAnchorProfileKind::Object,
        vec![(object.id, vec![bind(first_anchor.id)])],
    )
    .await
    .expect("evidence must be rebound for rebuild regression");
    rebuild_chapter_content_anchor_records(&db, project_id, chapter.id)
        .await
        .expect("anchor rebuild must succeed");
    assert_eq!(
        profile_evidence::Entity::find()
            .filter(profile_evidence::Column::ProfileId.eq(object.id))
            .count(&db)
            .await
            .expect("profile evidence must be counted"),
        0
    );
}
