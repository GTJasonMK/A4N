use serde_json::{Value, json};

use super::super::contracts::CharacterSyncStage;

pub(in crate::actions::character_ai) fn character_sync_stage_contract(
    stage: CharacterSyncStage,
) -> Value {
    match stage {
        CharacterSyncStage::EvidenceEvents => json!({
            "format": "json_object",
            "required_root_field": "events",
            "item_required_fields": ["chapter_number", "event_type", "summary", "evidence_anchor_id", "related_characters"],
            "rules": [
                "只抽取当前角色在本次章节中的事实事件、状态变化线索、关键行为线索和可能失效线索",
                "chapter_number 必须来自 input_chapter_numbers",
                "evidence_anchor_id 必须来自对应章节 chapters[].text_anchors[].id",
                "related_characters 没有时输出 []"
            ]
        }),
        CharacterSyncStage::AttributeChanges => json!({
            "format": "json_object",
            "required_root_field": "attribute_changes",
            "item_required_fields": ["chapter_number", "category", "key", "operation", "new_value", "change_description", "event_cause", "evidence_anchor_id"],
            "field_value_rules": {
                "category": {
                    "allowed_values": ["explicit", "implicit", "social"],
                    "forbidden_values": ["explicit_attributes", "implicit_attributes", "social_attributes"],
                    "mapping": {
                        "explicit": "baseline_state.explicit_attributes",
                        "implicit": "baseline_state.implicit_attributes",
                        "social": "baseline_state.social_attributes"
                    }
                },
                "operation": {
                    "allowed_values": ["add", "modify"],
                    "forbidden_values": ["delete"]
                }
            },
            "rules": [
                "只基于 previous_steps.character_sync_evidence_events 判断属性新增或修改",
                "不要输出 old_value、updated_attributes、current_state、delete 操作或最终状态快照",
                "category 必须只输出 explicit、implicit、social；不要输出 explicit_attributes、implicit_attributes、social_attributes",
                "key 必须是 category 对应属性对象下的顶层键，不能使用点路径",
                "没有属性变化时输出 []"
            ]
        }),
        CharacterSyncStage::ChapterStates => json!({
            "format": "json_object",
            "required_root_field": "chapter_states",
            "item_required_fields": ["chapter_number", "state_summary", "current_location", "current_goal", "emotional_state"],
            "rules": [
                "chapter_states 必须覆盖 input_chapter_numbers 中每个章节且不能重复",
                "只输出章节结束时该角色的稳定状态，不要输出属性变化或行为记录",
                "current_location、current_goal、emotional_state 不明确时输出 null"
            ]
        }),
        CharacterSyncStage::BehaviorRecords => json!({
            "format": "json_object",
            "required_root_field": "behaviors",
            "item_required_fields": ["chapter_number", "behavior_type", "behavior_description", "original_text_anchor_id", "behavior_tags", "related_characters", "impact_analysis"],
            "rules": [
                "只记录推动剧情、暴露性格、改变关系或影响目标的关键行为",
                "不要输出 classification_results 或 original_text",
                "impact_analysis 没有实质影响时输出空字符串"
            ]
        }),
        CharacterSyncStage::DeletionCandidates => json!({
            "format": "json_object",
            "required_root_field": "deletion_candidates",
            "item_required_fields": ["chapter_number", "category", "key", "reason", "evidence_anchor_id"],
            "field_value_rules": {
                "category": {
                    "allowed_values": ["explicit", "implicit", "social"],
                    "forbidden_values": ["explicit_attributes", "implicit_attributes", "social_attributes"],
                    "mapping": {
                        "explicit": "baseline_state.explicit_attributes",
                        "implicit": "baseline_state.implicit_attributes",
                        "social": "baseline_state.social_attributes"
                    }
                }
            },
            "rules": [
                "只记录已有属性被明确否定、失效或需要删除保护的候选",
                "不要输出 attribute_changes，也不要直接执行删除",
                "category 必须只输出 explicit、implicit、social；不要输出 explicit_attributes、implicit_attributes、social_attributes",
                "没有删除候选时输出 []"
            ]
        }),
    }
}
