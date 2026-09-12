use super::super::super::*;
use super::super::support::{pack_ref, requirement};

#[test]
fn budget_planner_drops_optional_packs_after_required_content() {
    let plan = ContextBudgetPlanner::new(100)
        .with_input_utilization_percent(100)
        .plan_requirements(
            1_000,
            200,
            vec![
                requirement("source", COMPRESSION_FOCUSED_RAW, 300, true),
                requirement("assets", COMPRESSION_FOCUSED_RAW, 500, false),
            ],
            150,
        )
        .expect("plan");

    assert_eq!(plan.available_input_tokens, 700);
    assert_eq!(plan.stable_prefix_tokens, 300);
    assert_eq!(plan.dynamic_delta_tokens, 150);
    assert_eq!(plan.selected_pack_refs.len(), 1);
    assert_eq!(plan.dropped_optional_pack_refs.len(), 1);
}

#[test]
fn budget_planner_uses_input_utilization_cap() {
    let plan = ContextBudgetPlanner::new(100)
        .plan_requirements(1_000, 200, vec![], 150)
        .expect("plan");

    assert_eq!(plan.raw_available_input_tokens, 700);
    assert_eq!(plan.available_input_tokens, 595);
    assert_eq!(plan.input_utilization_percent, 85);
}

#[test]
fn adaptive_budget_reserves_the_largest_output_that_fits_required_context() {
    let plan = ContextBudgetPlanner::new(0)
        .with_input_utilization_percent(100)
        .plan_requirements_adaptive(
            100,
            90,
            vec![requirement("source", COMPRESSION_MINIMAL, 20, true)],
            0,
        )
        .expect("adaptive plan");

    assert_eq!(plan.output_token_reserve, 80);
    assert_eq!(plan.estimated_input_tokens, 20);
}

#[test]
fn budget_planner_selects_compressed_required_variant() {
    let plan = ContextBudgetPlanner::new(100)
        .with_input_utilization_percent(100)
        .plan_requirements(
            1_000,
            200,
            vec![LlmContextPackRequirement {
                pack_kind: "chapter".to_string(),
                required: true,
                stable_prefix: true,
                variants: vec![
                    pack_ref("chapter", COMPRESSION_FULL_RAW, 650),
                    pack_ref("chapter", COMPRESSION_COMPRESSED_SCENE, 300),
                ],
            }],
            150,
        )
        .expect("compressed variant should fit");

    assert_eq!(plan.selected_pack_refs.len(), 1);
    assert_eq!(
        plan.selected_pack_refs[0].compression_level,
        COMPRESSION_COMPRESSED_SCENE
    );
    assert_eq!(plan.estimated_input_tokens, 450);
}

#[test]
fn budget_planner_reserves_space_for_later_required_packs() {
    let plan = ContextBudgetPlanner::new(100)
        .with_input_utilization_percent(100)
        .plan_requirements(
            1_000,
            200,
            vec![
                LlmContextPackRequirement {
                    pack_kind: "large_source".to_string(),
                    required: true,
                    stable_prefix: true,
                    variants: vec![
                        pack_ref("large_source", COMPRESSION_FULL_RAW, 500),
                        pack_ref("large_source", COMPRESSION_MINIMAL, 100),
                    ],
                },
                requirement("contract", COMPRESSION_FULL_RAW, 300, true),
            ],
            150,
        )
        .expect("planner should downgrade earlier pack for later required pack");

    assert_eq!(plan.selected_pack_refs.len(), 2);
    assert_eq!(
        plan.selected_pack_refs[0].compression_level,
        COMPRESSION_MINIMAL
    );
    assert_eq!(plan.estimated_input_tokens, 550);
}
