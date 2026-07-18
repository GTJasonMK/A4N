use super::ProjectBlueprintGraph;
use crate::project::duplicate::ProjectDuplicateError;

pub(in crate::project::duplicate::text) fn project_blueprint_graph_raw_blueprint(
    graph: &ProjectBlueprintGraph,
) -> Result<String, ProjectDuplicateError> {
    serde_json::to_string_pretty(&serde_json::json!({
        "blueprint": {
            "title": graph.blueprint.title,
            "genre": graph.blueprint.genre,
            "style": graph.blueprint.style,
            "tone": graph.blueprint.tone,
            "target_audience": graph.blueprint.target_audience,
            "one_sentence_summary": graph.blueprint.one_sentence_summary,
            "full_synopsis": graph.blueprint.full_synopsis,
            "world_setting": graph.blueprint.world_setting,
            "needs_part_outlines": graph.blueprint.needs_part_outlines,
            "total_chapters": graph.blueprint.total_chapters,
            "chapters_per_part": graph.blueprint.chapters_per_part,
        },
        "characters": graph
            .characters
            .iter()
            .map(|character| {
                serde_json::json!({
                    "name": character.name,
                    "character_type": character.character_type,
                    "gender": character.gender,
                    "age": character.age,
                    "appearance": character.appearance,
                    "personality": character.personality,
                    "background": character.background,
                    "motivation": character.motivation,
                    "arc_summary": character.arc_summary,
                    "relationship_to_protagonist": character.relationship_to_protagonist,
                    "arc_stages": character.arc_stages,
                })
            })
            .collect::<Vec<_>>(),
        "relationships": graph
            .relationships
            .iter()
            .map(|relationship| {
                serde_json::json!({
                    "relationship_type": relationship.relationship_type,
                    "character_a": relationship.character_a,
                    "character_b": relationship.character_b,
                    "description": relationship.description,
                    "strength": relationship.strength,
                    "evolution": relationship.evolution,
                })
            })
            .collect::<Vec<_>>(),
    }))
    .map_err(ProjectDuplicateError::Serialization)
}
