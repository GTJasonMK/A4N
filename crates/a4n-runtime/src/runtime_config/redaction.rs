use a4n_service_protocol::redact_sensitive_value;
use serde_json::Value;

use super::access::current_snapshot;
use super::contracts::RuntimeConfigSnapshot;

pub fn redacted_current_snapshot_value() -> Option<Value> {
    current_snapshot().map(|snapshot| redacted_snapshot_value(&snapshot))
}

pub fn redacted_snapshot_value(snapshot: &RuntimeConfigSnapshot) -> Value {
    let mut value =
        serde_json::to_value(snapshot).expect("RuntimeConfigSnapshot 只包含 JSON 可序列化字段");
    redact_sensitive_value(&mut value);
    value
}
