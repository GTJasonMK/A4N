mod contracts;
mod names;
mod portrait;
mod state;

pub use contracts::{
    AppliedImplicitUpdateAction, CharacterAttributeCategory,
    CharacterAttributeDeletionExecutionInput, CharacterAttributeDeletionRequestInput,
    CharacterAttributeOperation, CharacterAttributeValueChangeInput,
    CharacterImplicitReviewDecisionRecordInput, CharacterPortraitCreateInput,
    CharacterProfileCreateInput, CharacterSyncChapterStateRecordInput, CharacterTimelineSourceKind,
    DeletionMarkStatus, ForeshadowingRecordInput, ForeshadowingStatus, ImplicitUpdateDecision,
    InsertBehaviorRecordInput,
};
pub use names::*;
pub use portrait::*;
pub use state::*;

pub const CHARACTER_DELETION_THRESHOLD: i32 = 5;

#[cfg(test)]
mod portrait_tests {
    use super::*;
    use serde_json::json;

    fn profile() -> CharacterPortraitProfile {
        CharacterPortraitProfile {
            name: "陈恪".to_string(),
            gender: Some("男".to_string()),
            age: Some("二十余岁".to_string()),
            appearance: Some("清瘦".to_string()),
            background: Some("隋唐史博士".to_string()),
            personality: Some("冷静".to_string()),
            base_attributes: json!({"explicit_attributes":{"身份":"东宫书吏"}}),
            current_state: json!({"explicit_attributes":{"衣着":"青色官袍"}}),
            character_arc: Some(json!({"current":"试图改变玄武门结局"})),
            voice_fingerprint: None,
        }
    }

    #[test]
    fn portrait_prompt_keeps_explicit_profile_context() {
        let prompt = profile_portrait_seed_prompt(&profile()).expect("prompt should serialize");
        assert!(prompt.contains("角色名：陈恪"));
        assert!(prompt.contains("东宫书吏"));
        assert!(prompt.contains("青色官袍"));
        assert!(prompt.contains("至少包含服饰层级、发型、鞋履、材质色板四类视觉拆解"));
    }

    #[test]
    fn portrait_context_excludes_database_identity() {
        let context = portrait_profile_context(&profile());
        let text = serde_json::to_string(&context).expect("context should serialize");
        assert!(text.contains("玄武门"));
        assert!(!text.contains("project_id"));
        assert!(!text.contains("created_at"));
    }
}

#[cfg(test)]
mod character_name_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn character_name_projection_merges_analysis_and_discovery_aliases() {
        let analysis = json!({
            "metadata": {"characters": [" 林烬 "]},
            "character_states": {"苏晚": {}},
            "key_events": [{"involved_characters": ["林烬", "顾遥"]}]
        });
        let discovery = json!({
            "created": [{
                "name": "林烬",
                "aliases": ["阿烬"],
                "base_attributes": {"explicit_attributes": {"aliases": ["小烬"]}}
            }]
        });

        let analysis_names = character_names_from_chapter_analysis(&analysis);
        let discovery_names = character_names_from_discovery_result(&discovery);
        let names = analysis_sync_character_names(&analysis_names, &discovery_names);

        assert_eq!(
            names,
            ["林烬", "苏晚", "顾遥", "阿烬", "小烬"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        assert_eq!(
            character_keys_from_names(&names),
            ["林烬", "苏晚", "顾遥", "阿烬", "小烬"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{ForeshadowingStatus, apply_attribute_to_state};
    use serde_json::json;

    #[test]
    fn attribute_state_projection_applies_and_deletes_values() {
        let mut state = json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        });

        apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect("canonical state should accept attribute changes");
        assert_eq!(state["explicit_attributes"]["appearance"], json!("白衣"));

        apply_attribute_to_state(&mut state, "explicit", "appearance", "delete", json!(null))
            .expect("canonical state should accept attribute deletion");
        assert!(state["explicit_attributes"].get("appearance").is_none());
    }

    #[test]
    fn attribute_state_projection_rejects_non_object_state() {
        let mut state = json!("invalid");

        let error = apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect_err("non-object state must fail");

        assert_eq!(error, "角色当前状态不是 JSON 对象");
    }

    #[test]
    fn attribute_state_projection_rejects_non_object_category() {
        let mut state = json!({
            "explicit_attributes": [],
            "implicit_attributes": {},
            "social_attributes": {}
        });

        let error = apply_attribute_to_state(
            &mut state,
            "explicit",
            "appearance",
            "modify",
            json!("白衣"),
        )
        .expect_err("non-object category must fail");

        assert_eq!(error, "角色当前状态 explicit_attributes 不是 JSON 对象");
    }

    #[test]
    fn foreshadowing_status_accepts_only_canonical_values() {
        assert_eq!(
            ForeshadowingStatus::parse("partially_revealed"),
            Some(ForeshadowingStatus::PartiallyRevealed)
        );
        assert!(ForeshadowingStatus::Pending.can_resolve());
        assert!(!ForeshadowingStatus::Resolved.can_resolve());
        assert_eq!(ForeshadowingStatus::parse(" pending "), None);
        assert_eq!(ForeshadowingStatus::parse("legacy_pending"), None);
    }
}
