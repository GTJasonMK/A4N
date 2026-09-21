use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const REVERSE_ENGINEERING_PROGRESS_TOTAL: u32 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReverseEngineeringTarget {
    Blueprint,
    Outline,
    PartOutline,
}

impl ReverseEngineeringTarget {
    pub fn from_prompt_name(value: &str) -> Option<Self> {
        match value {
            "reverse_blueprint" => Some(Self::Blueprint),
            "reverse_outline" => Some(Self::Outline),
            "reverse_part_outline" => Some(Self::PartOutline),
            _ => None,
        }
    }

    pub const fn prompt_name(self) -> &'static str {
        match self {
            Self::Blueprint => "reverse_blueprint",
            Self::Outline => "reverse_outline",
            Self::PartOutline => "reverse_part_outline",
        }
    }

    pub const fn user_instruction(self) -> &'static str {
        match self {
            Self::Blueprint => "请从以下内容反推蓝图（世界观/角色/剧情大纲/主题，输出 JSON）：",
            Self::Outline => "请从以下章节内容反推章节大纲（输出 JSON）：",
            Self::PartOutline => "请从以下内容反推分部大纲结构（输出 JSON）：",
        }
    }

    pub const fn preview_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求预览",
            Self::Outline => "反推章节大纲 LLM 请求预览",
            Self::PartOutline => "反推分部大纲 LLM 请求预览",
        }
    }

    pub const fn preview_step_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求",
            Self::Outline => "反推章节大纲 LLM 请求",
            Self::PartOutline => "反推分部大纲 LLM 请求",
        }
    }

    pub const fn initial_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在准备蓝图归纳",
            Self::Outline => "正在准备章节大纲候选",
            Self::PartOutline => "正在准备分部大纲候选",
        }
    }

    pub const fn preparing_context_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在准备反推蓝图上下文",
            Self::Outline => "正在准备反推章节大纲上下文",
            Self::PartOutline => "正在准备反推分部大纲上下文",
        }
    }

    pub const fn calling_model_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在调用模型反推蓝图",
            Self::Outline => "正在调用模型反推章节大纲",
            Self::PartOutline => "正在调用模型反推分部大纲",
        }
    }

    pub const fn organizing_message(self) -> &'static str {
        match self {
            Self::Blueprint => "模型已返回，正在整理反推蓝图",
            Self::Outline => "模型已返回，正在整理章节大纲",
            Self::PartOutline => "模型已返回，正在整理分部大纲",
        }
    }

    pub const fn ready_message(self) -> &'static str {
        match self {
            Self::Blueprint => "蓝图已归纳并写入正式资料",
            Self::Outline => "章节大纲候选已生成，等待用户确认",
            Self::PartOutline => "分部大纲候选已生成，等待用户确认",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReverseEngineeringCandidateState {
    Proposal,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReverseEngineerInput {
    pub target: ReverseEngineeringTarget,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub content_sample: String,
    pub auto_context: bool,
    #[schemars(range(min = 1))]
    pub context_limit: Option<usize>,
    pub variables: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetReverseEngineeringCandidateInput {
    pub target: ReverseEngineeringTarget,
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfirmReverseEngineeringCandidateInput {
    #[schemars(range(min = 1))]
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReverseEngineeringCandidate {
    pub workflow_run_id: i64,
    pub target: ReverseEngineeringTarget,
    pub state: ReverseEngineeringCandidateState,
    pub warnings: Vec<String>,
    pub result: Value,
    pub applied: Option<Value>,
}

/// Host-facing read boundary for reverse-engineering candidates.
///
/// Candidate generation and confirmation state transitions remain with the
/// reverse-engineering owner.
#[async_trait::async_trait]
pub trait ImportReverseReadApplicationPort: Send + Sync {
    async fn latest(
        &self,
        project_id: i64,
        target: ReverseEngineeringTarget,
    ) -> Result<Option<ReverseEngineeringCandidate>, a4n_error::AppError>;
}

/// Host-facing confirmation boundary for reverse-engineering candidates.
///
/// Candidate generation remains with the reverse-engineering owner.
#[async_trait::async_trait]
pub trait ImportReverseConfirmationApplicationPort: Send + Sync {
    async fn confirm(
        &self,
        workflow_run_id: i64,
    ) -> Result<ReverseEngineeringCandidate, a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn target_owns_the_prompt_and_progress_contract() {
        assert_eq!(
            ReverseEngineeringTarget::from_prompt_name("reverse_part_outline"),
            Some(ReverseEngineeringTarget::PartOutline)
        );
        assert_eq!(
            ReverseEngineeringTarget::Blueprint.prompt_name(),
            "reverse_blueprint"
        );
        assert_eq!(REVERSE_ENGINEERING_PROGRESS_TOTAL, 3);
    }

    #[test]
    fn candidate_response_exposes_one_workflow_identity() {
        let value = serde_json::to_value(ReverseEngineeringCandidate {
            workflow_run_id: 9,
            target: ReverseEngineeringTarget::Outline,
            state: ReverseEngineeringCandidateState::Proposal,
            warnings: Vec::new(),
            result: json!({"outlines": []}),
            applied: None,
        })
        .unwrap();

        assert_eq!(value["workflow_run_id"], 9);
        assert_eq!(value["target"], "outline");
        assert!(value.get("candidate_id").is_none());
        assert!(value.get("origin").is_none());
        assert!(value["applied"].is_null());
    }
}
