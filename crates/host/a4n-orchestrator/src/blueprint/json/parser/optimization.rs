use a4n_domain::project::ParsedBlueprintOptimization;
use a4n_error::AppError;

mod scalars;
mod sections;
mod top_level;
use scalars::{ensure_optimization_has_value, parse_optimization_scalars};
use sections::parse_optimization_sections;
use top_level::parse_optimization_top_level;

pub(in crate::blueprint) fn parse_blueprint_optimization_json(
    content: &str,
) -> Result<ParsedBlueprintOptimization, AppError> {
    let value = parse_optimization_top_level(content)?;

    let (style, full_synopsis) = parse_optimization_scalars(&value)?;
    let (world_setting, characters, relationships) = parse_optimization_sections(&value)?;

    ensure_optimization_has_value(
        style.as_ref(),
        full_synopsis.as_ref(),
        world_setting.as_ref(),
        characters.as_ref(),
        relationships.as_ref(),
    )?;

    Ok(ParsedBlueprintOptimization {
        raw: value,
        style,
        full_synopsis,
        world_setting,
        characters,
        relationships,
    })
}
