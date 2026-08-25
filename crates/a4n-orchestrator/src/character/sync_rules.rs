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
///
/// `first_seen_chapter` 必须与区间计算同源（`character_sync_range_with_first_seen`）：
/// 档案声明的 first_seen 与正文证据 first_seen 取 min。只读声明值会让跳过判定与区间
/// 计算对同一角色给出相反结论。
pub fn profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    first_seen_chapter: Option<i32>,
) -> Option<CharacterProfileSyncSkipReason> {
    if profile.last_synced_chapter == target_chapter {
        Some(CharacterProfileSyncSkipReason::AlreadySyncedToTarget)
    } else if profile.last_synced_chapter > target_chapter {
        Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
    } else if first_seen_chapter.is_some_and(|first_seen| first_seen > target_chapter) {
        Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen)
    } else {
        None
    }
}

/// 分析后重同步跳过原因（仅 first_seen 门禁）。`first_seen_chapter` 口径同
/// [`profile_sync_skip_reason`]。
pub fn analysis_profile_sync_skip_reason(
    target_chapter: i32,
    first_seen_chapter: Option<i32>,
) -> Option<CharacterProfileSyncSkipReason> {
    if first_seen_chapter.is_some_and(|first_seen| first_seen > target_chapter) {
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

/// 带显式原因的跳过结果。`first_seen_chapter` 口径同 [`profile_sync_skip_reason`]，
/// 直接展示给用户，必须与做出跳过决定时使用的值一致。
pub fn character_profile_skipped_result(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    reason: CharacterProfileSyncSkipReason,
    first_seen_chapter: Option<i32>,
) -> CharacterProfileSkippedResult {
    CharacterProfileSkippedResult {
        name: profile.name.clone(),
        synced_chapter: profile.last_synced_chapter,
        target_chapter,
        first_seen_chapter,
        reason,
    }
}

/// 同步范围模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSyncRangeMode {
    Incremental,
    AnalysisResyncFromTarget,
}

pub(crate) fn profile_sync_skip_reason_for_mode(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    mode: CharacterSyncRangeMode,
    first_seen_chapter: Option<i32>,
) -> Option<CharacterProfileSyncSkipReason> {
    match mode {
        CharacterSyncRangeMode::Incremental => {
            profile_sync_skip_reason(profile, target_chapter, first_seen_chapter)
        }
        CharacterSyncRangeMode::AnalysisResyncFromTarget => {
            analysis_profile_sync_skip_reason(target_chapter, first_seen_chapter)
        }
    }
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
            if profile_sync_skip_reason(profile, target_chapter, first_seen_chapter).is_some() {
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
        let first_seen = profile_first_seen_chapter(&p);
        assert_eq!(
            profile_sync_skip_reason_for_mode(
                &p,
                3,
                CharacterSyncRangeMode::Incremental,
                first_seen
            ),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedToTarget)
        );
        assert_eq!(
            profile_sync_skip_reason_for_mode(
                &p,
                2,
                CharacterSyncRangeMode::Incremental,
                first_seen
            ),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
        );
        assert_eq!(
            profile_sync_skip_reason_for_mode(
                &p,
                1,
                CharacterSyncRangeMode::Incremental,
                first_seen
            ),
            Some(CharacterProfileSyncSkipReason::AlreadySyncedPastTarget)
        );
        assert_eq!(
            profile_sync_skip_reason_for_mode(
                &p,
                2,
                CharacterSyncRangeMode::AnalysisResyncFromTarget,
                first_seen,
            ),
            None
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
            profile_sync_skip_reason(&p, 2, profile_first_seen_chapter(&p)),
            Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen)
        );
        assert!(character_sync_range(&p, 2, CharacterSyncRangeMode::Incremental).is_none());
    }
}

#[cfg(test)]
mod first_seen_source_consistency {
    use super::*;

    /// 手工创建的角色：base_attributes 里没有 explicit_attributes.first_seen_chapter。
    /// 角色发现匹配到已存在档案时只绑定正文证据锚点，不回写声明值
    /// （crates/a4n-orchestrator/src/character/discovery/persistence.rs:85-92），
    /// 因此「声明值缺失、证据值存在」是常规状态而非边缘情况。
    fn profile_without_declared_first_seen(last_synced: i32) -> protagonist_profile::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "林岚".into(),
            gender: None,
            age: None,
            base_attributes: json!({
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {}
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

    fn profile_with_declared_first_seen(first_seen: i32) -> protagonist_profile::Model {
        let mut profile = profile_without_declared_first_seen(0);
        profile.base_attributes = json!({
            "explicit_attributes": { "first_seen_chapter": first_seen },
            "implicit_attributes": {},
            "social_attributes": {}
        });
        profile
    }

    /// 只有正文证据 first_seen 时，跳过判定与区间计算必须得出同一结论：
    /// 都认定「目标章早于首次出现」，而不是一个说跳过、一个说可同步。
    #[test]
    fn evidence_only_first_seen_drives_both_skip_and_range() {
        let profile = profile_without_declared_first_seen(0);
        let target_chapter = 5;
        let effective_first_seen = Some(7);

        assert_eq!(profile_first_seen_chapter(&profile), None);

        for mode in [
            CharacterSyncRangeMode::Incremental,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
        ] {
            assert!(
                character_sync_range_with_first_seen(
                    &profile,
                    target_chapter,
                    mode,
                    effective_first_seen,
                )
                .is_none(),
                "{mode:?}: 区间计算应判定不可同步"
            );
            assert_eq!(
                profile_sync_skip_reason_for_mode(
                    &profile,
                    target_chapter,
                    mode,
                    effective_first_seen,
                ),
                Some(CharacterProfileSyncSkipReason::TargetBeforeFirstSeen),
                "{mode:?}: 跳过判定必须与区间计算一致，否则下游会拿空章节列表报\"缺少可同步章节\""
            );
        }
    }

    /// 证据 first_seen 早于声明值时，两者同样必须一致地认定「可以同步」。
    #[test]
    fn evidence_earlier_than_declared_unblocks_both_skip_and_range() {
        let profile = profile_with_declared_first_seen(10);
        let target_chapter = 5;
        let effective_first_seen = Some(3);

        assert_eq!(profile_first_seen_chapter(&profile), Some(10));

        let range = character_sync_range_with_first_seen(
            &profile,
            target_chapter,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
            effective_first_seen,
        );
        assert_eq!(
            range,
            Some(CharacterSyncRange {
                start_chapter: 3,
                clear_to_chapter: 5
            })
        );
        assert_eq!(
            profile_sync_skip_reason_for_mode(
                &profile,
                target_chapter,
                CharacterSyncRangeMode::AnalysisResyncFromTarget,
                effective_first_seen,
            ),
            None,
            "区间已认定可同步第 3-5 章，跳过判定不能因为声明值 10 而丢弃这次准备"
        );
    }

    /// 核心不变式：区间计算判定「不可同步」与跳过判定「有跳过原因」必须完全等价。
    ///
    /// 两者一旦分叉，就会出现「区间说不能同步 → 章节列表为空 → 跳过判定说可以同步 →
    /// 下游拿空列表报『缺少可同步章节』」这类与真实原因无关的报错，
    /// 或者反过来白做一整轮 prepare 再丢弃。
    #[test]
    fn range_is_none_exactly_when_a_skip_reason_exists() {
        let modes = [
            CharacterSyncRangeMode::Incremental,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
        ];
        let first_seen_values = [None, Some(-1), Some(0), Some(1), Some(3), Some(7), Some(20)];

        for last_synced in 0..=8 {
            for target_chapter in 1..=8 {
                for first_seen in first_seen_values {
                    let profile = profile_without_declared_first_seen(last_synced);
                    for mode in modes {
                        let range = character_sync_range_with_first_seen(
                            &profile,
                            target_chapter,
                            mode,
                            first_seen,
                        );
                        let skip = profile_sync_skip_reason_for_mode(
                            &profile,
                            target_chapter,
                            mode,
                            first_seen,
                        );
                        assert_eq!(
                            range.is_none(),
                            skip.is_some(),
                            "分叉: mode={mode:?} last_synced={last_synced} \
                             target={target_chapter} first_seen={first_seen:?} \
                             range={range:?} skip={skip:?}"
                        );
                    }
                }
            }
        }
    }

    /// 跳过结果里展示给用户的 first_seen_chapter 必须是做决定时用的那个值。
    #[test]
    fn skipped_result_reports_the_first_seen_used_for_the_decision() {
        let profile = profile_without_declared_first_seen(0);
        let summary = character_profile_skipped_result(
            &profile,
            5,
            CharacterProfileSyncSkipReason::TargetBeforeFirstSeen,
            Some(7),
        );
        assert_eq!(summary.first_seen_chapter, Some(7));
        assert_eq!(summary.target_chapter, 5);
    }
}
