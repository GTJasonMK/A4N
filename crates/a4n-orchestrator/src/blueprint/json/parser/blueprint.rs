use a4n_core::blueprint::ParsedBlueprint;
use a4n_core::blueprint::validate_blueprint_chapter_count;
use a4n_core::error::AppError;

mod derived;
mod metadata;
mod sections;
mod top_level;
use derived::parse_blueprint_derived_fields;
use metadata::parse_blueprint_metadata;
use sections::parse_blueprint_sections;
use top_level::parse_blueprint_top_level;

pub fn parse_blueprint_json(
    content: &str,
    requested_chapters: i32,
) -> Result<ParsedBlueprint, AppError> {
    validate_blueprint_chapter_count(requested_chapters, "requested_chapter_count")?;
    let raw = parse_blueprint_top_level(content)?;

    let (total_chapters, needs_part_outlines, chapters_per_part) =
        parse_blueprint_derived_fields(&raw, requested_chapters)?;

    let (world_setting, characters, relationships) = parse_blueprint_sections(&raw)?;

    let (title, genre, style, tone, target_audience, one_sentence_summary, full_synopsis) =
        parse_blueprint_metadata(&raw)?;

    Ok(ParsedBlueprint {
        title,
        genre,
        style,
        tone,
        target_audience,
        one_sentence_summary,
        full_synopsis,
        world_setting,
        total_chapters,
        needs_part_outlines,
        chapters_per_part,
        characters,
        relationships,
    })
}
