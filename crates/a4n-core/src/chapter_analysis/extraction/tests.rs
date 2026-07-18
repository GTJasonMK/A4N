use crate::error::AppError;
use serde_json::json;

use super::{
    analysis_rag_items, character_state_snapshots, foreshadowing_entries, priority_to_importance,
};

fn sample_analysis() -> serde_json::Value {
    json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": [],
            "tags": ["转折"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "追查线索",
                "changes": ["发现隐藏标记"],
                "emotional_state": "警觉"
            }
        },
        "foreshadowing": {
            "planted": [{
                "description": "敌人去向未明",
                "category": "人物",
                "priority": "high",
                "related_entities": ["林岚"]
            }],
            "resolved": [],
            "tensions": ["不会提升为待处理伏笔"]
        },
        "key_events": [{
            "description": "发现隐藏标记",
            "importance": "high"
        }]
    })
}

#[test]
fn projects_character_state_contract() {
    let mut analysis = sample_analysis();
    let snapshots = character_state_snapshots(&analysis);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].name, "林岚");
    assert_eq!(snapshots[0].state_summary, "追查线索；发现隐藏标记");
    assert_eq!(snapshots[0].current_location.as_deref(), Some("旧城"));

    analysis["character_states"]["林岚"]
        .as_object_mut()
        .unwrap()
        .remove("changes");
    assert!(character_state_snapshots(&analysis).is_empty());
}

#[test]
fn projects_only_planted_foreshadowing() {
    let entries = foreshadowing_entries(&sample_analysis());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].description, "敌人去向未明");
    assert_eq!(entries[0].status, "pending");
}

#[test]
fn maps_priority_strictly() {
    assert_eq!(priority_to_importance(Some("high")).unwrap(), 5);
    assert_eq!(priority_to_importance(Some("中")).unwrap(), 3);
    assert_eq!(priority_to_importance(Some("低")).unwrap(), 1);
    assert!(matches!(
        priority_to_importance(None),
        Err(AppError::InvalidArgument(message)) if message.contains("优先级缺失")
    ));
    assert!(matches!(
        priority_to_importance(Some("critical")),
        Err(AppError::InvalidArgument(message)) if message.contains("优先级非法")
    ));
}

#[test]
fn projects_rag_items_in_metadata_event_foreshadowing_order() {
    let items = analysis_rag_items(&sample_analysis());
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].id_suffix, "metadata");
    assert_eq!(items[1].id_suffix, "event1");
    assert_eq!(items[2].id_suffix, "foreshadow1");
    assert!(items[0].text.contains("角色: 林岚"));
    assert_eq!(items[1].text, "发现隐藏标记");
    assert_eq!(items[2].metadata["status"], "pending");
}
