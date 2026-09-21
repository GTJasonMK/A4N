use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_LOG_READ_LINES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetLogLevelInput {
    pub filter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReadLogsInput {
    #[schemars(range(min = 1, max = 10_000))]
    pub lines: usize,
    pub days_ago: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LogReadResult {
    pub file_path: String,
    pub total_lines: usize,
    pub returned_lines: usize,
    pub lines: Vec<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ReadLogsInput;

    #[test]
    fn read_logs_input_requires_explicit_range_and_day() {
        for input in [json!({ "lines": 500 }), json!({ "days_ago": 0 })] {
            serde_json::from_value::<ReadLogsInput>(input)
                .expect_err("log read inputs must not use hidden defaults");
        }
    }
}
