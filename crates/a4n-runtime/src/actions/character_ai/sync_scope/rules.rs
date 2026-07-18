use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};

pub(in crate::actions::character_ai) fn profile_first_seen_chapter(
    profile: &protagonist_profile::Model,
) -> Option<i32> {
    profile.first_seen_chapter()
}

pub(in crate::actions::character_ai) fn profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<&'static str> {
    if profile.last_synced_chapter == target_chapter {
        Some("already_synced_to_target")
    } else if profile.last_synced_chapter > target_chapter {
        Some("already_synced_past_target")
    } else if profile_first_seen_chapter(profile)
        .is_some_and(|first_seen| first_seen > target_chapter)
    {
        Some("target_before_first_seen")
    } else {
        None
    }
}

pub(in crate::actions::character_ai) fn analysis_profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<&'static str> {
    if profile_first_seen_chapter(profile).is_some_and(|first_seen| first_seen > target_chapter) {
        Some("target_before_first_seen")
    } else {
        None
    }
}

pub(in crate::actions::character_ai) fn sync_skip_summary(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Value {
    sync_skip_summary_with_reason(
        profile,
        target_chapter,
        profile_sync_skip_reason(profile, target_chapter).unwrap_or("not_skipped"),
    )
}

pub(in crate::actions::character_ai) fn sync_skip_summary_with_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    reason: &str,
) -> Value {
    json!({
        "name": profile.name,
        "synced_chapter": profile.last_synced_chapter,
        "target_chapter": target_chapter,
        "first_seen_chapter": profile_first_seen_chapter(profile),
        "reason": reason,
    })
}

#[cfg(test)]
pub(in crate::actions::character_ai) fn character_sync_start_chapter(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<i32> {
    character_sync_range(profile, target_chapter, CharacterSyncRangeMode::Incremental)
        .map(|range| range.start_chapter)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::character_ai) enum CharacterSyncRangeMode {
    Incremental,
    AnalysisResyncFromTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::character_ai) struct CharacterSyncRange {
    pub(in crate::actions::character_ai) start_chapter: i32,
    pub(in crate::actions::character_ai) clear_to_chapter: i32,
}

pub(in crate::actions::character_ai) fn character_sync_range(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    mode: CharacterSyncRangeMode,
) -> Option<CharacterSyncRange> {
    let first_seen = profile_first_seen_chapter(profile).unwrap_or(1);
    if first_seen > target_chapter {
        return None;
    }

    match mode {
        CharacterSyncRangeMode::Incremental => {
            if profile_sync_skip_reason(profile, target_chapter).is_some() {
                return None;
            }
            let next_unsynced = profile.last_synced_chapter.max(0) + 1;
            Some(CharacterSyncRange {
                start_chapter: next_unsynced.max(first_seen),
                clear_to_chapter: target_chapter,
            })
        }
        CharacterSyncRangeMode::AnalysisResyncFromTarget => {
            if profile.last_synced_chapter >= target_chapter {
                Some(CharacterSyncRange {
                    start_chapter: target_chapter,
                    clear_to_chapter: profile.last_synced_chapter,
                })
            } else {
                let next_unsynced = profile.last_synced_chapter.max(0) + 1;
                Some(CharacterSyncRange {
                    start_chapter: next_unsynced.max(first_seen),
                    clear_to_chapter: target_chapter,
                })
            }
        }
    }
}
