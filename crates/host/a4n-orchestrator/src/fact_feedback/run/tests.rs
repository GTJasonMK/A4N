use super::*;
use a4n_api_contract::chapter::ChapterDerivedCleanupHostPort;
use a4n_db_models::models::chapter::chapter;
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatResponse};
use a4n_scene_host_contract::SceneProfileSyncHostPort;
use a4n_test_support::llm::CountingLlmProvider;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use std::{collections::HashMap, sync::Arc};

struct Template(bool);

#[async_trait]
impl PromptRenderer for Template {
    async fn render_prompt(
        &self,
        name: &str,
        _: &HashMap<String, Value>,
        _: Option<i64>,
    ) -> Result<String, AppError> {
        assert_eq!(name, FACT_CONSOLIDATION_PROMPT);
        if self.0 {
            Ok("归纳正文事实".into())
        } else {
            Err(AppError::ConfigError("测试归纳模板不可用".into()))
        }
    }
}

struct UnusedMaterialIndex;

#[async_trait]
impl ChapterDerivedCleanupHostPort for UnusedMaterialIndex {
    async fn finish_chapter_derived_cleanup(
        &self,
        _: i64,
        _: i64,
        _: i32,
        _: ChapterDerivedCleanupArtifacts,
    ) -> Result<(), AppError> {
        unreachable!()
    }
    async fn sync_chapter_rag_records(&self, _: i64, _: i64, _: i64) -> Result<(), AppError> {
        unreachable!()
    }
}

#[async_trait]
impl ChapterAnalysisHostPort for UnusedMaterialIndex {
    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort {
        unreachable!()
    }
    async fn sync_fact_material_rag_records(
        &self,
        _: i64,
        _: i64,
        blueprint: bool,
        characters: bool,
    ) -> Result<(), AppError> {
        assert!(!blueprint && !characters);
        Ok(())
    }
}

#[tokio::test]
async fn preparation_failure_remains_visible_and_resumes_after_repair() {
    let db = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = crate::test_support::TestOrchestratorBuilder::new(db.clone())
        .with_llm(Arc::new(CountingLlmProvider::new(ChatResponse {
            message: ChatMessage::assistant(r#"{"changes":[]}"#),
            usage: None,
            finish_reason: Some("stop".into()),
            reasoning_content: None,
        })))
        .build()
        .await;
    let project = a4n_db::project::create_project_record(
        db.as_ref(),
        a4n_db::project::CreateProjectRecordInput {
            user_id: 1,
            title: "归纳准备失败恢复".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        content: Set("本章明确正文。".into()),
        word_count: Set(7),
        status: Set("draft".into()),
        analysis_data: Set(Some(json!({"world_facts":[]}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let chapter_record =
        a4n_db::chapter_content::chapter_by_id_for_analysis(db.as_ref(), chapter.id)
            .await
            .unwrap();
    let source_run = context
        .begin_or_resume_workflow_run_validated(workflow_input(
            project.id,
            "chapter_feedback_fixture",
            json!({}),
            None,
        ))
        .await
        .unwrap();
    let tx = db.begin().await.unwrap();
    a4n_db::fact_feedback::record_feedback_completion(&tx, &chapter_record, source_run.id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let error = context
        .consolidate_facts(
            project.id,
            true,
            None,
            &Template(false),
            &UnusedMaterialIndex,
            Default::default(),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("模板不可用"));
    let failed = context
        .latest_workflow_run_by_business_key(&business_key(project.id, FACT_CONSOLIDATION_KIND))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert!(failed.can_resume);
    assert!(context.fact_feedback_warnings(project.id).await.unwrap()[0].contains("模板不可用"));
    context
        .consolidate_facts(
            project.id,
            true,
            Some(failed.id),
            &Template(true),
            &UnusedMaterialIndex,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        context
            .workflow_run_model_by_id(failed.id)
            .await
            .unwrap()
            .status,
        "completed"
    );
    assert!(
        context
            .fact_feedback_warnings(project.id)
            .await
            .unwrap()
            .is_empty()
    );
}
