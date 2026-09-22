use a4n_domain::project::ParsedBlueprintCharacter;
use a4n_error::AppError;
use serde_json::Value;

mod entry;
use entry::parse_character_entry;

pub(in crate::blueprint::json) fn parse_characters(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintCharacter>, AppError> {
    parse_characters_with_minimum(items, 3)
}

pub(in crate::blueprint::json) fn parse_sparse_characters(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintCharacter>, AppError> {
    parse_characters_with_minimum(items, 0)
}

fn parse_characters_with_minimum(
    items: &[Value],
    minimum: usize,
) -> Result<Vec<ParsedBlueprintCharacter>, AppError> {
    if items.len() < minimum {
        return Err(AppError::InvalidArgument(
            "blueprint.characters 至少需要 3 个角色".to_string(),
        ));
    }
    let characters = items
        .iter()
        .enumerate()
        .map(parse_character_entry)
        .collect::<Result<Vec<_>, _>>()?;
    a4n_domain::project::blueprint_character_identity_keys(
        characters.iter().map(|character| character.name.as_str()),
    )
    .map_err(AppError::InvalidArgument)?;
    Ok(characters)
}
