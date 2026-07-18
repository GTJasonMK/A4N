use super::contracts::CharacterSyncPersistenceInput;
use crate::character::{CharacterDbError, validate_chapter_number};

pub(super) fn validate_character_sync_persistence_input(
    input: &CharacterSyncPersistenceInput,
) -> Result<(), CharacterDbError> {
    validate_chapter_number(input.start_chapter)?;
    validate_chapter_number(input.target_chapter)?;
    if input.start_chapter > input.target_chapter {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步起始章节不能大于目标章节".to_string(),
        ));
    }
    if let Some(clear_to_chapter) = input.clear_to_chapter {
        validate_chapter_number(clear_to_chapter)?;
        if input.start_chapter > clear_to_chapter {
            return Err(CharacterDbError::InvalidArgument(
                "角色同步清理起始章节不能大于清理结束章节".to_string(),
            ));
        }
    }
    if input.selected_chapter_numbers.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步缺少选中章节".to_string(),
        ));
    }
    if !input
        .selected_chapter_numbers
        .contains(&input.target_chapter)
    {
        return Err(CharacterDbError::InvalidArgument(format!(
            "角色同步选中章节缺少目标章节: {}",
            input.target_chapter
        )));
    }
    for chapter_number in &input.selected_chapter_numbers {
        validate_chapter_number(*chapter_number)?;
    }
    if input.generation_run.project_id != input.project_id {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步 generation run project_id 与项目不一致".to_string(),
        ));
    }
    if !input.generation_run.targets.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步 generation run 目标由 DB 层绑定，调用方不能预填 targets".to_string(),
        ));
    }
    if !input.implicit_review_decisions.is_array() {
        return Err(CharacterDbError::InvalidArgument(
            "角色同步隐性复审决策快照必须是 JSON 数组".to_string(),
        ));
    }
    Ok(())
}
