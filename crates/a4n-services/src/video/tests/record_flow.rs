use super::*;

fn video_generation_run_input(project_id: i64) -> RecordStartedGenerationRunInput {
    RecordStartedGenerationRunInput {
        project_id,
        workflow: "generate_chapter_video".to_string(),
        step_key: "video_provider".to_string(),
        title: "章节视频 provider 生成".to_string(),
        model_kind: "video".to_string(),
        provider: Some("test-provider".to_string()),
        model: Some("test-model".to_string()),
        prompt_template_names_json: None,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: None,
        input_variables_json: None,
        source_refs_json: None,
        media_refs_json: None,
        request_json: Some(json!({"request": true})),
        targets: Vec::new(),
    }
}

#[tokio::test]
async fn create_video_record_rejects_missing_project_without_insert() {
    let db = Arc::new(setup_db().await);
    let service = VideoService::new(db.clone());

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
    let service = VideoService::new(db);
    let mut input = video_record_input(project.id);
    input.workflow_mode = Some("first_frame_chain".to_string());

    let video = service.create_video_record(input).await.unwrap();

    assert_eq!(video.workflow_mode.as_deref(), Some("first_frame_chain"));
}

#[tokio::test]
async fn create_video_record_rejects_missing_workflow_mode_without_insert() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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
    assert_eq!(video.video_type, "chapter_video");
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
    assert_eq!(target.target_kind, "chapter_video");
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
    let service = VideoService::new(db.clone());
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
    let service = VideoService::new(db.clone());
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

#[tokio::test]
async fn list_videos_by_missing_chapter_returns_not_found() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = VideoService::new(db);

    let err = service
        .list_by_chapter(project.id, 505)
        .await
        .expect_err("missing chapter must be visible");
    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "章节不存在: 505"
    ));
}

#[tokio::test]
async fn failed_video_records_remain_listed_and_deletable_assets() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let failed = service
        .mark_failed_and_update_prompt(video.id, None, "provider failed")
        .await
        .unwrap();

    assert_eq!(failed.status, "failed");
    let listed = service
        .list_by_chapter(project.id, chapter.id)
        .await
        .unwrap();
    assert_eq!(
        listed.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![video.id]
    );

    service.delete(video.id).await.unwrap();

    assert_eq!(generated_video_count(db.as_ref()).await, 0);
}

#[tokio::test]
async fn mark_job_rejects_blank_provider_job_id_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = VideoService::new(db.clone());
    let video = service
        .create_video_record(video_record_input(project.id))
        .await
        .unwrap();

    let err = service
        .mark_job(video.id, " \n\t ", None)
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = service
        .mark_job_and_update_prompt(
            video.id,
            " provider-job ",
            Some(json!({"submitted": true})),
            Some(prompt.id),
        )
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = service
        .update_waiting_status_and_prompt(video.id, Some(prompt.id), "pending")
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let err = service
        .mark_failed_and_update_prompt(video.id, Some(prompt.id), "provider failed")
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let failed = service
        .mark_failed_and_update_prompt(
            video.id,
            Some(prompt.id),
            " provider failed with complete diagnostic context ",
        )
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let err = service
        .mark_completed_and_link_prompt(
            video.id,
            "projects/1/videos/chapter_1/video.mp4",
            None,
            1024,
            None,
            Some(prompt.id),
        )
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = service
        .mark_completed_and_link_prompt(
            video.id,
            "projects/1/videos/chapter_1/video.mp4",
            Some("projects/1/videos/chapter_1/thumb.jpg"),
            2048,
            Some(json!({"job": "done"})),
            Some(prompt.id),
        )
        .await
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
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(video_project.id);
    input.chapter_id = Some(root_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let updated = service
        .mark_completed_and_link_prompt(
            video.id,
            "projects/2/videos/chapter_1/imported.mp4",
            None,
            4096,
            Some(json!({"source": "manual_import"})),
            Some(prompt.id),
        )
        .await
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

#[tokio::test]
async fn update_prompt_status_for_video_rejects_prompt_from_another_chapter_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let video_chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let other_chapter = insert_video_chapter(db.as_ref(), project.id, 2).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, other_chapter.id, None).await;
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.chapter_id = Some(video_chapter.id);
    let video = service.create_video_record(input).await.unwrap();

    let err = service
        .update_prompt_status_for_video(video.id, prompt.id, "running")
        .await
        .expect_err("metadata prompt id must not update a prompt from another chapter");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message == format!("视频 #{} 不属于提示词章节 {}", video.id, other_chapter.id)
    ));
    let stored_prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_prompt.status, "completed");
    assert_eq!(stored_prompt.generated_video_id, None);
    let stored_video = generated_video::Entity::find_by_id(video.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_video.status, "running");
}

#[tokio::test]
async fn begin_prompt_video_generation_rejects_orphan_prompt_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, 999, Some(777)).await;
    let service = VideoService::new(db.clone());

    let err = service
        .begin_prompt_video_generation(prompt.id)
        .await
        .expect_err("orphan prompt must not be locked as running");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "章节不存在: 999"
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, "completed");
    assert_eq!(stored.generated_video_id, Some(777));
}

#[tokio::test]
async fn begin_prompt_video_generation_rejects_generated_video_linked_prompt_without_update() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let prompt = insert_video_prompt(db.as_ref(), project.id, chapter.id, Some(777)).await;
    let service = VideoService::new(db.clone());

    let err = service
        .begin_prompt_video_generation(prompt.id)
        .await
        .expect_err("generated video must lock prompt video generation");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("已存在由该视频提示词生成的视频")
                && message.contains("请先删除对应视频")
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, "completed");
    assert_eq!(stored.generated_video_id, Some(777));
}

#[tokio::test]
async fn delete_missing_video_returns_not_found_without_touching_prompt_reference() {
    let db = Arc::new(setup_db().await);
    let prompt = insert_video_prompt(db.as_ref(), 1, 1, Some(999)).await;
    let service = VideoService::new(db.clone());

    let err = service
        .delete(999)
        .await
        .expect_err("missing video deletion must be visible");

    assert!(matches!(err, AppError::NotFound(message) if message == "视频不存在"));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_video_id, Some(999));
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn delete_missing_video_references_clears_stale_prompt_lock() {
    if std::env::var("A4N_TEST_DATABASE_URL").is_err() {
        eprintln!(
            "skipping missing video reference cleanup DB test: A4N_TEST_DATABASE_URL is not set"
        );
        return;
    }
    let db = Arc::new(setup_db().await);
    let prompt = insert_video_prompt(db.as_ref(), 1, 1, Some(999)).await;
    let service = VideoService::new(db.clone());

    let project_ids = service
        .missing_video_reference_project_ids(999)
        .await
        .unwrap();
    assert_eq!(project_ids, vec![1]);

    let cleared = service.delete_missing_video_references(999).await.unwrap();
    assert_eq!(cleared, 1);
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_video_id, None);
    assert_eq!(stored.status, "pending");
}

#[tokio::test]
async fn delete_video_marks_canvas_nodes_missing_without_deleting_them() {
    if std::env::var("A4N_TEST_DATABASE_URL").is_err() {
        eprintln!("skipping video canvas deletion DB test: A4N_TEST_DATABASE_URL is not set");
        return;
    }
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let service = VideoService::new(db.clone());
    let mut input = video_record_input(project.id);
    input.video_type = "canvas_video".to_string();
    input.workflow_mode = Some("canvas_video".to_string());
    let video = service.create_video_record(input).await.unwrap();
    let now = fixed_time(3);
    let graph = canvas_graph::ActiveModel {
        project_id: Set(project.id),
        name: Set("视频画布".to_string()),
        graph_kind: Set("video_studio".to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let node = canvas_node::ActiveModel {
        graph_id: Set(graph.id),
        node_kind: Set("VideoNode".to_string()),
        ref_type: Set(Some("legacy_generated_video".to_string())),
        ref_id: Set(Some(video.id)),
        config_json: Set(json!({
            "generated_video_id": video.id,
            "title": "可删除视频引用"
        })),
        position_json: Set(json!({"x": 0, "y": 0})),
        state: Set("ready".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();

    service.delete(video.id).await.unwrap();

    let stored_node = canvas_node::Entity::find_by_id(node.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("canvas node should be preserved");
    assert_eq!(stored_node.state, "missing");
    assert_eq!(stored_node.ref_type, None);
    assert_eq!(stored_node.ref_id, None);
    assert_eq!(
        stored_node
            .config_json
            .get("missing_ref_type")
            .and_then(Value::as_str),
        Some("generated_video")
    );
    assert_eq!(
        stored_node
            .config_json
            .get("missing_ref_id")
            .and_then(Value::as_i64),
        Some(video.id)
    );
    assert!(stored_node.config_json.get("generated_video_id").is_none());
}
