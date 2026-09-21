//! 角色发现工作流指纹与 business key 纯规则（无 DB）。

use a4n_db_models::models::{chapter::chapter, project::text_anchor};
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use crate::discovery_workflow::{discovery_call_fingerprint_json, discovery_workflow_run_input};
use crate::workflow::UpsertWorkflowRunInput;

use super::discovery::{CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND};

/// 角色发现单次 LLM 调用指纹。
pub(in crate::character) fn character_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    discovery_call_fingerprint_json(
        CHARACTER_DISCOVERY_KIND,
        CHARACTER_DISCOVERY_CALL_KEY,
        project_id,
        chapter_number,
        request,
    )
}

/// 构造角色发现 begin 输入（status=RUNNING）。
pub(in crate::character) fn character_discovery_workflow_run_input(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
    active_task_id: Option<&str>,
) -> Result<UpsertWorkflowRunInput, String> {
    discovery_workflow_run_input(
        CHARACTER_DISCOVERY_KIND,
        CHARACTER_DISCOVERY_CALL_KEY,
        project_id,
        chapter_number,
        chapters,
        text_anchors,
        request,
        active_task_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_provider_contract::provider::ChatMessage;

    #[test]
    fn workflow_run_input_rejects_non_positive_project() {
        let request = ChatRequest::new(vec![ChatMessage::user("hi")], None, None);
        assert!(character_discovery_workflow_run_input(0, None, &[], &[], &request, None).is_err());
    }
}
