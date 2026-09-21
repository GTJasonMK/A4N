use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceOperationMetrics {
    pub requests_total: u64,
    pub errors_total: u64,
    pub cancelled_total: u64,
    pub in_flight: u64,
    pub duration_us_total: u64,
    pub duration_us_max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceRuntimeMetrics {
    pub service: String,
    pub pid: u32,
    pub started_at_unix_ms: u64,
    pub uptime_ms: u64,
    pub restart_count: u32,
    pub requests_total: u64,
    pub errors_total: u64,
    pub cancelled_total: u64,
    pub in_flight: u64,
    pub duration_us_total: u64,
    pub duration_us_max: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operations: BTreeMap<String, ServiceOperationMetrics>,
}
