//! 角色快照摘要差异。

use serde_json::{Value, json};

pub(super) fn snapshot_diff_summary(from: &Value, to: &Value) -> Value {
    json!({
        "from_chapter": from.get("synced_chapter"),
        "to_chapter": to.get("synced_chapter"),
        "from_state_summary": from.get("state_summary"),
        "to_state_summary": to.get("state_summary"),
        "from_changes_in_chapter": from.get("changes_in_chapter"),
        "to_changes_in_chapter": to.get("changes_in_chapter"),
        "from_behaviors_in_chapter": from.get("behaviors_in_chapter"),
        "to_behaviors_in_chapter": to.get("behaviors_in_chapter"),
    })
}
