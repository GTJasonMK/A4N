use std::collections::HashSet;

use a4n_api_contract::character::CharacterProfileSyncFailure;
use a4n_core::{
    chapter_analysis::{
        foreshadowing_entries, priority_to_importance, validate_saved_chapter_analysis,
    },
    error::AppError,
    models::chapter::chapter,
};
use serde_json::Value;

use crate::{OrchestratorContext, project::ForeshadowingAnalysisDraft};

#[derive(Debug, thiserror::Error)]
pub enum ChapterAnalysisOutputError {
    #[error("章节缺少已保存结构化分析，不能同步派生数据: chapter_id={chapter_id}")]
    MissingSavedAnalysis { chapter_id: i64 },
    #[error("章节已保存的结构化分析数据无效，不能同步派生数据: chapter_id={chapter_id}：{source}")]
    InvalidSavedAnalysis {
        chapter_id: i64,
        #[source]
        source: AppError,
    },
    #[error(
        "同步章节伏笔失败：同一章节存在重复伏笔：chapter_id={chapter_id}, description={description}"
    )]
    DuplicateForeshadowing {
        chapter_id: i64,
        description: String,
    },
    #[error(
        "同步章节伏笔失败：优先级无效：chapter_id={chapter_id}, description={description}：{source}"
    )]
    InvalidForeshadowingPriority {
        chapter_id: i64,
        description: String,
        #[source]
        source: AppError,
    },
    #[error(
        "分析后角色档案同步未完全成功：失败 {failed} 个：{details}；已成功写入的角色会保留，重新同步会从未完成角色的断点继续"
    )]
    CharacterSyncIncomplete {
        failed: usize,
        details: String,
    },
    #[error(transparent)]
    Persistence(AppError),
    #[error("{0}")]
    Rag(String),
}

pub fn chapter_analysis_foreshadowing_drafts(
    chapter_id: i64,
    analysis: &Value,
) -> Result<Vec<ForeshadowingAnalysisDraft>, ChapterAnalysisOutputError> {
    validate_saved_chapter_analysis(analysis).map_err(|source| {
        ChapterAnalysisOutputError::InvalidSavedAnalysis { chapter_id, source }
    })?;

    let mut seen_descriptions = HashSet::new();
    foreshadowing_entries(analysis)
        .into_iter()
        .map(|entry| {
            let description = foreshadowing_description(
                &entry.description,
                entry.category.as_deref(),
                entry.original_text.as_deref(),
            );
            if !seen_descriptions.insert(description.clone()) {
                return Err(ChapterAnalysisOutputError::DuplicateForeshadowing {
                    chapter_id,
                    description,
                });
            }
            let importance =
                priority_to_importance(entry.priority.as_deref()).map_err(|source| {
                    ChapterAnalysisOutputError::InvalidForeshadowingPriority {
                        chapter_id,
                        description: description.clone(),
                        source,
                    }
                })?;
            Ok(ForeshadowingAnalysisDraft {
                description,
                status: entry.status,
                related_characters: entry.related_entities,
                importance,
            })
        })
        .collect()
}

pub(super) fn ensure_chapter_analysis_character_sync_completed(
    failed_items: &[CharacterProfileSyncFailure],
) -> Result<(), ChapterAnalysisOutputError> {
    if failed_items.is_empty() {
        return Ok(());
    }

    let details = failed_items
        .iter()
        .map(|item| format!("{}: {}", item.name, item.error))
        .take(5)
        .collect::<Vec<_>>()
        .join("；");
    Err(ChapterAnalysisOutputError::CharacterSyncIncomplete {
        failed: failed_items.len(),
        details,
    })
}

impl OrchestratorContext {
    pub async fn sync_chapter_analysis_foreshadowing_outputs(
        &self,
        chapter: &chapter::Model,
    ) -> Result<(), ChapterAnalysisOutputError> {
        let analysis = chapter.analysis_data.as_ref().ok_or(
            ChapterAnalysisOutputError::MissingSavedAnalysis {
                chapter_id: chapter.id,
            },
        )?;
        let drafts = chapter_analysis_foreshadowing_drafts(chapter.id, analysis)?;
        let synced = self
            .sync_chapter_analysis_foreshadowing(chapter.project_id, chapter.chapter_number, drafts)
            .await
            .map_err(ChapterAnalysisOutputError::Persistence)?;
        for item in &synced.current {
            self.sync_foreshadowing_rag(chapter.project_id, item)
                .await
                .map_err(ChapterAnalysisOutputError::Rag)?;
        }
        self.delete_foreshadowing_rag_by_ids(chapter.project_id, &synced.stale_ids)
            .await
            .map_err(|error| ChapterAnalysisOutputError::Rag(error.to_string()))?;
        Ok(())
    }
}

fn foreshadowing_description(
    description: &str,
    category: Option<&str>,
    original_text: Option<&str>,
) -> String {
    match (category, original_text) {
        (Some(category), Some(original_text)) if !original_text.trim().is_empty() => {
            format!("{description} [{category}] 原文: {original_text}")
        }
        (Some(category), _) => format!("{description} [{category}]"),
        (_, Some(original_text)) if !original_text.trim().is_empty() => {
            format!("{description} 原文: {original_text}")
        }
        _ => description.to_string(),
    }
}
