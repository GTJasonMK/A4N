#[tokio::test]
async fn duplicate_text_project_record_preserves_status_outlines_chapters_and_links() {
    let db = setup_db().await;
    let mut source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let (source_part, source_outline, source_chapter) =
        insert_outline_and_chapter(&db, source.id).await;
    source.current_chapter = 1;
    let mut active: novel_project::ActiveModel = source.clone().into();
    active.current_chapter = Set(1);
    active.import_analysis_progress = Set(Some("{\"phase\":\"completed\"}".to_string()));
    source = active.update(&db).await.unwrap();

    let duplicated = duplicate_text_project_record(&db, &source, 1, "源项目 副本")
        .await
        .expect("writing project should duplicate all core creative data");

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
    assert_eq!(duplicated.import_analysis_progress, None);
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
async fn duplicate_text_project_rejects_blueprint_graph_without_raw_blueprint() {
    let db = setup_db().await;
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let mut active: novel_project::ActiveModel = source.into();
    active.raw_blueprint = Set(None);
    let source = active.update(&db).await.unwrap();
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let error = duplicate_text_project_record(&db, &source, 1, "不一致副本")
        .await
        .expect_err("蓝图双事实源不一致时不得临时重建 raw_blueprint");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(matches!(
        error,
        ProjectDuplicateError::InvalidState { ref current, .. }
            if current.contains("存在性不一致")
    ));
    assert_eq!(after, before);
}

#[tokio::test]
async fn duplicate_text_project_record_rolls_back_when_chapter_outline_link_is_invalid() {
    let db = setup_db().await;
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

    let err = duplicate_text_project_record(&db, &source, 1, "坏副本")
        .await
        .expect_err("invalid chapter outline reference must reject duplicate");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(matches!(
        err,
        ProjectDuplicateError::InvalidState { ref current, .. }
            if current.contains("缺失的章节大纲")
    ));
    assert_eq!(after, before);
}
