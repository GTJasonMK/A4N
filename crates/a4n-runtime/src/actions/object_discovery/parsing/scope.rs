use crate::actions::object_discovery::types::ParsedObjectProfileDraft;
use std::collections::BTreeSet;

pub(super) fn validate_object_chapter_scope(
    drafts: &[ParsedObjectProfileDraft],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("物体档案发现缺少已分析章节".to_string());
    }
    for (index, draft) in drafts.iter().enumerate() {
        if draft.draft.first_seen_chapter <= 0 || draft.draft.last_seen_chapter <= 0 {
            return Err(format!(
                "objects[{index}] first_seen_chapter/last_seen_chapter 必须大于 0"
            ));
        }
        if draft.draft.last_seen_chapter < draft.draft.first_seen_chapter {
            return Err(format!(
                "objects[{index}] last_seen_chapter 不能早于 first_seen_chapter"
            ));
        }
        if !analyzed_chapters.contains(&draft.draft.first_seen_chapter) {
            return Err(format!(
                "objects[{index}].first_seen_chapter 第 {} 章不在本次物体发现输入章节中",
                draft.draft.first_seen_chapter
            ));
        }
        if !analyzed_chapters.contains(&draft.draft.last_seen_chapter) {
            return Err(format!(
                "objects[{index}].last_seen_chapter 第 {} 章不在本次物体发现输入章节中",
                draft.draft.last_seen_chapter
            ));
        }
    }
    Ok(())
}
