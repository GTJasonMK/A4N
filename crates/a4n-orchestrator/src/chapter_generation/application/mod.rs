use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use a4n_api_contract::{
    chapter::{
        ChapterDerivedCleanupHostPort, PreparedGenerationRagEmbeddingRequest,
        PreviewChapterPromptInput, StartChapterGenerationInput,
    },
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
};
use a4n_db::workflow;
use a4n_db_models::models::chapter::chapter_outline;
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{GenerationError, GenerationResultRef, StreamTextHash};
use futures::future::BoxFuture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    context::{GenerationTaskExecution, fail_generation_workflow_execution},
    generation::{
        checkpoint::GenerationCheckpointMetadata,
        events::GenerationEventPublisher,
        generation_finish_reason_text as shared_generation_finish_reason_text,
        generation_progress as chapter_progress, generation_public_error, generation_session_error,
        generation_terminal_error,
        stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
        terminal::CompletionDescriptor,
    },
    workflow::workflow_checkpoint_contract_app_error,
};

use super::service::{
    ChapterGenerationService, ChapterGenerationSession,
    generation::checkpoint::{
        ChapterGenerationCheckpointSaveInput, chapter_generation_call_fingerprint_json,
        save_completed_chapter_generation_checkpoint,
        validate_chapter_generation_source_fingerprint,
    },
};

use super::{
    ChapterGenerationApplicationContext, ChapterGenerationApplicationPort,
    ChapterGenerationRagPreparationApplicationPort, ChapterGenerationRequestPreview,
    ChapterGenerationRequestPreviewApplicationPort, ChapterGenerationWordCountRange,
    generation_chapter_title, generation_rag_query_fingerprint, merge_generation_outline_text,
    non_empty_trimmed, resolve_generation_word_count_range, upsert_chapter_generation_variables,
    validate_generation_rag_query_fingerprint,
};
use a4n_rag_core::validation::validate_query_vectors;

struct PreparedChapterGeneration {
    chapter_title: String,
    source_outline: Option<chapter_outline::Model>,
    outline_text: String,
    context_before: Option<String>,
    word_count_range: ChapterGenerationWordCountRange,
    use_rag: bool,
    variables: HashMap<String, Value>,
    source_fingerprint_json: Value,
    rag_embedding_identity: Option<EmbeddingModelIdentity>,
}

struct PreparedChapterGenerationSources {
    chapter_title: String,
    source_outline: Option<chapter_outline::Model>,
    writing_instruction: Option<String>,
    rag_query_outline_text: String,
    base_outline_text: String,
    reference_context: Option<String>,
    context_before: Option<String>,
    word_count_range: ChapterGenerationWordCountRange,
}

struct ResolvedChapterGenerationRagContext {
    text: Option<String>,
    embedding_identity: Option<EmbeddingModelIdentity>,
    query_fingerprint: Option<String>,
}

#[derive(Debug)]
struct ClientGenerationRagInput {
    embedding_identity: EmbeddingModelIdentity,
    query_fingerprint: String,
    query_vectors: Vec<Vec<f32>>,
}

struct PrepareChapterGenerationInput<'a> {
    project_id: i64,
    chapter_number: i32,
    writing_notes: Option<&'a str>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag: Option<ClientGenerationRagInput>,
    runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    cancel_token: Option<&'a CancellationToken>,
}

#[cfg(test)]
mod cancellation_tests {
    use super::{StartChapterGenerationInput, client_generation_rag_input};

    #[test]
    fn chapter_generation_start_errors_use_stable_codes_and_retry_policy() {
        let invalid = crate::GenerationTaskAccessError::InvalidArgument("请求无效".to_string())
            .into_start_error();
        assert_eq!(invalid.code(), "generation_request_invalid");
        assert!(!invalid.retryable);

        let forbidden =
            crate::GenerationTaskAccessError::Forbidden("无权限".to_string()).into_start_error();
        assert_eq!(forbidden.code(), "generation_task_forbidden");
        assert!(!forbidden.retryable);
    }

    fn chapter_generation_input(chapter_number: i32) -> StartChapterGenerationInput {
        StartChapterGenerationInput {
            meta: a4n_api_contract::generation_tasks::GenerationStartRequestMeta {
                start_request_id: "test-request".to_string(),
                retry_of_task_id: None,
            },
            project_id: 1,
            chapter_number,
            writing_notes: None,
            min_word_count: None,
            max_word_count: None,
            client_rag_embedding_identity: None,
            client_rag_query_fingerprint: None,
            client_rag_query_vectors: None,
        }
    }

    #[test]
    fn client_rag_fields_must_be_provided_together() {
        let mut input = chapter_generation_input(2);
        input.client_rag_query_fingerprint = Some("a".repeat(64));

        assert!(
            client_generation_rag_input(&input)
                .expect_err("partial client RAG payload must fail")
                .contains("必须同时提供")
        );
    }

    #[test]
    fn first_chapter_rejects_client_rag_vectors() {
        let mut input = chapter_generation_input(1);
        input.client_rag_embedding_identity =
            Some(a4n_api_contract::embedding::EmbeddingModelIdentity {
                backend: "local".to_string(),
                model: "test".to_string(),
                dimension: 2,
            });
        input.client_rag_query_fingerprint = Some("a".repeat(64));
        input.client_rag_query_vectors = Some(vec![vec![0.0, 1.0]]);

        assert!(
            client_generation_rag_input(&input)
                .expect_err("chapter one must not accept client RAG vectors")
                .contains("第 1 章")
        );
    }
}

include!("generation.rs");
include!("contracts.rs");
