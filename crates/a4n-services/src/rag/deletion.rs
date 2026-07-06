use a4n_core::error::AppError;
use a4n_core::rag::RagDataType;
use a4n_engine::rag::vector_store::DeleteFilter;

use super::RagService;

impl RagService {
    /// 删除项目所有数据
    pub async fn clear_project(&self, project_id: i64) -> Result<(), AppError> {
        self.vector_store
            .drop_collection(&project_id.to_string())
            .await
            .map_err(|e| AppError::VectorStoreError(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_chapter_by_id(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
    ) -> Result<usize, AppError> {
        let chapter_id_prefixes = vec![format!("chapter:{project_id}:{chapter_id}:")];
        self.delete_chapter_records(project_id, chapter_number, chapter_id_prefixes)
            .await
    }

    async fn delete_chapter_records(
        &self,
        project_id: i64,
        chapter_number: i32,
        chapter_id_prefixes: Vec<String>,
    ) -> Result<usize, AppError> {
        if chapter_number <= 0 {
            return Ok(0);
        }

        if !chapter_id_prefixes.is_empty() {
            return self
                .vector_store
                .delete(
                    &project_id.to_string(),
                    DeleteFilter {
                        project_id,
                        data_type: None,
                        chapter_number: Some(chapter_number as u32),
                        ids: Vec::new(),
                        id_prefixes: chapter_id_prefixes,
                    },
                )
                .await
                .map_err(|e| AppError::VectorStoreError(e.to_string()));
        }

        let mut deleted = 0;
        for data_type in RagDataType::CHAPTER_DERIVED {
            deleted += self
                .vector_store
                .delete(
                    &project_id.to_string(),
                    DeleteFilter {
                        project_id,
                        data_type: Some(data_type.as_str().to_string()),
                        chapter_number: Some(chapter_number as u32),
                        ids: Vec::new(),
                        id_prefixes: Vec::new(),
                    },
                )
                .await
                .map_err(|e| AppError::VectorStoreError(e.to_string()))?;
        }
        Ok(deleted)
    }
}
