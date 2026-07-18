mod api;
mod contracts;
mod preparation;

pub(in crate::actions::chapter_generation) use self::api::{
    build_generation_rag_context, build_generation_rag_context_with_embeddings,
    prepare_generation_rag_embedding_request,
};
pub(in crate::actions::chapter_generation) use self::contracts::PreparedGenerationRagEmbeddingRequest;
pub(in crate::actions::chapter_generation) use a4n_orchestrator::chapter_generation::GenerationRagContext;
