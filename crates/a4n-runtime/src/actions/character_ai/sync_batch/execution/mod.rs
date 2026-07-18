mod analysis;
mod incremental;

pub(super) use analysis::sync_analysis_character_profile_models_to_chapter_batched;
pub(super) use incremental::sync_character_profile_models_to_chapter;

use super::super::sync_types::CHARACTER_PROFILE_SYNC_STEPS;

pub(super) fn character_sync_progress_total(item_count: usize) -> Result<u32, String> {
    let item_count =
        u32::try_from(item_count.max(1)).map_err(|_| "角色同步数量超出 u32 范围".to_string())?;
    item_count
        .checked_mul(CHARACTER_PROFILE_SYNC_STEPS)
        .ok_or_else(|| "角色同步进度总量超出 u32 范围".to_string())
}

pub(super) fn character_sync_progress_base(index: usize) -> Result<u32, String> {
    u32::try_from(index)
        .map_err(|_| "角色同步进度索引超出 u32 范围".to_string())?
        .checked_mul(CHARACTER_PROFILE_SYNC_STEPS)
        .ok_or_else(|| "角色同步进度索引超出 u32 范围".to_string())
}
