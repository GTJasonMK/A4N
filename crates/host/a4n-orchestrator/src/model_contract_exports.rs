use a4n_domain::{
    json_contract::{json_schema_from_output_shape, typed_output_schema},
    project::*,
};
use a4n_test_support::model_contract::emit_schema;

#[test]
fn model_contract_matrix_owned_domain_schemas() {
    for (name, shape) in [
        ("blueprint_story_core", BLUEPRINT_STORY_CORE_SHAPE),
        ("blueprint_world", BLUEPRINT_WORLD_SHAPE),
        ("blueprint_characters", BLUEPRINT_CHARACTERS_SHAPE),
        ("blueprint_relationships", BLUEPRINT_RELATIONSHIPS_SHAPE),
    ] {
        emit_schema(
            name,
            json_schema_from_output_shape(&serde_json::from_str(shape).unwrap()).unwrap(),
        );
    }
    emit_schema(
        "blueprint_optimization",
        blueprint_refinement_output_schema(),
    );
    emit_schema(
        "part_outline_single",
        a4n_domain::chapter::part_outline_output_schema(),
    );
    emit_schema(
        "outline",
        a4n_domain::chapter::chapter_outline_output_schema(None),
    );
    emit_schema(
        "inspiration",
        a4n_domain::inspiration::inspiration_output_schema(),
    );
    for name in [
        a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS,
        a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_SUMMARY,
        a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
        a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
    ] {
        emit_schema(
            name,
            a4n_chapter_analysis::chapter_analysis_output_schema(name).unwrap(),
        );
    }
    emit_schema(
        "chapter_media_adaptation_plan",
        typed_output_schema::<a4n_domain::media_authoring::AdaptationPlanV1>(),
    );
    emit_schema(
        "project_fact_consolidation",
        typed_output_schema::<a4n_domain::fact_feedback::FactConsolidationOutput>(),
    );
}
