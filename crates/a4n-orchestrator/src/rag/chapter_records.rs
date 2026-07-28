mod chunks;
mod contracts;
pub mod records;
mod sync;

pub use contracts::ChapterRagChunkingMode;
pub use records::chapter_rag_record_label;

#[cfg(test)]
mod tests;
