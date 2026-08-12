use std::collections::HashSet;

use a4n_api_contract::character::CharacterProfileSyncFailure;
use a4n_core::{
    chapter_analysis::{
        foreshadowing_entries, priority_to_importance, summary_text,
        validate_saved_chapter_analysis,
    },
    error::AppError,
    models::chapter::chapter,
};
use serde_json::{Value, json};

use crate::{OrchestratorContext, project::ForeshadowingAnalysisDraft};
use a4n_db::project::RetrospectiveChapterOutlineUpsertInput;

#[derive(Debug, thiserror::Error)]
pub enum ChapterAnalysisOutputError {
    #[error("章节缺少已保存结构化分析，不能同步派生数据: chapter_id={chapter_id}")]
    MissingSavedAnalysis { chapter_id: i64 },
    #[error("章节分析缺少可用摘要，不能生成回顾性章纲: chapter_id={chapter_id}")]
    MissingSummary { chapter_id: i64 },
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
    #[error("同步章节伏笔失败：回收伏笔 id 必须是正整数：chapter_id={chapter_id}, id={id}")]
    InvalidForeshadowingResolutionId { chapter_id: i64, id: String },
    #[error(
        "同步章节伏笔失败：同一章节重复回收伏笔：chapter_id={chapter_id}, foreshadowing_id={foreshadowing_id}"
    )]
    DuplicateForeshadowingResolution {
        chapter_id: i64,
        foreshadowing_id: i64,
    },
    #[error(
        "分析后角色档案同步未完全成功：失败 {failed} 个：{details}；已成功写入的角色会保留，重新同步会从未完成角色的断点继续"
    )]
    CharacterSyncIncomplete { failed: usize, details: String },
    #[error(transparent)]
    Persistence(AppError),
    #[error("{0}")]
    Rag(String),
}

pub fn retrospective_chapter_outline_input(
    chapter: &chapter::Model,
    source_fingerprint_json: Value,
) -> Result<RetrospectiveChapterOutlineUpsertInput, ChapterAnalysisOutputError> {
    let analysis =
        chapter
            .analysis_data
            .as_ref()
            .ok_or(ChapterAnalysisOutputError::MissingSavedAnalysis {
                chapter_id: chapter.id,
            })?;
    validate_saved_chapter_analysis(analysis).map_err(|source| {
        ChapterAnalysisOutputError::InvalidSavedAnalysis {
            chapter_id: chapter.id,
            source,
        }
    })?;
    let summary = summary_text(analysis).ok_or(ChapterAnalysisOutputError::MissingSummary {
        chapter_id: chapter.id,
    })?;

    Ok(RetrospectiveChapterOutlineUpsertInput {
        project_id: chapter.project_id,
        chapter_number: chapter.chapter_number,
        title: chapter
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("第{}章", chapter.chapter_number)),
        summary,
        key_scenes: non_empty_analysis_array(analysis, &["key_events"]),
        foreshadowing_setup: non_empty_analysis_array(analysis, &["foreshadowing", "planted"]),
        foreshadowing_payoff: non_empty_analysis_array(analysis, &["foreshadowing", "resolved"]),
        characters_involved: non_empty_analysis_array(analysis, &["metadata", "characters"]),
        word_count_target: chapter.word_count,
        source_fingerprint_json,
    })
}

fn non_empty_analysis_array(analysis: &Value, path: &[&str]) -> Option<Value> {
    path.iter()
        .try_fold(analysis, |current, key| current.get(*key))
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .map(|items| Value::Array(items.clone()))
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
                related_characters: entry.related_entities,
                importance,
            })
        })
        .collect()
}

fn chapter_analysis_foreshadowing_resolved_ids(
    chapter_id: i64,
    analysis: &Value,
) -> Result<Vec<i64>, ChapterAnalysisOutputError> {
    let mut seen = HashSet::new();
    analysis
        .get("foreshadowing")
        .and_then(|value| value.get("resolved"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            let raw_id = item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            let foreshadowing_id =
                raw_id
                    .parse::<i64>()
                    .ok()
                    .filter(|id| *id > 0)
                    .ok_or_else(|| {
                        ChapterAnalysisOutputError::InvalidForeshadowingResolutionId {
                            chapter_id,
                            id: raw_id.to_string(),
                        }
                    })?;
            if !seen.insert(foreshadowing_id) {
                return Err(
                    ChapterAnalysisOutputError::DuplicateForeshadowingResolution {
                        chapter_id,
                        foreshadowing_id,
                    },
                );
            }
            Ok(foreshadowing_id)
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
    ) -> Result<Value, ChapterAnalysisOutputError> {
        let analysis = chapter.analysis_data.as_ref().ok_or(
            ChapterAnalysisOutputError::MissingSavedAnalysis {
                chapter_id: chapter.id,
            },
        )?;
        let drafts = chapter_analysis_foreshadowing_drafts(chapter.id, analysis)?;
        let resolved_ids = chapter_analysis_foreshadowing_resolved_ids(chapter.id, analysis)?;
        let synced = self
            .sync_chapter_analysis_foreshadowing(
                a4n_db::project::ChapterSourceSnapshot::from(chapter),
                drafts,
                resolved_ids,
            )
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
        let rows = |items: &[a4n_core::models::character::foreshadowing_index::Model]| {
            items
                .iter()
                .map(|item| json!({ "id": item.id, "name": item.description }))
                .collect::<Vec<_>>()
        };
        Ok(json!({
            "created": rows(&synced.created),
            "updated": rows(&synced.updated),
            "existing": rows(&synced.existing),
            "resolved": rows(&synced.resolved),
        }))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn resolved_ids_are_positive_and_unique() {
        let analysis = json!({
            "foreshadowing": {
                "resolved": [
                    {"id": "7", "resolution": "回收一"},
                    {"id": "9", "resolution": "回收二"}
                ]
            }
        });
        assert_eq!(
            chapter_analysis_foreshadowing_resolved_ids(3, &analysis).unwrap(),
            [7, 9]
        );

        let duplicate = json!({
            "foreshadowing": {
                "resolved": [
                    {"id": "7", "resolution": "回收一"},
                    {"id": "7", "resolution": "回收二"}
                ]
            }
        });
        assert!(matches!(
            chapter_analysis_foreshadowing_resolved_ids(3, &duplicate),
            Err(
                ChapterAnalysisOutputError::DuplicateForeshadowingResolution {
                    chapter_id: 3,
                    foreshadowing_id: 7
                }
            )
        ));

        let invalid = json!({
            "foreshadowing": {"resolved": [{"id": "0", "resolution": "无效"}]}
        });
        assert!(matches!(
            chapter_analysis_foreshadowing_resolved_ids(3, &invalid),
            Err(ChapterAnalysisOutputError::InvalidForeshadowingResolutionId { chapter_id: 3, .. })
        ));
    }

    #[test]
    fn character_sync_incomplete_preserves_failed_profile_details() {
        ensure_chapter_analysis_character_sync_completed(&[])
            .expect("empty failure list should complete");

        let failed = [CharacterProfileSyncFailure {
            name: "林岚".to_string(),
            error: "模型响应缺少字段".to_string(),
        }];
        let error = ensure_chapter_analysis_character_sync_completed(&failed)
            .expect_err("failed profile sync must remain visible");

        assert!(matches!(
            &error,
            ChapterAnalysisOutputError::CharacterSyncIncomplete { failed: 1, .. }
        ));
        assert!(error.to_string().contains("失败 1 个"));
        assert!(error.to_string().contains("林岚: 模型响应缺少字段"));
    }
}
