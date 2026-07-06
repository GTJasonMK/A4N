use super::shared::{non_empty_trimmed, truncate_chars};
use crate::state::AppState;
use a4n_core::models::scene::scene_profile;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;

const SCENE_PROFILE_CONTEXT_LIMIT: usize = 8;

pub(super) async fn recent_scene_profile_context_lines(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<String>, String> {
    let mut profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::FirstSeenChapter.lt(chapter_number))
        .order_by_desc(scene_profile::Column::LastSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("构建场景档案上下文失败：查询场景档案失败: {err}"))?;
    profiles.sort_by_key(|profile| {
        (
            importance_rank(&profile.importance),
            std::cmp::Reverse(profile.last_seen_chapter),
            profile.id,
        )
    });
    Ok(profiles
        .iter()
        .filter_map(scene_profile_context_line)
        .take(SCENE_PROFILE_CONTEXT_LIMIT)
        .collect())
}

fn scene_profile_context_line(profile: &scene_profile::Model) -> Option<String> {
    let mut details = Vec::new();
    push_text_detail(&mut details, "视觉", &profile.visual_description, 100);
    push_text_detail(&mut details, "布局", &profile.spatial_layout, 100);
    push_text_detail(&mut details, "氛围", &profile.atmosphere, 80);
    if let Some(current_state) = compact_json_detail(&profile.current_state_json, 160) {
        details.push(format!("当前: {current_state}"));
    }
    if details.is_empty() {
        None
    } else {
        Some(format!(
            "- {}（第{}-{}章，{} / {}）：{}",
            profile.name,
            profile.first_seen_chapter,
            profile.last_seen_chapter,
            profile.importance,
            profile.scene_type,
            details.join("；")
        ))
    }
}

fn push_text_detail(details: &mut Vec<String>, label: &str, value: &str, limit: usize) {
    if let Some(text) = non_empty_trimmed(value) {
        details.push(format!("{label}: {}", truncate_chars(text, limit)));
    }
}

fn compact_json_detail(value: &Value, limit: usize) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Array(items) if items.is_empty() => None,
        Value::Object(map) if map.is_empty() => None,
        _ => serde_json::to_string(value)
            .ok()
            .and_then(|text| non_empty_trimmed(&text).map(|text| truncate_chars(text, limit))),
    }
}

fn importance_rank(value: &str) -> u8 {
    match value {
        "primary" => 0,
        "secondary" => 1,
        _ => 2,
    }
}
