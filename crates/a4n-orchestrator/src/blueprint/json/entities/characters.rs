use a4n_core::blueprint::ParsedBlueprintCharacter;
use a4n_core::error::AppError;
use serde_json::Value;

mod entry;
use entry::parse_character_entry;

pub(in crate::blueprint::json) fn parse_characters(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintCharacter>, AppError> {
    if items.len() < 3 {
        return Err(AppError::InvalidArgument(
            "blueprint.characters 至少需要 3 个角色".to_string(),
        ));
    }
    items
        .iter()
        .enumerate()
        .map(parse_character_entry)
        .collect()
}
