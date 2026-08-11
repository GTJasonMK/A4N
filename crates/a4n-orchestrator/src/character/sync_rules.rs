//! 角色同步范围与跳过判定纯规则（无 DB）。

use a4n_api_contract::character::{CharacterProfileSkippedResult, CharacterProfileSyncSkipReason};
use a4n_core::models::chapter::chapter;
use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// 校验同步目标章节是否存在且可同步（调用方已加载章节列表）。
pub fn validate_character_sync_target_chapter(
    chapters: &[chapter::Model],
    project_id: i64,
    chapter_number: i32,
) -> Result<i32, String> {
    if chapter_number <= 0 {
        return Err("同步目标章节必须大于 0".to_string());
    }
    let latest_chapter = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .ok_or_else(|| format!("项目还没有可同步章节: project_id={project_id}"))?;
    if chapter_number > latest_chapter {
        return Err(format!(
            "同步目标章节不存在: 第 {chapter_number} 章，当前最新章节为第 {latest_chapter} 章"
        ));
    }
    if chapters
        .iter()
        .any(|chapter| chapter.chapter_number == chapter_number)
    {
        Ok(chapter_number)
    } else {
        Err(format!(
            "第 {chapter_number} 章没有可同步正文，请先生成或导入正文"
        ))
    }
}

/// 校验同步章节范围连续且输入章节完整。
pub fn validate_contiguous_character_sync_range(
    start_chapter: i32,
    target_chapter: i32,
    selected_chapter_numbers: &[i32],
) -> Result<(), String> {
    if start_chapter <= 0 || target_chapter < start_chapter {
        return Err(format!(
            "角色档案同步范围无效：第 {start_chapter} 章至第 {target_chapter} 章"
        ));
    }

    let available = selected_chapter_numbers
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let missing = (start_chapter..=target_chapter)
        .filter(|chapter| !available.contains(chapter))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let missing_text = missing
        .iter()
        .map(|chapter| format!("第 {chapter} 章"))
        .collect::<Vec<_>>()
        .join("、");
    Err(format!(
        "角色档案同步范围不完整：第 {start_chapter} 章至第 {target_chapter} 章中缺少可同步章节：{missing_text}"
    ))
}

/// 档案首次出现章节。
pub fn profile_first_seen_chapter(profile: &protagonist_profile::Model) -> Option<i32> {
    profile.first_seen_chapter()
}

/// 增量同步跳过原因。
pub fn profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<CharacterProfileSyncSkipReason> {
    if profile.last_synced_chapter == target_chapter {
        Some(CharacterProfileSyncSkipReason::AlreadySyncedToTarget)
    } else if profile.last_synced_chapter > target_chapter {
        Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
    } else if profile_first_seen_chapter(profile)
        .is_some_and(|first_seen| first_seen > target_chapter)
    {
        Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen)
    } else {
        None
    }
}

/// 分析后重同步跳过原因（仅 first_seen 门禁）。
pub fn analysis_profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<CharacterProfileSyncSkipReason> {
    if profile_first_seen_chapter(profile).is_some_and(|first_seen| first_seen > target_chapter) {
        Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen)
    } else {
        None
    }
}

/// 角色档案同步 workflow 输入指纹（纯规则，无 DB）。
pub fn character_profile_sync_workflow_fingerprint(
    project_id: i64,
    profile_id: i64,
    profile_name: &str,
    last_synced_chapter: i32,
    start_chapter: i32,
    target_chapter: i32,
    clear_to_chapter: i32,
    selected_chapter_numbers: &[i32],
    baseline_state: &Value,
    base_attributes: &Value,
    chapter_data: &Value,
) -> Value {
    json!({
        "project_id": project_id,
        "profile_id": profile_id,
        "profile_name": profile_name,
        "last_synced_chapter": last_synced_chapter,
        "start_chapter": start_chapter,
        "target_chapter": target_chapter,
        "clear_to_chapter": clear_to_chapter,
        "selected_chapter_numbers": selected_chapter_numbers,
        "baseline_state": baseline_state,
        "base_attributes": base_attributes,
        "chapter_data": chapter_data,
    })
}

/// 角色档案同步 workflow business_key。
pub fn character_profile_sync_business_key(profile_id: i64, target_chapter: i32) -> String {
    format!("character_profile_sync:{profile_id}:{target_chapter}")
}

/// 角色同步分阶段 LLM call_key：`character_sync:{profile_id}:{target_chapter}:{stage_key}`。
pub fn character_profile_sync_stage_call_key(
    profile_id: i64,
    target_chapter: i32,
    stage_key: &str,
) -> Result<String, String> {
    if profile_id <= 0 {
        return Err("角色档案 ID 必须大于 0".to_string());
    }
    if target_chapter <= 0 {
        return Err("同步目标章节必须大于 0".to_string());
    }
    let stage_key = stage_key.trim();
    if stage_key.is_empty() {
        return Err("角色同步 stage_key 不能为空".to_string());
    }
    Ok(format!(
        "character_sync:{profile_id}:{target_chapter}:{stage_key}"
    ))
}

/// 同步章节区间 mode 字符串。
pub fn character_profile_sync_mode_label(start_chapter: i32, target_chapter: i32) -> String {
    format!("{start_chapter}-{target_chapter}")
}

/// 带显式原因的跳过结果。
pub fn character_profile_skipped_result(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    reason: CharacterProfileSyncSkipReason,
) -> CharacterProfileSkippedResult {
    CharacterProfileSkippedResult {
        name: profile.name.clone(),
        synced_chapter: profile.last_synced_chapter,
        target_chapter,
        first_seen_chapter: profile_first_seen_chapter(profile),
        reason,
    }
}

/// 同步范围模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSyncRangeMode {
    Incremental,
    AnalysisResyncFromTarget,
}

/// 同步起止章节（含 clear_to）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterSyncRange {
    pub start_chapter: i32,
    pub clear_to_chapter: i32,
}

/// 解析同步范围；不可同步时返回 None。
pub fn character_sync_range(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    mode: CharacterSyncRangeMode,
) -> Option<CharacterSyncRange> {
    character_sync_range_with_first_seen(
        profile,
        target_chapter,
        mode,
        profile_first_seen_chapter(profile),
    )
}

pub fn character_sync_range_with_first_seen(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    mode: CharacterSyncRangeMode,
    first_seen_chapter: Option<i32>,
) -> Option<CharacterSyncRange> {
    let first_seen = first_seen_chapter
        .filter(|chapter| *chapter > 0)
        .unwrap_or(1);
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

/// 增量同步起始章。
pub fn character_sync_start_chapter(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<i32> {
    character_sync_range(profile, target_chapter, CharacterSyncRangeMode::Incremental)
        .map(|range| range.start_chapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(last_synced: i32, first_seen: i32) -> protagonist_profile::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "测试".into(),
            gender: None,
            age: None,
            base_attributes: json!({
                "explicit_attributes": {
                    "first_seen_chapter": first_seen
                }
            }),
            current_state: json!({}),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter: last_synced,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn character_profile_sync_stage_call_key_gates() {
        assert!(character_profile_sync_stage_call_key(0, 1, "a").is_err());
        assert!(character_profile_sync_stage_call_key(1, 0, "a").is_err());
        assert!(character_profile_sync_stage_call_key(1, 1, "  ").is_err());
        assert_eq!(
            character_profile_sync_stage_call_key(9, 3, "attribute_changes").unwrap(),
            "character_sync:9:3:attribute_changes"
        );
    }

    #[test]
    fn character_profile_sync_business_key_and_fingerprint() {
        assert_eq!(
            character_profile_sync_business_key(9, 3),
            "character_profile_sync:9:3"
        );
        assert_eq!(character_profile_sync_mode_label(1, 4), "1-4");
        let fp = character_profile_sync_workflow_fingerprint(
            1,
            2,
            "角色",
            0,
            1,
            2,
            2,
            &[1, 2],
            &json!({}),
            &json!({}),
            &json!([]),
        );
        assert_eq!(fp["profile_id"], 2);
        assert_eq!(fp["target_chapter"], 2);
        assert_eq!(fp["selected_chapter_numbers"], json!([1, 2]));
    }

    #[test]
    fn profile_sync_skip_when_already_at_or_past_target() {
        let p = profile(3, 1);
        assert_eq!(
            profile_sync_skip_reason(&p, 3),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedToTarget)
        );
        assert_eq!(
            profile_sync_skip_reason(&p, 2),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
        );
        assert_eq!(
            profile_sync_skip_reason(&p, 1),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
        );
    }

    #[test]
    fn incremental_range_starts_after_last_synced() {
        let p = profile(2, 1);
        let range =
            character_sync_range(&p, 5, CharacterSyncRangeMode::Incremental).expect("range");
        assert_eq!(range.start_chapter, 3);
        assert_eq!(range.clear_to_chapter, 5);
    }

    #[test]
    fn current_text_evidence_can_start_an_unsynced_formal_profile_at_the_target_chapter() {
        let mut p = profile(0, 1);
        p.base_attributes = json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        });

        let range = character_sync_range_with_first_seen(
            &p,
            7,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
            Some(7),
        )
        .expect("current text evidence should define the first synchronizable chapter");

        assert_eq!(range.start_chapter, 7);
        assert_eq!(range.clear_to_chapter, 7);
    }

    #[test]
    fn target_before_first_seen_skips() {
        let p = profile(0, 4);
        assert_eq!(
            profile_sync_skip_reason(&p, 2),
            Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen)
        );
        assert!(character_sync_range(&p, 2, CharacterSyncRangeMode::Incremental).is_none());
    }
}
