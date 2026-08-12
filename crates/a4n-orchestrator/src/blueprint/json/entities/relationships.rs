use a4n_core::blueprint::ParsedBlueprintRelationship;
use a4n_core::error::AppError;
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
    items
        .iter()
        .enumerate()
        .map(parse_relationship_entry)
        .collect()
}
