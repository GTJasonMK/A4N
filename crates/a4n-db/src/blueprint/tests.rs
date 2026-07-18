use serde_json::json;

use super::version::blueprint_version_snapshots;

#[test]
fn blueprint_version_snapshots_project_canonical_sections() {
    let snapshots = blueprint_version_snapshots(&json!({
        "world_setting": {"premise": "星潮改写记忆"},
        "characters": [{"name": "林澈"}],
        "relationships": [{"character_a": "林澈", "character_b": "顾遥"}],
        "full_synopsis": "主角追查星潮背后的阴谋。",
        "ignored": {"value": true}
    }));

    assert_eq!(
        snapshots.world_setting,
        Some(json!({"premise": "星潮改写记忆"}))
    );
    assert_eq!(snapshots.characters, Some(json!([{"name": "林澈"}])));
    assert_eq!(
        snapshots.relationships,
        Some(json!([{"character_a": "林澈", "character_b": "顾遥"}]))
    );
    assert_eq!(
        snapshots.synopsis.as_deref(),
        Some("主角追查星潮背后的阴谋。")
    );
}
