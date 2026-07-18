mod client;
mod server;

pub(in crate::actions::chapter_generation) use client::{
    build_generation_rag_context_with_embeddings, prepare_generation_rag_embedding_request,
};
pub(in crate::actions::chapter_generation) use server::build_generation_rag_context;
