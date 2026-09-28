use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const AGENT_DEFAULT_MAX_STEPS: u32 = 32;
pub const AGENT_DEFAULT_MAX_TOOLS: u32 = 64;

pub fn agent_model_token_reservation(request: &serde_json::Value) -> u64 {
    serde_json::to_vec(request)
        .expect("JSON Value serializes")
        .len() as u64
        + request
            .get("max_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
        + 1024
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AgentBudget {
    pub max_duration_seconds: Option<u64>,
    pub max_steps: Option<u32>,
    pub max_model_calls: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub max_tokens: Option<u64>,
    pub max_cost_micros: Option<u64>,
    pub max_parallel_tools: Option<u16>,
    pub max_delegation_depth: Option<u8>,
    pub max_children: Option<u16>,
}

impl AgentBudget {
    pub fn effective_for_run(&self, root: bool) -> Self {
        let mut effective = self.clone();
        if root {
            effective.max_steps = Some(self.max_steps.unwrap_or(AGENT_DEFAULT_MAX_STEPS));
            effective.max_model_calls =
                Some(self.max_model_calls.unwrap_or(AGENT_DEFAULT_MAX_STEPS));
            effective.max_tool_calls = Some(self.max_tool_calls.unwrap_or(AGENT_DEFAULT_MAX_TOOLS));
        }
        effective
    }

    pub fn validate(&self) -> Result<(), String> {
        if [
            self.max_duration_seconds,
            self.max_tokens,
            self.max_cost_micros,
        ]
        .into_iter()
        .flatten()
        .any(|value| value == 0)
            || [self.max_steps, self.max_model_calls, self.max_tool_calls]
                .into_iter()
                .flatten()
                .any(|value| value == 0)
            || [self.max_parallel_tools, self.max_children]
                .into_iter()
                .flatten()
                .any(|value| value == 0)
            || self.max_delegation_depth == Some(0)
        {
            return Err("Agent budget 上限必须为正数".into());
        }
        if self.max_duration_seconds.is_some_and(|value| {
            i64::try_from(value)
                .ok()
                .and_then(chrono::Duration::try_seconds)
                .and_then(|duration| chrono::Utc::now().checked_add_signed(duration))
                .is_none()
                || std::time::Instant::now()
                    .checked_add(std::time::Duration::from_secs(value))
                    .is_none()
        }) {
            return Err("Agent max_duration_seconds 超出时间实现上限".into());
        }
        if self.max_cost_micros.is_some() {
            return Err(
                "当前 provider 没有可靠的 Agent cost usage，暂不支持 max_cost_micros".into(),
            );
        }
        Ok(())
    }

    pub fn extended(&self, extension: &Self, root: bool) -> Result<Self, String> {
        self.validate()?;
        extension.validate()?;
        if extension == &Self::default() {
            return Err("请至少提供一个新的 Agent 预算上限".into());
        }
        let mut next = self.effective_for_run(root);
        extend_budget_limit(
            &mut next.max_duration_seconds,
            extension.max_duration_seconds,
            None,
            "max_duration_seconds",
        )?;
        extend_budget_limit(&mut next.max_steps, extension.max_steps, None, "max_steps")?;
        extend_budget_limit(
            &mut next.max_model_calls,
            extension.max_model_calls,
            None,
            "max_model_calls",
        )?;
        extend_budget_limit(
            &mut next.max_tool_calls,
            extension.max_tool_calls,
            None,
            "max_tool_calls",
        )?;
        extend_budget_limit(
            &mut next.max_tokens,
            extension.max_tokens,
            None,
            "max_tokens",
        )?;
        extend_budget_limit(
            &mut next.max_parallel_tools,
            extension.max_parallel_tools,
            Some(1),
            "max_parallel_tools",
        )?;
        extend_budget_limit(
            &mut next.max_delegation_depth,
            extension.max_delegation_depth,
            None,
            "max_delegation_depth",
        )?;
        extend_budget_limit(
            &mut next.max_children,
            extension.max_children,
            None,
            "max_children",
        )?;
        Ok(next)
    }
}

fn extend_budget_limit<T: Copy + Ord>(
    current: &mut Option<T>,
    requested: Option<T>,
    default: Option<T>,
    field: &str,
) -> Result<(), String> {
    let Some(requested) = requested else {
        return Ok(());
    };
    if current.or(default).is_none_or(|limit| requested < limit) {
        return Err(format!("续做不能缩小 Agent {field} 上限"));
    }
    *current = Some(requested);
    Ok(())
}
