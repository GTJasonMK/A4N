use std::collections::HashMap;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_db::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING, upsert_workflow_run};
use a4n_test_support::llm::StaticLlmProvider;
use sea_orm::*;

use super::super::generation::ChapterGenerationStreamInput;
use super::support::{insert_project, setup_db, test_service, writing_prompt_engine};

#[tokio::test]
async fn failed_new_chapter_stream_generation_does_not_create_empty_chapter_row() {
    let db = Arc::new(setup_db().await);
    let service = test_service(db.clone());
    let project = insert_project(db.as_ref()).await;
    let prompt_engine = writing_prompt_engine();
    let llm = StaticLlmProvider::new(" \n\t ");
    let workflow_run = upsert_workflow_run(
        db.as_ref(),
        UpsertWorkflowRunInput {
            business_key: format!("chapter_generation:{}:{}", project.id, 1),
            workflow_kind: "chapter_generation".to_string(),
            project_id: Some(project.id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("stream".to_string()),
            input_fingerprint_json: None,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("chapter_generation:chapter:1".to_string()),
            active_task_id: Some("test-chapter-generation".to_string()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();

    let variables = HashMap::new();
    let err = service
        .generate_stream(ChapterGenerationStreamInput {
            project_id: project.id,
            chapter_number: 1,
            chapter_outline: "第 1 章大纲",
            context_before: None,
            llm: &llm,
            prompt_renderer: &prompt_engine,
            variables: &variables,
            workflow_run_id: workflow_run.id,
            cancel_token: None,
            on_chunk: |_delta, _index| {},
        })
        .await
        .expect_err("invalid LLM response must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project.id))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(chapters.is_empty());
}
