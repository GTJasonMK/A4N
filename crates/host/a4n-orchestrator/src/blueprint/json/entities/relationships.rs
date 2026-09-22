use a4n_domain::project::ParsedBlueprintRelationship;
use a4n_error::AppError;
use serde_json::Value;

mod entry;
use entry::parse_relationship_entry;

pub(in crate::blueprint::json) fn parse_relationships(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintRelationship>, AppError> {
    parse_relationships_with_requirement(items, true)
}

pub(in crate::blueprint::json) fn parse_sparse_relationships(
    items: &[Value],
) -> Result<Vec<ParsedBlueprintRelationship>, AppError> {
    parse_relationships_with_requirement(items, false)
}

fn parse_relationships_with_requirement(
    items: &[Value],
    required: bool,
) -> Result<Vec<ParsedBlueprintRelationship>, AppError> {
    if required && items.is_empty() {
        return Err(AppError::InvalidArgument(
            "blueprint.relationships 不能为空".to_string(),
        ));
    }
    let relationships = items
        .iter()
        .enumerate()
        .map(parse_relationship_entry)
        .collect::<Result<Vec<_>, _>>()?;
    a4n_domain::project::blueprint_relationship_identity_keys(relationships.iter().map(
        |relationship| {
            (
                relationship.relationship_type.as_str(),
                relationship.character_a.as_str(),
                relationship.character_b.as_str(),
            )
        },
    ))
    .map_err(AppError::InvalidArgument)?;
    Ok(relationships)
}
