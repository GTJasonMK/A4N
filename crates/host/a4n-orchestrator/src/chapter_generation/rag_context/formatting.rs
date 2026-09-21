mod context;
mod result;

pub(in crate::chapter_generation::rag_context) use context::format_generation_rag_context;
#[cfg(test)]
pub(in crate::chapter_generation::rag_context) use context::{
    GENERATION_RAG_CONTEXT_CHAR_LIMIT, GENERATION_RAG_MAX_PASSAGE_RESULTS,
};
#[cfg(test)]
pub(in crate::chapter_generation::rag_context) use result::format_generation_rag_result;
