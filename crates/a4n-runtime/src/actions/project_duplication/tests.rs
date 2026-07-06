use super::*;
use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    generation::{generation_spec, generation_spec_text_anchor, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project, project_derivation, project_source_snapshot, source_snapshot_chapter,
        text_anchor,
    },
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_core::state::status::ProjectStatus;
use chrono::NaiveDate;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::json;

async fn setup_db() -> Option<DatabaseConnection> {
    if std::env::var("A4N_TEST_DATABASE_URL").is_err() {
        eprintln!("skipping project duplication DB test: A4N_TEST_DATABASE_URL is not set");
        return None;
    }
    Some(a4n_migration::test_support::connect_test_database().await)
}

fn fixed_time() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 6, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
}

async fn insert_project(db: &DatabaseConnection, status: &str) -> novel_project::Model {
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("源项目".to_string()),
        description: Set("用于分支复制测试".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_sync_status: Set("current".to_string()),
        status: Set(status.to_string()),
        target_chapters: Set(80),
        current_chapter: Set(12),
        total_chapters: Set(Some(80)),
        chapters_per_part: Set(Some(20)),
        avatar_svg: Set(Some("<svg>avatar</svg>".to_string())),
        avatar_animal: Set(Some("wolf".to_string())),
        raw_blueprint: Set(Some("{\"source\":true}".to_string())),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_blueprint_graph(db: &DatabaseConnection, project_id: i64) {
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set("旧城疑云".to_string()),
        genre: Set("悬疑".to_string()),
        style: Set(Some("冷峻".to_string())),
        tone: Set(Some("紧张".to_string())),
        target_audience: Set(Some("成人".to_string())),
        one_sentence_summary: Set(Some("一封信引出真相。".to_string())),
        full_synopsis: Set(Some("完整梗概".to_string())),
        world_setting: Set(Some(json!({"city": "旧城"}))),
        current_version: Set("1.2".to_string()),
        needs_part_outlines: Set(true),
        total_chapters: Set(Some(80)),
        chapters_per_part: Set(Some(20)),
        avatar_svg: Set(Some("<svg>blueprint</svg>".to_string())),
        avatar_animal: Set(Some("fox".to_string())),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_version::ActiveModel {
        project_id: Set(project_id),
        version: Set("1.1".to_string()),
        change_description: Set(Some("保存旧版本".to_string())),
        blueprint_data: Set(json!({"title": "旧城疑云"})),
        world_setting_snapshot: Set(Some(json!({"city": "旧城"}))),
        characters_snapshot: Set(Some(json!([{"name": "林岚"}]))),
        relationships_snapshot: Set(Some(json!([]))),
        synopsis_snapshot: Set(Some("旧梗概".to_string())),
        created_at: Set(fixed_time()),
        created_by: Set(Some("user".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_character::ActiveModel {
        project_id: Set(project_id),
        name: Set("林岚".to_string()),
        character_type: Set("protagonist".to_string()),
        gender: Set(Some("女".to_string())),
        age: Set(Some("28".to_string())),
        appearance: Set(Some("黑色风衣".to_string())),
        personality: Set(Some("克制".to_string())),
        background: Set(Some("记者".to_string())),
        motivation: Set(Some("寻找真相".to_string())),
        arc_summary: Set(Some("从旁观者到揭露者".to_string())),
        relationship_to_protagonist: Set(None),
        arc_stages: Set(Some(json!(["怀疑", "行动"]))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    blueprint_relationship::ActiveModel {
        project_id: Set(project_id),
        relationship_type: Set("rival".to_string()),
        character_a: Set("林岚".to_string()),
        character_b: Set("沈默".to_string()),
        description: Set(Some("互相试探".to_string())),
        strength: Set(Some(7)),
        evolution: Set(Some(json!(["冲突", "合作"]))),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn insert_outline_and_chapter(
    db: &DatabaseConnection,
    project_id: i64,
) -> (part_outline::Model, chapter_outline::Model, chapter::Model) {
    let part = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("疑云".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(20),
        summary: Set(Some("第一部".to_string())),
        theme: Set(Some("真相".to_string())),
        key_events: Set(Some(json!(["收到密信"]))),
        character_focus: Set(Some(json!({"林岚": "追查"}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let outline = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(Some(part.id)),
        chapter_number: Set(1),
        title: Set(Some("密信".to_string())),
        summary: Set(Some("林岚收到密信。".to_string())),
        beats: Set(Some(json!(["收到信", "前往旧城"]))),
        key_scenes: Set(Some(json!(["报社"]))),
        pacing_score: Set(Some(7.5)),
        foreshadowing_setup: Set(Some(json!(["邮戳"]))),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(Some(json!(["林岚"]))),
        word_count_target: Set(Some(3000)),
        status: Set("completed".to_string()),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(Some(outline.id)),
        chapter_number: Set(1),
        title: Set(Some("密信".to_string())),
        content: Set("林岚收到一封没有署名的信。".to_string()),
        word_count: Set(14),
        status: Set("completed".to_string()),
        summary: Set(Some("密信出现。".to_string())),
        style_tags: Set(Some(json!(["悬疑"]))),
        analysis_data: Set(Some(json!({"foreshadowing": {"planted": ["邮戳"]}}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (part, outline, chapter)
}

#[tokio::test]
async fn branch_project_data_copies_blueprint_graph_for_non_draft_branch() {
    let Some(db) = setup_db().await else {
        return;
    };
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;

    let branched = branch_project_data(
        &db,
        &source,
        "源项目 分支",
        ProjectStatus::BlueprintReady.as_str(),
        fixed_time(),
    )
    .await
    .expect("branch should copy blueprint graph");

    let copied_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(branched.id))
        .one(&db)
        .await
        .unwrap()
        .expect("branched project should have blueprint");
    let copied_characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();
    let copied_relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(branched.status, ProjectStatus::BlueprintReady.as_str());
    assert_eq!(copied_blueprint.title, "旧城疑云");
    assert_eq!(
        copied_blueprint.world_setting,
        Some(json!({"city": "旧城"}))
    );
    assert_eq!(copied_characters.len(), 1);
    assert_eq!(copied_characters[0].name, "林岚");
    assert_eq!(copied_relationships.len(), 1);
    assert_eq!(copied_relationships[0].character_b, "沈默");
}

#[tokio::test]
async fn branch_project_data_rejects_non_draft_branch_without_blueprint_before_insert() {
    let Some(db) = setup_db().await else {
        return;
    };
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let err = branch_project_data(
        &db,
        &source,
        "坏分支",
        ProjectStatus::BlueprintReady.as_str(),
        fixed_time(),
    )
    .await
    .expect_err("non-draft branch must not be created without source blueprint");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(err.contains("缺少可复制的蓝图数据"));
    assert_eq!(after, before);
}

#[tokio::test]
async fn branch_project_data_rejects_status_beyond_source_before_insert() {
    let Some(db) = setup_db().await else {
        return;
    };
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let err = branch_project_data(
        &db,
        &source,
        "未来分支",
        ProjectStatus::Writing.as_str(),
        fixed_time(),
    )
    .await
    .expect_err("branch must not advance source status");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(err.contains("分支状态不能超过源项目当前状态"));
    assert_eq!(after, before);
}

#[tokio::test]
async fn branch_project_data_copies_content_graph_for_writing_branch() {
    let Some(db) = setup_db().await else {
        return;
    };
    let mut source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let (source_part, source_outline, source_chapter) =
        insert_outline_and_chapter(&db, source.id).await;
    source.current_chapter = 1;
    let mut active: novel_project::ActiveModel = source.clone().into();
    active.current_chapter = Set(1);
    source = active.update(&db).await.unwrap();

    let branched = branch_project_data(
        &db,
        &source,
        "源项目 写作分支",
        ProjectStatus::Writing.as_str(),
        fixed_time(),
    )
    .await
    .expect("writing branch should preserve content graph");

    let copied_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();
    let copied_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();
    let copied_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(branched.status, ProjectStatus::Writing.as_str());
    assert_eq!(branched.current_chapter, 1);
    assert_eq!(copied_parts.len(), 1);
    assert_ne!(copied_parts[0].id, source_part.id);
    assert_eq!(copied_outlines.len(), 1);
    assert_eq!(copied_outlines[0].part_id, Some(copied_parts[0].id));
    assert_ne!(copied_outlines[0].id, source_outline.id);
    assert_eq!(copied_chapters.len(), 1);
    assert_eq!(copied_chapters[0].outline_id, Some(copied_outlines[0].id));
    assert_ne!(copied_chapters[0].id, source_chapter.id);
    assert_eq!(copied_chapters[0].content, source_chapter.content);
}

#[tokio::test]
async fn branch_project_data_prunes_generated_data_for_draft_branch() {
    let Some(db) = setup_db().await else {
        return;
    };
    let source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    insert_outline_and_chapter(&db, source.id).await;

    let branched = branch_project_data(
        &db,
        &source,
        "源项目 草稿分支",
        ProjectStatus::Draft.as_str(),
        fixed_time(),
    )
    .await
    .expect("draft branch should be created without generated data");

    let copied_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(branched.id))
        .one(&db)
        .await
        .unwrap();
    let copied_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();
    let copied_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();
    let copied_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(branched.id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(branched.status, ProjectStatus::Draft.as_str());
    assert_eq!(branched.current_chapter, 0);
    assert_eq!(branched.total_chapters, None);
    assert_eq!(branched.raw_blueprint, None);
    assert!(copied_blueprint.is_none());
    assert!(copied_parts.is_empty());
    assert!(copied_outlines.is_empty());
    assert!(copied_chapters.is_empty());
}

#[tokio::test]
async fn duplicate_project_data_preserves_status_outlines_chapters_and_links() {
    let Some(db) = setup_db().await else {
        return;
    };
    let mut source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let (source_part, source_outline, source_chapter) =
        insert_outline_and_chapter(&db, source.id).await;
    source.current_chapter = 1;
    let mut active: novel_project::ActiveModel = source.clone().into();
    active.current_chapter = Set(1);
    source = active.update(&db).await.unwrap();

    let txn = db.begin().await.unwrap();
    let duplicated = duplicate_project_data(&txn, &source, 1, "源项目 副本", fixed_time())
        .await
        .expect("writing project should duplicate all core creative data");
    txn.commit().await.unwrap();

    let copied_versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let copied_parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let copied_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let copied_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();

    assert_ne!(duplicated.id, source.id);
    assert_eq!(duplicated.title, "源项目 副本");
    assert_eq!(duplicated.status, ProjectStatus::Writing.as_str());
    assert_eq!(duplicated.current_chapter, 1);
    assert_eq!(copied_versions.len(), 1);
    assert_eq!(copied_parts.len(), 1);
    assert_eq!(copied_parts[0].part_number, source_part.part_number);
    assert_ne!(copied_parts[0].id, source_part.id);
    assert_eq!(copied_outlines.len(), 1);
    assert_eq!(
        copied_outlines[0].chapter_number,
        source_outline.chapter_number
    );
    assert_eq!(copied_outlines[0].part_id, Some(copied_parts[0].id));
    assert_ne!(copied_outlines[0].id, source_outline.id);
    assert_eq!(copied_chapters.len(), 1);
    assert_eq!(
        copied_chapters[0].chapter_number,
        source_chapter.chapter_number
    );
    assert_eq!(copied_chapters[0].outline_id, Some(copied_outlines[0].id));
    assert_eq!(copied_chapters[0].status, "completed");
    assert_eq!(copied_chapters[0].content, source_chapter.content);
    assert_eq!(copied_chapters[0].style_tags, Some(json!(["悬疑"])));
    assert_eq!(
        copied_chapters[0].analysis_data,
        Some(json!({"foreshadowing": {"planted": ["邮戳"]}}))
    );
}

#[tokio::test]
async fn duplicate_project_data_rolls_back_when_chapter_outline_link_is_invalid() {
    let Some(db) = setup_db().await else {
        return;
    };
    let source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    chapter::ActiveModel {
        project_id: Set(source.id),
        outline_id: Set(Some(999_999)),
        chapter_number: Set(1),
        title: Set(Some("坏章节".to_string())),
        content: Set("章节引用了不存在的大纲。".to_string()),
        word_count: Set(12),
        status: Set("completed".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let txn = db.begin().await.unwrap();
    let err = duplicate_project_data(&txn, &source, 1, "坏副本", fixed_time())
        .await
        .expect_err("invalid chapter outline reference must reject duplicate");
    drop(txn);
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(err.contains("引用的章节大纲不存在"));
    assert_eq!(after, before);
}

#[tokio::test]
async fn duplicate_derived_project_data_copies_snapshot_without_copying_root_chapters() {
    let Some(db) = setup_db().await else {
        return;
    };
    let root = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    let root_chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("源章节".to_string())),
        content: Set("根文本章节正文。".to_string()),
        word_count: Set(8),
        status: Set("completed".to_string()),
        summary: Set(Some("根文本摘要".to_string())),
        style_tags: Set(None),
        analysis_data: Set(Some(json!({"k": "v"}))),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id),
        source_version_hash: Set("source-hash".to_string()),
        blueprint_version_id: Set(None),
        chapter_scope_json: Set(json!({"start": 1, "end": 1})),
        chapter_hashes_json: Set(json!([{"chapter_number": 1}])),
        profile_hashes_json: Set(json!([])),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let snapshot_chapter = source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(root_chapter.id),
        chapter_number: Set(1),
        title: Set(Some("源章节".to_string())),
        content: Set(root_chapter.content.clone()),
        summary: Set(root_chapter.summary.clone()),
        analysis_data_json: Set(root_chapter.analysis_data.clone()),
        content_hash: Set("content-hash".to_string()),
        analysis_hash: Set(Some("analysis-hash".to_string())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("视频项目".to_string()),
        description: Set("派生视频".to_string()),
        project_type: Set(root.project_type.clone()),
        project_kind: Set("video".to_string()),
        source_mode: Set("derived".to_string()),
        root_project_id: Set(Some(root.id)),
        parent_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set("current".to_string()),
        derived_project_status: Set(Some("draft".to_string())),
        status: Set(ProjectStatus::Draft.as_str().to_string()),
        target_chapters: Set(1),
        current_chapter: Set(0),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(root.chapters_per_part),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let anchor = text_anchor::ActiveModel {
        project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        source_kind: Set("chapter_content".to_string()),
        source_id: Set(snapshot_chapter.id),
        paragraph_index: Set(0),
        start_offset: Set(0),
        end_offset: Set(6),
        quote: Set("根文本章节正文。".to_string()),
        quote_hash: Set("quote-hash".to_string()),
        source_version_hash: Set("content-hash".to_string()),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let segment = video_segment::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        segment_type: Set("normal".to_string()),
        order_index: Set(1),
        title: Set("镜头 1".to_string()),
        duration_secs: Set(6.0),
        source_generation_spec_id: Set(None),
        result_media_asset_id: Set(None),
        previous_segment_id: Set(None),
        next_segment_id: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let spec = generation_spec::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        target_kind: Set("video_segment".to_string()),
        target_entity_kind: Set("segment".to_string()),
        target_entity_id: Set(Some(segment.id)),
        status: Set("draft".to_string()),
        source_version_hash: Set("source-hash".to_string()),
        params_json: Set(json!({"duration_secs": 6})),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    generation_spec_text_anchor::ActiveModel {
        generation_spec_id: Set(spec.id),
        text_anchor_id: Set(anchor.id),
        role: Set("direct_content".to_string()),
    }
    .insert(&db)
    .await
    .unwrap();
    let prompt = prompt_artifact::ActiveModel {
        generation_spec_id: Set(spec.id),
        prompt_kind: Set("video_prompt".to_string()),
        prompt_text: Set("镜头自然语言提示词".to_string()),
        renderer_version: Set("test".to_string()),
        edited_by_user: Set(true),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let media_dir =
        std::path::Path::new("/tmp").join(format!("a4n_duplication_media_{}", std::process::id()));
    std::fs::create_dir_all(&media_dir).unwrap();
    let segment_relative = format!(
        "{}/segment.mp4",
        media_dir.file_name().unwrap().to_string_lossy()
    );
    let preview_relative = format!(
        "{}/preview.mp4",
        media_dir.file_name().unwrap().to_string_lossy()
    );
    std::fs::write(
        std::path::Path::new("/tmp").join(&segment_relative),
        b"segment",
    )
    .unwrap();
    std::fs::write(
        std::path::Path::new("/tmp").join(&preview_relative),
        b"preview",
    )
    .unwrap();
    let asset = media_asset::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        media_kind: Set("video".to_string()),
        origin: Set("imported".to_string()),
        file_path: Set(Some(segment_relative)),
        thumbnail_path: Set(None),
        width: Set(None),
        height: Set(None),
        duration_secs: Set(Some(6.0)),
        fps: Set(None),
        file_size: Set(None),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let preview_asset = media_asset::ActiveModel {
        project_id: Set(derived.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        media_kind: Set("video".to_string()),
        origin: Set("assembled_preview".to_string()),
        file_path: Set(Some(preview_relative)),
        thumbnail_path: Set(None),
        width: Set(None),
        height: Set(None),
        duration_secs: Set(Some(6.0)),
        fps: Set(None),
        file_size: Set(None),
        checksum: Set(None),
        status: Set("completed".to_string()),
        error_message: Set(None),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(Some(spec.id)),
        prompt_artifact_id: Set(Some(prompt.id)),
        provider_job_id: Set(None),
        source_media_asset_id: Set(None),
        source_external_path: Set(None),
        imported_from_user: Set(true),
        request_snapshot_json: Set(Some(json!({"provider": "test"}))),
        created_at: Set(fixed_time()),
    }
    .insert(&db)
    .await
    .unwrap();
    a4n_core::models::generation::generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(spec.id),
        media_asset_id: Set(asset.id),
        role: Set("style_reference".to_string()),
    }
    .insert(&db)
    .await
    .unwrap();
    video_segment::ActiveModel {
        id: Set(segment.id),
        source_generation_spec_id: Set(Some(spec.id)),
        result_media_asset_id: Set(Some(asset.id)),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .update(&db)
    .await
    .unwrap();
    media_binding::ActiveModel {
        media_asset_id: Set(asset.id),
        target_kind: Set("video_segment".to_string()),
        target_id: Set(segment.id),
        role: Set("generated_result".to_string()),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let timeline = assembly_timeline::ActiveModel {
        project_id: Set(derived.id),
        source_chapter_id: Set(Some(root_chapter.id)),
        source_snapshot_chapter_id: Set(Some(snapshot_chapter.id)),
        version: Set(1),
        status: Set("preview_ready".to_string()),
        preview_media_asset_id: Set(Some(preview_asset.id)),
        preview_cache_key: Set(Some("cache-key".to_string())),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assembly_clip::ActiveModel {
        timeline_id: Set(timeline.id),
        media_asset_id: Set(asset.id),
        order_index: Set(1),
        trim_start_secs: Set(0.0),
        trim_end_secs: Set(6.0),
        volume: Set(1.0),
        transition_kind: Set("cut".to_string()),
        source_segment_id: Set(Some(segment.id)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    project_derivation::ActiveModel {
        root_project_id: Set(root.id),
        derived_project_id: Set(derived.id),
        derived_kind: Set("video".to_string()),
        source_snapshot_id: Set(snapshot.id),
        chapter_scope_json: Set(json!({"start": 1, "end": 1})),
        creation_options_json: Set(json!({"style": "film"})),
        created_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let txn = db.begin().await.unwrap();
    let mut copied_media_files = Vec::new();
    let duplicated = duplicate_derived_project_data(
        &txn,
        &derived,
        1,
        "视频项目 副本",
        fixed_time(),
        std::path::Path::new("/tmp"),
        &mut copied_media_files,
    )
    .await
    .expect("derived project should duplicate snapshot and derivation");
    txn.commit().await.unwrap();

    let duplicated_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let duplicated_derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("duplicated derived project should have derivation");
    let duplicated_snapshot_id = duplicated
        .source_snapshot_id
        .expect("duplicated project should reference snapshot");
    let duplicated_snapshot_chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(duplicated_snapshot_id))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(duplicated.project_kind, "video");
    assert_eq!(duplicated.root_project_id, Some(root.id));
    assert_ne!(duplicated_snapshot_id, snapshot.id);
    assert_eq!(
        duplicated_derivation.source_snapshot_id,
        duplicated_snapshot_id
    );
    assert!(duplicated_chapters.is_empty());
    assert_eq!(duplicated_snapshot_chapters.len(), 1);
    assert_eq!(
        duplicated_snapshot_chapters[0].content,
        root_chapter.content
    );

    let copied_segment = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("video segment should be copied");
    let copied_spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("generation spec should be copied");
    let copied_assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(duplicated.id))
        .all(&db)
        .await
        .unwrap();
    let copied_asset = copied_assets
        .iter()
        .find(|asset| asset.origin == "imported")
        .expect("segment media asset should be copied");
    let copied_preview_asset = copied_assets
        .iter()
        .find(|asset| asset.origin == "assembled_preview")
        .expect("preview media asset should be copied");
    let copied_prompt = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(copied_spec.id))
        .one(&db)
        .await
        .unwrap()
        .expect("prompt artifact should be copied");
    let copied_anchor_ref = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(copied_spec.id))
        .one(&db)
        .await
        .unwrap()
        .expect("text anchor ref should be copied");
    let copied_anchor = text_anchor::Entity::find_by_id(copied_anchor_ref.text_anchor_id)
        .one(&db)
        .await
        .unwrap()
        .expect("text anchor should be copied");
    let copied_provenance = media_provenance::Entity::find_by_id(copied_asset.id)
        .one(&db)
        .await
        .unwrap()
        .expect("media provenance should be copied");
    let copied_timeline = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(duplicated.id))
        .one(&db)
        .await
        .unwrap()
        .expect("assembly timeline should be copied");
    let copied_clip = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(copied_timeline.id))
        .one(&db)
        .await
        .unwrap()
        .expect("assembly clip should be copied");
    let copied_binding = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(copied_asset.id))
        .one(&db)
        .await
        .unwrap()
        .expect("media binding should be copied");

    assert_eq!(copied_spec.source_snapshot_id, Some(duplicated_snapshot_id));
    assert_eq!(
        copied_spec.source_snapshot_chapter_id,
        Some(duplicated_snapshot_chapters[0].id)
    );
    assert_eq!(copied_spec.target_entity_id, Some(copied_segment.id));
    assert_eq!(
        copied_segment.source_generation_spec_id,
        Some(copied_spec.id)
    );
    assert_eq!(copied_segment.result_media_asset_id, Some(copied_asset.id));
    assert_eq!(copied_prompt.prompt_text, "镜头自然语言提示词");
    assert!(copied_prompt.edited_by_user);
    assert_eq!(
        copied_anchor.source_snapshot_id,
        Some(duplicated_snapshot_id)
    );
    assert_eq!(
        copied_anchor.source_snapshot_chapter_id,
        Some(duplicated_snapshot_chapters[0].id)
    );
    assert_eq!(copied_provenance.generation_spec_id, Some(copied_spec.id));
    assert_eq!(copied_provenance.prompt_artifact_id, Some(copied_prompt.id));
    assert_eq!(
        copied_timeline.preview_media_asset_id,
        Some(copied_preview_asset.id)
    );
    assert_eq!(copied_clip.media_asset_id, copied_asset.id);
    assert_eq!(copied_clip.source_segment_id, Some(copied_segment.id));
    assert_eq!(copied_binding.target_id, copied_segment.id);
}
