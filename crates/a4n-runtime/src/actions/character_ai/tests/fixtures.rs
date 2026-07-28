use super::prelude::*;
pub(super) fn discovery_chapter(
    chapter_number: i32,
    status: &str,
    content: &str,
) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: i64::from(chapter_number),
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn character_profile_for_test(name: &str) -> protagonist_profile::Model {
    let now = chrono::Utc::now().naive_utc();
    protagonist_profile::Model {
        id: 1,
        project_id: 1,
        name: name.to_string(),
        gender: None,
        age: None,
        base_attributes: json!({}),
        current_state: json!({
            "explicit_attributes": {
                "personality": "谨慎"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter: 0,
        created_at: now,
        updated_at: now,
    }
}
