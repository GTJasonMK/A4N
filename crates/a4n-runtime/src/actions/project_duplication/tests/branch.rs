#[tokio::test]
async fn branch_text_project_record_copies_blueprint_graph_for_non_draft_branch() {
    let db = setup_db().await;
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;

    let branched = branch_text_project_record(
        &db,
        &source,
        "源项目 分支",
        ProjectStatus::BlueprintReady.as_str(),
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
async fn branch_text_project_record_rejects_non_draft_branch_without_blueprint_before_insert() {
    let db = setup_db().await;
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let err = branch_text_project_record(
        &db,
        &source,
        "坏分支",
        ProjectStatus::BlueprintReady.as_str(),
    )
    .await
    .expect_err("non-draft branch must not be created without source blueprint");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(matches!(
        err,
        ProjectDuplicateError::InvalidState { ref current, .. }
            if current.contains("缺少蓝图数据")
    ));
    assert_eq!(after, before);
}

#[tokio::test]
async fn branch_text_project_record_rejects_status_beyond_source_before_insert() {
    let db = setup_db().await;
    let source = insert_project(&db, ProjectStatus::BlueprintReady.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let before = novel_project::Entity::find().all(&db).await.unwrap().len();

    let err = branch_text_project_record(&db, &source, "未来分支", ProjectStatus::Writing.as_str())
        .await
        .expect_err("branch must not advance source status");
    let after = novel_project::Entity::find().all(&db).await.unwrap().len();

    assert!(matches!(
        err,
        ProjectDuplicateError::InvalidState {
            ref current,
            ref operation,
        } if current == ProjectStatus::BlueprintReady.as_str()
            && operation.contains(ProjectStatus::Writing.as_str())
    ));
    assert_eq!(after, before);
}

#[tokio::test]
async fn branch_text_project_record_copies_content_graph_for_writing_branch() {
    let db = setup_db().await;
    let mut source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    let (source_part, source_outline, source_chapter) =
        insert_outline_and_chapter(&db, source.id).await;
    source.current_chapter = 1;
    let mut active: novel_project::ActiveModel = source.clone().into();
    active.current_chapter = Set(1);
    source = active.update(&db).await.unwrap();

    let branched = branch_text_project_record(
        &db,
        &source,
        "源项目 写作分支",
        ProjectStatus::Writing.as_str(),
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
async fn branch_text_project_record_prunes_generated_data_for_draft_branch() {
    let db = setup_db().await;
    let source = insert_project(&db, ProjectStatus::Writing.as_str()).await;
    insert_blueprint_graph(&db, source.id).await;
    insert_outline_and_chapter(&db, source.id).await;

    let branched = branch_text_project_record(
        &db,
        &source,
        "源项目 草稿分支",
        ProjectStatus::Draft.as_str(),
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
