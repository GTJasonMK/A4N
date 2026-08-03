//! 角色发现工作流指纹与 business key 纯规则（无 DB）。

use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::discovery_workflow::{
    discovery_business_key, discovery_call_fingerprint_json, discovery_workflow_run_input,
};
use crate::workflow::UpsertWorkflowRunInput;

const CHARACTER_DISCOVERY_WORKFLOW_KIND: &str = "character_discovery";

/// 角色发现 LLM 调用键。
pub(in crate::character) const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";

/// 角色发现 exclusive workflow business key。
pub(in crate::character) fn character_discovery_business_key(
    project_id: i64,
    chapter_number: Option<i32>,
) -> String {
    discovery_business_key(
        CHARACTER_DISCOVERY_WORKFLOW_KIND,
        project_id,
        chapter_number,
    )
}

/// 角色发现单次 LLM 调用指纹。
pub(in crate::character) fn character_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    discovery_call_fingerprint_json(
        CHARACTER_DISCOVERY_WORKFLOW_KIND,
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
        CHARACTER_DISCOVERY_WORKFLOW_KIND,
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
    use a4n_llm::provider::ChatMessage;

    #[test]
    fn business_key_distinguishes_chapter_and_all() {
        assert_eq!(
            character_discovery_business_key(3, Some(2)),
            "character_discovery:3:chapter:2"
        );
        assert_eq!(
            character_discovery_business_key(3, None),
            "character_discovery:3:all"
        );
    }

    #[test]
    fn workflow_run_input_rejects_non_positive_project() {
        let request = ChatRequest::new(vec![ChatMessage::user("hi")], None, None, None);
        assert!(character_discovery_workflow_run_input(0, None, &[], &[], &request, None).is_err());
    }
}
