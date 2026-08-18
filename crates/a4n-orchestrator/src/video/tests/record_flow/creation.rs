#[tokio::test]
async fn create_video_record_rejects_missing_project_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = crate::video::test_video_orchestrator(db.clone()).await;

    let err = service
        .create_video_record(video_record_input(404))
        .await
        .expect_err("missing project must not create orphan video");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "项目不存在: 404"
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn create_video_record_persists_explicit_workflow_mode() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db).await;
    let mut input = video_record_input(project.id);
    input.workflow_mode = Some("first_frame_chain".to_string());

    let video = service.create_video_record(input).await.unwrap();

    assert_eq!(video.workflow_mode.as_deref(), Some("first_frame_chain"));
}

#[tokio::test]
async fn create_video_record_rejects_missing_workflow_mode_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.workflow_mode = None;

    let err = service
        .create_video_record(input)
        .await
        .expect_err("new video records must belong to an explicit workflow mode");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "视频记录缺少 workflow_mode"
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn create_video_record_rejects_invalid_workflow_mode_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.workflow_mode = Some("shared".to_string());

    let err = service
        .create_video_record(input)
        .await
        .expect_err("invalid workflow mode must be visible");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "视频记录 workflow_mode 无效: shared"
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn create_video_record_rejects_chapter_from_another_project_without_insert() {
    let db = Arc::new(setup_db().await);
    let first_project = insert_video_project(db.as_ref(), "项目一").await;
    let second_project = insert_video_project(db.as_ref(), "项目二").await;
    let other_chapter = insert_video_chapter(db.as_ref(), second_project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(first_project.id);
    input.chapter_id = Some(other_chapter.id);

    let err = service
        .create_video_record(input)
        .await
        .expect_err("cross-project chapter binding must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!(
                "章节 #{} 不属于文本项目 {}",
                other_chapter.id, first_project.id
            )
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn create_video_record_rejects_source_image_from_another_project_without_insert() {
    let db = Arc::new(setup_db().await);
    let first_project = insert_video_project(db.as_ref(), "项目一").await;
    let second_project = insert_video_project(db.as_ref(), "项目二").await;
    let source_image = insert_generated_image(db.as_ref(), second_project.id).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(first_project.id);
    input.source_image_id = Some(source_image.id);

    let err = service
        .create_video_record(input)
        .await
        .expect_err("cross-project source image must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!("源图片 #{} 不属于项目 {}", source_image.id, first_project.id)
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn create_video_record_rejects_source_video_from_another_project_without_insert() {
    let db = Arc::new(setup_db().await);
    let first_project = insert_video_project(db.as_ref(), "项目一").await;
    let second_project = insert_video_project(db.as_ref(), "项目二").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let source_video = service
        .create_video_record(video_record_input(second_project.id))
        .await
        .unwrap();
    let mut input = video_record_input(first_project.id);
    input.source_video_id = Some(source_video.id);

    let err = service
        .create_video_record(input)
        .await
        .expect_err("cross-project source video must be rejected");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!("源视频 #{} 不属于项目 {}", source_video.id, first_project.id)
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 1);
}

#[tokio::test]
async fn create_prompt_video_record_starts_prompt_and_generation_run_together() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);

    let video = service
        .create_prompt_video_record_with_generation_run(
            input,
            video_generation_run_input(project.id),
            prompt.id,
        )
        .await
        .expect("video, prompt lock, and generation run should commit together");

    assert_eq!(video.status, "running");
    assert_eq!(video.video_type, generated_video::VIDEO_TYPE_CHAPTER);
    assert_eq!(generated_video_count(db.as_ref()).await, 1);

    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "running");
    assert_eq!(stored_prompt.generated_video_id, None);

    let run = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project.id))
        .filter(generation_run::Column::Workflow.eq("generate_chapter_video"))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("generation run should be recorded");
    assert_eq!(run.status, "running");

    let target = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::GenerationRunId.eq(run.id))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("generation run should target the created video");
    assert_eq!(target.target_table, "generated_video");
    assert_eq!(
        target.target_kind,
        generated_video::VIDEO_TYPE_CHAPTER
    );
    assert_eq!(target.target_id, video.id);
    assert_eq!(target.role, "output");
}

#[tokio::test]
async fn create_prompt_video_record_rejects_prompt_from_another_chapter_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let video_chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let other_chapter = insert_video_chapter(db.as_ref(), project.id, 2).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, other_chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);

    let err = service
        .create_prompt_video_record_with_generation_run(
            input,
            video_generation_run_input(project.id),
            prompt.id,
        )
        .await
        .expect_err("prompt from another chapter must reject the whole start transaction");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains(&format!("不属于提示词章节 {}", other_chapter.id))
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
    assert_eq!(
        generation_run::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        generation_run_target::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, None);
}

#[tokio::test]
async fn create_prompt_video_record_rejects_linked_prompt_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, Some(777)).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);

    let err = service
        .create_prompt_video_record_with_generation_run(
            input,
            video_generation_run_input(project.id),
            prompt.id,
        )
        .await
        .expect_err("generated video linked prompt must reject the whole start transaction");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("已存在由该视频提示词生成的视频")
                && message.contains("请先删除对应视频")
    ));
    assert_eq!(generated_video_count(db.as_ref()).await, 0);
    assert_eq!(
        generation_run::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        generation_run_target::Entity::find()
            .count(db.as_ref())
            .await
            .unwrap(),
        0
    );
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, Some(777));
}
