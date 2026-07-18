//! RAG 数据类型契约。

mod adapters;
mod catalog;
mod contracts;
mod rules;

pub use contracts::RagDataType;

#[cfg(test)]
mod tests;
