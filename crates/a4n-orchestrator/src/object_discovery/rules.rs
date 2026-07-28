//! 物体发现工作流指纹与章节范围纯规则（无 DB）。

use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 物体发现 LLM 调用键。
pub const OBJECT_DISCOVERY_CALL_KEY: &str = "object_discovery.llm";

/// 物体发现 exclusive workflow business key。
pub fn object_discovery_business_key(project_id: i64, chapter_number: Option<i32>) -> String {
    match chapter_number {
        Some(chapter_number) => format!("object_discovery:{project_id}:chapter:{chapter_number}"),
        None => format!("object_discovery:{project_id}:all"),
    }
}

/// 物体发现工作流输入指纹。
pub fn object_discovery_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": "object_discovery",
        "contract_version": 1,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data})).collect::<Vec<_>>(),
        "text_anchors": text_anchors.iter().map(|anchor| json!({
            "id": anchor.id,
            "source_chapter_id": anchor.source_chapter_id,
            "source_id": anchor.source_id,
            "quote_hash": anchor.quote_hash,
            "start_offset": anchor.start_offset,
            "end_offset": anchor.end_offset})).collect::<Vec<_>>(),
        "system_prompt_sha256": request.messages.first()
            .map(|message| source_content_sha256(&message.content)),
        "temperature": request.temperature,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format})
}

/// 物体发现单次 LLM 调用指纹。
pub fn object_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "object_discovery",
        "call_key": OBJECT_DISCOVERY_CALL_KEY,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?}))
}

/// 从可分析章节中选择本次物体发现范围，并在 IO 前后的 application 边界统一校验输入。
pub fn select_object_discovery_chapters(
    mut chapters: Vec<chapter::Model>,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if project_id <= 0 {
        return Err("物体档案发现 project_id 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("物体档案发现章节号必须大于 0".to_string());
    }
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
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("物体档案发现章节号必须大于 0".to_string());
    }
    let mode = chapter_number
        .map(|value| format!("chapter:{value}"))
        .unwrap_or_else(|| "all".to_string());
    Ok(UpsertWorkflowRunInput {
        business_key: object_discovery_business_key(project_id, chapter_number),
        workflow_kind: "object_discovery".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".to_string()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: Some(mode),
        input_fingerprint_json: Some(object_discovery_workflow_fingerprint(
            project_id,
            chapter_number,
            chapters,
            text_anchors,
            request,
        )),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(OBJECT_DISCOVERY_CALL_KEY.to_string()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    })
}

/// 校验物体档案 first/last_seen 是否落在本次已分析章节集合内。
///
/// `objects` 为 `(first_seen_chapter, last_seen_chapter)` 按 objects 数组下标顺序。
pub fn validate_object_discovery_chapter_scope(
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
    fn business_key_covers_chapter_and_all_scope() {
        assert_eq!(
            object_discovery_business_key(5, Some(1)),
            "object_discovery:5:chapter:1"
        );
        assert_eq!(
            object_discovery_business_key(5, None),
            "object_discovery:5:all"
        );
    }

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
        assert!(select_object_discovery_chapters(Vec::new(), 0, None).is_err());
        assert!(select_object_discovery_chapters(Vec::new(), 5, Some(0)).is_err());
    }
}
