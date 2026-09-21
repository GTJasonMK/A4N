use a4n_db::import_reverse::{ReverseBlueprintApplyDraft, ReverseBlueprintApplyResult};
use serde_json::{Value, json};

pub(super) use self::sections::validate_reverse_blueprint_sections;
pub(super) use self::top_level::{
    optional_needs_part_outlines, validate_reverse_blueprint_top_level_keys,
};
use super::super::validation::{optional_positive_i32, required_object_clone};
use super::entities::{reverse_blueprint_characters, reverse_blueprint_relationships};
use crate::import_reverse::application::reverse_json::required_string;

mod sections;
mod top_level;

pub fn reverse_blueprint_apply_draft(data: &Value) -> Result<ReverseBlueprintApplyDraft, String> {
    validate_reverse_blueprint_top_level_keys(data)?;
    let title = required_string(data, "title", "reverse_blueprint")?;
    let genre = required_string(data, "genre", "reverse_blueprint")?;
    let style = required_string(data, "style", "reverse_blueprint")?;
    let tone = required_string(data, "tone", "reverse_blueprint")?;
    let target_audience = required_string(data, "target_audience", "reverse_blueprint")?;
    let one_sentence_summary = required_string(data, "one_sentence_summary", "reverse_blueprint")?;
    let full_synopsis = required_string(data, "full_synopsis", "reverse_blueprint")?;
    let world_setting = required_object_clone(data, "world_setting", "reverse_blueprint")?;
    validate_reverse_blueprint_sections(data, &world_setting)?;
    let incoming_total_chapters =
        optional_positive_i32(data, "total_chapters", "reverse_blueprint")?;
    let incoming_needs_part_outlines = optional_needs_part_outlines(data)?;
    let characters = reverse_blueprint_characters(data)?;
    let relationships = reverse_blueprint_relationships(data)?;

    Ok(ReverseBlueprintApplyDraft {
        title,
        genre,
        style,
        tone,
        target_audience,
        one_sentence_summary,
        full_synopsis,
        world_setting,
        incoming_total_chapters,
        incoming_needs_part_outlines,
        characters,
        relationships,
    })
}

pub fn reverse_blueprint_apply_result(result: ReverseBlueprintApplyResult) -> Value {
    json!({
        "blueprint_applied": true,
        "blueprint_version": result.version,
        "character_count": result.character_count,
        "relationship_count": result.relationship_count,
    })
}
