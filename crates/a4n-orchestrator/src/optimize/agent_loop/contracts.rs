use a4n_error::AppError;
use serde_json::Value;

pub(in crate::optimize) const AGENT_LOOP_RUNAWAY_GUARD_STEPS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::optimize) enum AgentLoopAction {
    Think,
    RunTool,
    Final,
}

impl AgentLoopAction {
    pub(in crate::optimize) fn as_str(&self) -> &'static str {
        match self {
            Self::Think => "think",
            Self::RunTool => "run_tool",
            Self::Final => "final",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::optimize) struct AgentLoopDecision {
    pub(in crate::optimize) thought: String,
    pub(in crate::optimize) public_update: Option<String>,
    pub(in crate::optimize) action: AgentLoopAction,
    pub(in crate::optimize) arguments: Value,
}

impl AgentLoopDecision {
    pub(in crate::optimize) fn public_update_for_non_final(&self) -> Result<&str, AppError> {
        self.public_update.as_deref().ok_or_else(|| {
            AppError::InvalidArgument("Agent Loop 非 final 决策缺少非空 public_update".to_string())
        })
    }
}

#[derive(Debug)]
pub(in crate::optimize) struct ThinkingObservation {
    pub(in crate::optimize) value: Value,
    pub(in crate::optimize) event_summary: String,
}
