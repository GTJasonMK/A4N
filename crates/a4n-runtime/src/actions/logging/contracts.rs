use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SetLogLevelInput {
    /// 新的过滤表达式，例如 "debug" / "a4n=trace,info" / "warn"
    pub filter: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadLogsInput {
    /// 读取最后 N 行（默认 500）
    pub lines: Option<usize>,
    /// 读取相对于今天的偏移天数（0=今天，1=昨天）
    pub days_ago: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogReadResult {
    pub file_path: String,
    pub total_lines: usize,
    pub returned_lines: usize,
    pub lines: Vec<String>,
}
