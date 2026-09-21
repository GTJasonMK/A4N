//! 物体发现工作流指纹与章节范围纯规则（无 DB）。

use a4n_db_models::models::{chapter::chapter, project::text_anchor};
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;
use std::collections::BTreeSet;

use crate::discovery_workflow::{discovery_call_fingerprint_json, discovery_workflow_run_input};
use crate::workflow::UpsertWorkflowRunInput;

use super::{OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_KIND};

/// 物体发现单次 LLM 调用指纹。
pub(super) fn object_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    discovery_call_fingerprint_json(
        OBJECT_DISCOVERY_KIND,
        OBJECT_DISCOVERY_CALL_KEY,
        project_id,
        chapter_number,
        request,
    )
}

/// 从可分析章节中选择本次物体发现范围。
pub(super) fn select_object_discovery_chapters(
    mut chapters: Vec<chapter::Model>,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if let Some(chapter_number) = chapter_number {
        chapters.retain(|chapter| chapter.chapter_number == chapter_number);
        if chapters.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于物体档案发现的有效正文"
            ));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于物体档案发现的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

/// 构造物体发现 workflow application 输入。
pub(super) fn object_discovery_workflow_run_input(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    discovery_workflow_run_input(
        OBJECT_DISCOVERY_KIND,
        OBJECT_DISCOVERY_CALL_KEY,
        project_id,
        chapter_number,
        chapters,
        text_anchors,
        request,
        active_task_id,
    )
}

/// 校验物体档案 first/last_seen 是否落在本次已分析章节集合内。
///
/// `objects` 为 `(first_seen_chapter, last_seen_chapter)` 按 objects 数组下标顺序。
pub(super) fn validate_object_discovery_chapter_scope(
    objects: &[(i32, i32)],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("物体档案发现缺少已分析章节".to_string());
    }
    for (index, (first_seen, last_seen)) in objects.iter().copied().enumerate() {
        if first_seen <= 0 || last_seen <= 0 {
            return Err(format!(
                "objects[{index}] first_seen_chapter/last_seen_chapter 必须大于 0"
            ));
        }
        if last_seen < first_seen {
            return Err(format!(
                "objects[{index}] last_seen_chapter 不能早于 first_seen_chapter"
            ));
        }
        if !analyzed_chapters.contains(&first_seen) {
            return Err(format!(
                "objects[{index}].first_seen_chapter 第 {first_seen} 章不在本次物体发现输入章节中"
            ));
        }
        if !analyzed_chapters.contains(&last_seen) {
            return Err(format!(
                "objects[{index}].last_seen_chapter 第 {last_seen} 章不在本次物体发现输入章节中"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_scope_requires_analyzed_and_bounds() {
        let analyzed = BTreeSet::from([1, 2, 3]);
        validate_object_discovery_chapter_scope(&[(1, 2)], &analyzed).unwrap();
        assert!(validate_object_discovery_chapter_scope(&[(1, 2)], &BTreeSet::new()).is_err());
        assert!(validate_object_discovery_chapter_scope(&[(0, 1)], &analyzed).is_err());
        assert!(validate_object_discovery_chapter_scope(&[(2, 1)], &analyzed).is_err());
        assert!(validate_object_discovery_chapter_scope(&[(1, 9)], &analyzed).is_err());
    }

    #[test]
    fn chapter_selection_validates_scope_and_content() {
        use super::super::{
            contracts::ObjectDiscoveryInput, execution::validate_object_discovery_input,
        };

        let now = chrono::Utc::now().naive_utc();
        let chapter = chapter::Model {
            id: 1,
            project_id: 5,
            outline_id: None,
            chapter_number: 2,
            title: None,
            content: "正文".to_string(),
            word_count: 2,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(
            select_object_discovery_chapters(vec![chapter.clone()], 5, Some(2))
                .unwrap()
                .len(),
            1
        );
        assert!(select_object_discovery_chapters(vec![chapter], 5, Some(3)).is_err());
        assert!(select_object_discovery_chapters(Vec::new(), 5, None).is_err());
        assert!(
            validate_object_discovery_input(&ObjectDiscoveryInput {
                project_id: 0,
                chapter_number: None,
            })
            .is_err()
        );
        assert!(
            validate_object_discovery_input(&ObjectDiscoveryInput {
                project_id: 5,
                chapter_number: Some(0),
            })
            .is_err()
        );
    }
}
