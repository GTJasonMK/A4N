use std::sync::Arc;

use a4n_api_contract::optimize::{
    InspectOptimizationContextInput, OptimizationContextSelection, ReadOptimizationContextInput,
};
use a4n_db::optimize::{OptimizationSessionCreateInput, create_optimization_session};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{ai::optimization_session, chapter::chapter};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;

use crate::optimize::permissions::default_agent_permissions;
use crate::optimize::runtime_tools::{inspect_project_context, read_project_context};
use crate::test_support::{TestOrchestratorBuilder, create_test_user};

#[tokio::test]
async fn agent_context_keeps_chapter_evidence_and_never_recovers_invalid_permissions_by_granting_reads()
 {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let user_id = create_test_user(&database).await;
    let project = create_project_record(
        database.as_ref(),
        CreateProjectRecordInput {
            user_id,
            title: "上下文权限边界".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        content: Set("\r\n第一段。\r\n第二段。\r\n\r\n第三段。".into()),
        word_count: Set(9),
        status: Set(chapter::STATUS_IMPORTED.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database.as_ref())
    .await
    .unwrap();
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    let index = || InspectOptimizationContextInput {
        source: None,
        cursor: None,
        expected_hash: None,
    };
    let initial = inspect_project_context(&context, user_id, project.id, chapter.id, index())
        .await
        .unwrap();
    assert!(
        initial["content"]["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source"] == "project_detail")
    );

    let page = read_project_context(
        &context,
        user_id,
        project.id,
        chapter.id,
        ReadOptimizationContextInput {
            source: "current_chapter".into(),
            selection: OptimizationContextSelection::Paragraphs { indexes: vec![2] },
            include: vec!["content".into(), "source_spans".into()],
            cursor: None,
            expected_hash: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(page["content"]["paragraphs"][0]["content"], "第二段。");
    assert_eq!(
        page["content"]["paragraphs"][0]["source_spans"][0]["text"],
        "第二段"
    );

    let mut permissions = default_agent_permissions();
    permissions.read_project_detail = false;
    permissions.read_blueprint = false;
    permissions.read_outlines = false;
    permissions.read_character_profiles = false;
    permissions.read_foreshadowing = false;
    let session_input = OptimizationSessionCreateInput {
        project_id: project.id,
        chapter_id: chapter.id,
        name: Some("受限会话".into()),
        status: "waiting_user".into(),
        issues: None,
        agent_permissions: Some(permissions.as_value()),
        agent_instruction: None,
        fixes_applied: 0,
        summary: None,
    };
    let restricted = create_optimization_session(database.as_ref(), session_input.clone())
        .await
        .unwrap();
    create_optimization_session(
        database.as_ref(),
        OptimizationSessionCreateInput {
            name: Some("已取消的全读会话".into()),
            status: "cancelled".into(),
            agent_permissions: Some(default_agent_permissions().as_value()),
            ..session_input
        },
    )
    .await
    .unwrap();
    let limited = inspect_project_context(&context, user_id, project.id, chapter.id, index())
        .await
        .unwrap();
    let sources = limited["content"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["source"], "current_chapter");

    let mut row: optimization_session::ActiveModel = restricted.into();
    row.agent_permissions = Set(Some(json!({"invalid": true})));
    let row = row.update(database.as_ref()).await.unwrap();
    let error = inspect_project_context(&context, user_id, project.id, chapter.id, index())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("invalid_agent_permissions"));
    let mut row: optimization_session::ActiveModel = row.into();
    row.agent_permissions = Set(None);
    row.update(database.as_ref()).await.unwrap();
    let error = inspect_project_context(&context, user_id, project.id, chapter.id, index())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("missing_agent_permissions"));
}
