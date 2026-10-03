//! Database-to-domain adapter for portrait prompt rules.

use a4n_domain::character::CharacterProfileRecord;
use serde_json::Value;

fn portrait_profile(
    profile: &CharacterProfileRecord,
) -> a4n_domain::character::CharacterPortraitProfile {
    a4n_domain::character::CharacterPortraitProfile {
        name: profile.name.clone(),
        gender: profile.gender.clone(),
        age: profile.age.clone(),
        appearance: profile.appearance.clone(),
        background: profile.background.clone(),
        personality: profile.personality.clone(),
        base_attributes: profile.base_attributes.clone(),
        current_state: profile.current_state.clone(),
        character_arc: profile.character_arc.clone(),
        voice_fingerprint: profile.voice_fingerprint.clone(),
    }
}

pub fn profile_portrait_seed_prompt(profile: &CharacterProfileRecord) -> Result<String, String> {
    a4n_domain::character::profile_portrait_seed_prompt(&portrait_profile(profile))
}

pub fn portrait_profile_context(profile: &CharacterProfileRecord) -> Value {
    a4n_domain::character::portrait_profile_context(&portrait_profile(profile))
}

pub fn expression_or_default(expression: Option<&str>) -> String {
    a4n_domain::character::expression_or_default(expression)
}

pub fn portrait_style_with_pose(style: Option<&str>, pose: Option<&str>) -> Option<String> {
    a4n_domain::character::portrait_style_with_pose(style, pose)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile_with(name: &str) -> CharacterProfileRecord {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        CharacterProfileRecord {
            id: 1,
            project_id: 1,
            name: name.into(),
            gender: Some("女".into()),
            age: None,
            base_attributes: json!({"combat": "sword"}),
            current_state: json!({}),
            character_arc: None,
            voice_fingerprint: None,
            appearance: Some("黑发".into()),
            background: None,
            personality: None,
            last_synced_chapter: 0,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn expression_defaults_to_neutral_and_trims_explicit_value() {
        assert_eq!(expression_or_default(None), "neutral");
        assert_eq!(expression_or_default(Some("  ")), "neutral");
        assert_eq!(expression_or_default(Some("  smile ")), "smile");
    }

    #[test]
    fn style_with_pose_merges() {
        assert_eq!(
            portrait_style_with_pose(Some("anime"), Some("站立")).as_deref(),
            Some("anime\n姿势要求：站立")
        );
        assert_eq!(
            portrait_style_with_pose(None, Some("坐")).as_deref(),
            Some("姿势要求：坐")
        );
    }

    #[test]
    fn seed_prompt_includes_name_and_context() {
        let prompt = profile_portrait_seed_prompt(&profile_with("小雪")).expect("prompt");
        assert!(prompt.contains("小雪"));
        assert!(prompt.contains("黑发"));
        assert!(prompt.contains("base_attributes"));
    }
}
