use std::collections::HashSet;

use a4n_api_contract::character::CharacterProfileSyncFailure;
use a4n_chapter_analysis::{
    foreshadowing_entries, priority_to_importance, summary_text, validate_saved_chapter_analysis,
};
use a4n_db_models::models::chapter::chapter;
use a4n_error::AppError;
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
    #[error("同步章节伏笔失败：伏笔 id 必须是正整数：chapter_id={chapter_id}, id={id}")]
    InvalidForeshadowingResolutionId { chapter_id: i64, id: String },
    #[error(
        "同步章节伏笔失败：同一章节重复更新伏笔：chapter_id={chapter_id}, foreshadowing_id={foreshadowing_id}"
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
                existing_id: entry
                    .id
                    .as_deref()
                    .map(|id| {
                        id.parse::<i64>().ok().filter(|id| *id > 0).ok_or_else(|| {
                            ChapterAnalysisOutputError::InvalidForeshadowingResolutionId {
                                chapter_id,
                                id: id.to_string(),
                            }
                        })
                    })
                    .transpose()?,
                evidence: entry.original_text.clone().ok_or_else(|| {
                    ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(
                        "伏笔缺少原文证据".into(),
                    ))
                })?,
                description,
                related_characters: entry.related_entities,
                importance,
            })
        })
        .collect()
}

fn chapter_analysis_foreshadowing_updates(
    chapter_id: i64,
    analysis: &Value,
    source_index: &a4n_domain::source_spans::SourceSpanIndex<'_>,
) -> Result<Vec<a4n_db::project::ForeshadowingFactUpdate>, ChapterAnalysisOutputError> {
    let mut seen = HashSet::new();
    let mut updates = Vec::new();
    for (key, resolved) in [("resolved", true), ("progressed", false)] {
        for item in analysis["foreshadowing"][key]
            .as_array()
            .into_iter()
            .flatten()
        {
            let raw_id = item["id"].as_str().unwrap_or_default().trim();
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
            let status = if resolved {
                a4n_domain::character::ForeshadowingStatus::Resolved
            } else {
                item["status"]
                    .as_str()
                    .and_then(a4n_domain::character::ForeshadowingStatus::parse)
                    .ok_or_else(|| {
                        ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(
                            "伏笔进展状态无效".into(),
                        ))
                    })?
            };
            let ids: Vec<String> = serde_json::from_value(item["source_span_ids"].clone())
                .map_err(|error| {
                    ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(format!(
                        "伏笔更新缺少原文证据，需要重新分析：{error}"
                    )))
                })?;
            a4n_domain::fact_feedback::validate_evidence_ids(&ids).map_err(|error| {
                ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(error))
            })?;
            let evidence = ids
                .iter()
                .map(|id| source_index.resolve_text(id))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(error))
                })?;
            let reason = item["resolution"]
                .as_str()
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .ok_or_else(|| {
                    ChapterAnalysisOutputError::Persistence(AppError::InvalidArgument(
                        "伏笔进展缺少变化说明".into(),
                    ))
                })?;
            updates.push(a4n_db::project::ForeshadowingFactUpdate {
                id: foreshadowing_id,
                status,
                evidence,
                reason: reason.to_string(),
            });
        }
    }
    Ok(updates)
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
        let source_spans =
            a4n_domain::source_spans::source_spans_for_chapters(std::slice::from_ref(chapter));
        let source_index = a4n_domain::source_spans::SourceSpanIndex::new(&source_spans);
        let updates = chapter_analysis_foreshadowing_updates(chapter.id, analysis, &source_index)?;
        let synced = self
            .sync_chapter_analysis_foreshadowing(
                a4n_db::project::ChapterSourceSnapshot::from(chapter),
                drafts,
                updates,
            )
            .await
            .map_err(ChapterAnalysisOutputError::Persistence)?;
        let current_ids = synced
            .current
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let rows = |items: &[a4n_db_models::models::character::foreshadowing_index::Model]| {
            items
                .iter()
                .map(|item| json!({ "id": item.id, "name": item.description }))
                .collect::<Vec<_>>()
        };
        Ok(json!({
            "current_ids": current_ids,
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
    fn updates_require_unique_positive_ids_reasons_and_current_evidence() {
        let spans = a4n_domain::source_spans::source_spans_for_content(3, 1, "脚步声来自守门人。");
        let index = a4n_domain::source_spans::SourceSpanIndex::new(&spans);
        let analysis = json!({
            "foreshadowing": {
                "resolved": [{"id": "7", "resolution": "回收一", "source_span_ids": ["ch1:s1:f1"]}],
                "progressed": [
                    {"id": "9", "status": "partially_revealed", "resolution": "部分揭示", "source_span_ids": ["ch1:s1:f1"]}
                ]
            }
        });
        let updates = chapter_analysis_foreshadowing_updates(3, &analysis, &index).unwrap();
        assert_eq!(
            updates.iter().map(|update| update.id).collect::<Vec<_>>(),
            [7, 9]
        );
        assert_eq!(updates[0].evidence, ["脚步声来自守门人"]);

        let mut duplicate = analysis.clone();
        duplicate["foreshadowing"]["progressed"][0]["id"] = json!("7");
        assert!(matches!(
            chapter_analysis_foreshadowing_updates(3, &duplicate, &index),
            Err(
                ChapterAnalysisOutputError::DuplicateForeshadowingResolution {
                    chapter_id: 3,
                    foreshadowing_id: 7
                }
            )
        ));

        let mut invalid = analysis.clone();
        invalid["foreshadowing"]["resolved"][0]["id"] = json!("0");
        assert!(matches!(
            chapter_analysis_foreshadowing_updates(3, &invalid, &index),
            Err(ChapterAnalysisOutputError::InvalidForeshadowingResolutionId { chapter_id: 3, .. })
        ));
        for (field, value) in [
            ("resolution", Value::Null),
            ("resolution", json!(" ")),
            ("source_span_ids", json!([])),
            ("source_span_ids", json!(["ch1:s1:f1", "ch1:s1:f1"])),
            ("source_span_ids", json!(["ch2:s1:f1"])),
        ] {
            let mut invalid = analysis.clone();
            invalid["foreshadowing"]["progressed"][0][field] = value;
            assert!(chapter_analysis_foreshadowing_updates(3, &invalid, &index).is_err());
        }
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
