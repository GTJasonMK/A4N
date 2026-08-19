#[tokio::test]
async fn list_videos_by_missing_chapter_returns_not_found() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db).await;

    let err = service
        .ensure_chapter_belongs_to_project(project.id, 505)
        .await
        .expect_err("missing chapter must be visible");
    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "章节不存在: 505"
    ));
}

#[tokio::test]
async fn video_update_uses_the_public_nullable_patch_contract() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut record = video_record_input(project.id);
    record.negative_prompt = Some("avoid blur".to_string());
    let video = service.create_video_record(record).await.unwrap();
    a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
        db.as_ref(),
        video.id,
        "projects/1/videos/video.mp4",
        None,
        1024,
        None,
        None,
    )
    .await
    .unwrap();

    let updated = service
        .update_video_record_validated(
            video.id,
            UpdateGeneratedVideoInput {
                prompt: None,
                negative_prompt: Some(None),
                is_favorite: Some(true),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.negative_prompt, None);
    assert!(updated.is_favorite);

    let error = service
        .update_video_record_validated(
            video.id,
            UpdateGeneratedVideoInput {
                prompt: None,
                negative_prompt: None,
                is_favorite: None,
            },
        )
        .await
        .expect_err("empty patches must not update timestamps");
    assert!(matches!(
        error,
        AppError::InvalidArgument(message) if message == "视频更新内容不能为空"
    ));
}

#[tokio::test]
async fn failed_video_records_remain_listed_and_deletable_assets() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let failed = a4n_db::video::mark_generated_video_error_status_and_prompt_record(
        db.as_ref(),
        video.id,
        None,
        "failed",
        "provider failed",
    )
    .await
    .map_err(crate::video::video_db_error)
    .unwrap();

    assert_eq!(failed.status, "failed");
    service
        .ensure_chapter_belongs_to_project(project.id, chapter.id)
        .await
        .unwrap();
    let listed = a4n_db::video::generated_videos_for_chapter(db.as_ref(), project.id, chapter.id)
        .await
        .unwrap();
    assert_eq!(
        listed.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![video.id]
    );

    a4n_db::video::delete_generated_video_record_with_cleanup(db.as_ref(), video.id)
        .await
        .map_err(crate::video::video_db_error)
        .unwrap();

    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn cancelled_video_rejects_late_job_waiting_and_completion_updates() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let video = service
        .create_video_record(video_record_input(project.id))
        .await
        .unwrap();
    let cancelled = a4n_db::video::mark_generated_video_error_status_and_prompt_record(
        db.as_ref(),
        video.id,
        None,
        "cancelled",
        "用户取消",
    )
    .await
    .unwrap();
    assert_eq!(cancelled.status, "cancelled");

    for error in [
        a4n_db::video::mark_generated_video_job_and_prompt_record(
            db.as_ref(),
            video.id,
            "late-job",
            None,
            None,
        )
        .await
        .expect_err("迟到 job 不能覆盖 cancelled"),
        a4n_db::video::update_generated_video_waiting_status_and_prompt_record(
            db.as_ref(),
            video.id,
            None,
            "running",
        )
        .await
        .expect_err("迟到进度不能覆盖 cancelled"),
        a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
            db.as_ref(),
            video.id,
            "generated/late.mp4",
            None,
            8,
            None,
            None,
        )
        .await
        .expect_err("迟到结果不能覆盖 cancelled"),
    ] {
        assert!(matches!(
            error,
            a4n_db::video::VideoDbError::InvalidState { current, .. }
                if current == "cancelled"
        ));
    }

    let stored = generated_video::Entity::find_by_id(video.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, "cancelled");
    assert_eq!(stored.provider_job_id, None);
    assert_eq!(stored.file_path, None);
}

#[tokio::test]
async fn mark_job_rejects_blank_provider_job_id_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let video = service
        .create_video_record(video_record_input(project.id))
        .await
        .unwrap();

    let err = a4n_db::video::mark_generated_video_job_and_prompt_record(
        db.as_ref(),
        video.id,
        " \n\t ",
        None,
        None,
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect_err("blank provider job id must not be saved");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "provider_job_id 不能为空"
    ));
    let stored = generated_video::Entity::find_by_id(video.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.provider_job_id, None);
}

#[tokio::test]
async fn mark_job_updates_video_and_matching_prompt_together() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = a4n_db::video::mark_generated_video_job_and_prompt_record(
        db.as_ref(),
        video.id,
        " provider-job ",
        Some(json!({"submitted": true})),
        Some(prompt.id),
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect("provider job and prompt should enter running together");

    assert_eq!(updated.status, "running");
    assert_eq!(updated.provider_job_id.as_deref(), Some("provider-job"));
    assert_eq!(
        updated
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("provider_response")),
        Some(&json!({"submitted": true}))
    );

    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "running");
}

#[tokio::test]
async fn waiting_status_updates_video_and_matching_prompt_together() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = a4n_db::video::update_generated_video_waiting_status_and_prompt_record(
        db.as_ref(),
        video.id,
        Some(prompt.id),
        "pending",
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect("waiting video and prompt should update together");

    assert_eq!(updated.status, "pending");
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "running");
}

#[tokio::test]
async fn mark_failed_rejects_prompt_from_another_chapter_without_status_changes() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let video_chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let other_chapter = insert_video_chapter(db.as_ref(), project.id, 2).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, other_chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let err = a4n_db::video::mark_generated_video_error_status_and_prompt_record(
        db.as_ref(),
        video.id,
        Some(prompt.id),
        "failed",
        "provider failed",
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect_err("failure status must not be propagated to a prompt from another chapter");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!("视频 #{} 不属于提示词章节 {}", video.id, other_chapter.id)
    ));
    let stored_video = generated_video::Entity::find_by_id(video.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_video.status, "running");
    assert_eq!(stored_video.error_message, None);
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, None);
}

#[tokio::test]
async fn mark_failed_updates_video_and_matching_prompt_together() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let failed = a4n_db::video::mark_generated_video_error_status_and_prompt_record(
        db.as_ref(),
        video.id,
        Some(prompt.id),
        "failed",
        " provider failed with complete diagnostic context ",
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect("matching prompt and video should fail together");

    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.error_message.as_deref(),
        Some("provider failed with complete diagnostic context")
    );

    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "failed");
    assert_eq!(stored_prompt.generated_video_id, None);
}

#[tokio::test]
async fn mark_completed_rejects_prompt_from_another_chapter_without_partial_completion() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let video_chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let other_chapter = insert_video_chapter(db.as_ref(), project.id, 2).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, other_chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let err = a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
        db.as_ref(),
        video.id,
        "projects/1/videos/chapter_1/video.mp4",
        None,
        1024,
        None,
        Some(prompt.id),
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect_err("completion must not be committed before prompt linkage validation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!("视频 #{} 不属于提示词章节 {}", video.id, other_chapter.id)
    ));
    let stored_video = generated_video::Entity::find_by_id(video.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_video.status, "running");
    assert_eq!(stored_video.file_path, None);
    assert_eq!(stored_video.file_size, None);
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, None);
}

#[tokio::test]
async fn mark_completed_updates_video_and_matching_prompt_together() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
        db.as_ref(),
        video.id,
        "projects/1/videos/chapter_1/video.mp4",
        Some("projects/1/videos/chapter_1/thumb.jpg"),
        2048,
        Some(json!({"job": "done"})),
        Some(prompt.id),
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect("matching prompt and video should be completed together");

    assert_eq!(updated.status, "completed");
    assert_eq!(
        updated.file_path,
        Some("projects/1/videos/chapter_1/video.mp4".to_string())
    );
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, Some(video.id));
}

#[tokio::test]
async fn mark_completed_accepts_video_project_prompt_for_root_chapter() {
    let db = Arc::new(setup_db().await);
    let root_project = insert_video_project(db.as_ref(), "根文本项目").await;
    let root_chapter = insert_video_chapter(db.as_ref(), root_project.id, 1).await;
    let video_project =
        insert_derived_video_project(db.clone(), root_project.id, "视频派生项目").await;
    let prompt = insert_video_prompt(db.as_ref(), video_project.id, root_chapter.id, None).await;
    let service = crate::video::test_video_orchestrator(db.clone()).await;
    let mut input = video_record_input(video_project.id);
    input.chapter_id = Some(root_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = a4n_db::video::mark_generated_video_completed_and_link_prompt_record(
        db.as_ref(),
        video.id,
        "projects/2/videos/chapter_1/imported.mp4",
        None,
        4096,
        Some(json!({"source": "manual_import"})),
        Some(prompt.id),
    )
    .await
    .map_err(crate::video::video_db_error)
    .expect("video project prompt should accept its root text chapter");

    assert_eq!(updated.status, "completed");
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.project_id, video_project.id);
    assert_eq!(stored_prompt.chapter_id, root_chapter.id);
    assert_eq!(stored_prompt.generated_video_id, Some(video.id));
}
