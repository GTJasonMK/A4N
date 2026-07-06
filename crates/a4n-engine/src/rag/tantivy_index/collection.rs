use std::path::PathBuf;

use a4n_core::error::AppError;
use tantivy::schema::{
    Field, INDEXED, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
};
use tantivy::tokenizer::NgramTokenizer;
use tantivy::{Index, IndexReader, ReloadPolicy};

use super::{CONTENT_NGRAM_TOKENIZER, FIELD_CHAPTER_NUMBER, FIELD_PROJECT_ID};

/// 单个 collection 的索引上下文。
pub(super) struct CollectionIndex {
    pub(super) index: Index,
    pub(super) reader: IndexReader,
    pub(super) field_id: Field,
    pub(super) field_content: Field,
    pub(super) field_search_text: Field,
    pub(super) field_data_type: Field,
    pub(super) field_chapter_number: Field,
    pub(super) field_project: Field,
}

impl CollectionIndex {
    fn schema_for() -> (Schema, Field, Field, Field, Field, Field, Field) {
        let mut builder = Schema::builder();
        let id = builder.add_text_field("id", STRING | STORED);
        let content = builder.add_text_field("content", STORED);
        let search_text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(CONTENT_NGRAM_TOKENIZER)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );
        let search_text = builder.add_text_field("search_text", search_text_options);
        let data_type = builder.add_text_field("data_type", STRING | STORED);
        let chapter_number = builder.add_i64_field(FIELD_CHAPTER_NUMBER, INDEXED | STORED);
        let project = builder.add_i64_field(FIELD_PROJECT_ID, INDEXED | STORED);
        (
            builder.build(),
            id,
            content,
            search_text,
            data_type,
            chapter_number,
            project,
        )
    }

    fn register_tokenizers(index: &Index) -> Result<(), AppError> {
        let tokenizer = NgramTokenizer::new(2, 3, false).map_err(|e| {
            AppError::VectorStoreError(format!("创建中文 ngram tokenizer 失败: {e}"))
        })?;
        index
            .tokenizers()
            .register(CONTENT_NGRAM_TOKENIZER, tokenizer);
        Ok(())
    }

    pub(super) fn open_or_create(dir: PathBuf) -> Result<Self, AppError> {
        let existed = dir.exists();
        if existed && !dir.is_dir() {
            return Err(AppError::VectorStoreError(format!(
                "tantivy 索引路径不是目录: {}",
                dir.display()
            )));
        }
        if !existed {
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::VectorStoreError(format!("创建 tantivy 索引目录失败: {e}"))
            })?;
        }

        let (
            schema,
            field_id,
            field_content,
            field_search_text,
            field_data_type,
            field_chapter_number,
            field_project,
        ) = Self::schema_for();

        let index = if existed {
            Index::open_in_dir(&dir)
                .map_err(|e| AppError::VectorStoreError(format!("打开 tantivy 索引失败: {e}")))?
        } else {
            Index::create_in_dir(&dir, schema)
                .map_err(|e| AppError::VectorStoreError(format!("创建 tantivy 索引失败: {e}")))?
        };
        Self::register_tokenizers(&index)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| AppError::VectorStoreError(format!("打开 tantivy reader 失败: {e}")))?;

        Ok(Self {
            index,
            reader,
            field_id,
            field_content,
            field_search_text,
            field_data_type,
            field_chapter_number,
            field_project,
        })
    }
}
