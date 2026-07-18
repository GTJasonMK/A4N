const DEFAULT_CONTEXT_SAFETY_MARGIN_TOKENS: usize = 2048;
const DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT: u8 = 85;

#[derive(Debug, Clone)]
pub struct ContextBudgetPlanner {
    pub(super) safety_margin_tokens: usize,
    pub(super) input_utilization_percent: u8,
}

impl Default for ContextBudgetPlanner {
    fn default() -> Self {
        Self {
            safety_margin_tokens: DEFAULT_CONTEXT_SAFETY_MARGIN_TOKENS,
            input_utilization_percent: DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT,
        }
    }
}

impl ContextBudgetPlanner {
    pub fn new(safety_margin_tokens: usize) -> Self {
        Self {
            safety_margin_tokens,
            input_utilization_percent: DEFAULT_CONTEXT_INPUT_UTILIZATION_PERCENT,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_input_utilization_percent(mut self, input_utilization_percent: u8) -> Self {
        self.input_utilization_percent = input_utilization_percent;
        self
    }
}
