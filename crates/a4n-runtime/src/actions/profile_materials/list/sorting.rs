use super::super::contracts::ProfileMaterialGroup;

pub(in crate::actions::profile_materials) fn apply_group_filters(
    groups: &mut Vec<ProfileMaterialGroup>,
    query: Option<&str>,
    filter: Option<&str>,
) {
    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        let query = query.to_lowercase();
        groups.retain(|group| group.profile.name.to_lowercase().contains(&query));
    }
    match filter.map(str::trim).filter(|value| !value.is_empty()) {
        Some("missing") => groups.retain(|group| group.material_state == "missing"),
        Some("ready") => groups.retain(|group| group.material_state == "ready"),
        Some("stale") => groups.retain(|group| group.material_state == "stale"),
        Some("unknown") => groups.retain(|group| group.material_state == "unknown"),
        Some("favorite") => groups.retain(|group| group.has_favorite_image),
        _ => {}
    }
}

pub(in crate::actions::profile_materials) fn apply_group_sort(
    groups: &mut [ProfileMaterialGroup],
    sort: Option<&str>,
) {
    match sort.map(str::trim).filter(|value| !value.is_empty()) {
        Some("stale_first") => groups.sort_by(|a, b| {
            state_rank(&a.material_state)
                .cmp(&state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        Some("newest") => groups.sort_by_key(|group| std::cmp::Reverse(newest_image_id(group))),
        Some("importance") => groups.sort_by(|a, b| {
            importance_rank(&a.profile.importance)
                .cmp(&importance_rank(&b.profile.importance))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        _ => groups.sort_by(|a, b| {
            state_rank(&a.material_state)
                .cmp(&state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
                .then_with(|| a.profile.id.cmp(&b.profile.id))
        }),
    }
}

fn newest_image_id(group: &ProfileMaterialGroup) -> Option<(chrono::NaiveDateTime, i64)> {
    group.newest_image_sort_key
}

fn state_rank(state: &str) -> i32 {
    match state {
        "missing" => 0,
        "stale" => 1,
        "unknown" => 2,
        _ => 3,
    }
}

fn importance_rank(value: &str) -> i32 {
    match value {
        "primary" | "主要" | "high" => 0,
        "secondary" | "次要" | "medium" => 1,
        _ => 2,
    }
}
