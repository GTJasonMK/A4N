use super::super::super::*;
use super::super::support::{pack_ref, requirement};

#[test]
fn budget_planner_rejects_required_pack_that_cannot_fit() {
    let err = ContextBudgetPlanner::new(100)
        .plan_requirements(
            1_000,
            200,
            vec![requirement("source", COMPRESSION_FULL_RAW, 800, true)],
            150,
        )
        .expect_err("required pack should fail");

    assert!(err.to_string().contains("必需上下文包"));
}

#[test]
fn budget_planner_rejects_required_minimal_overflow() {
    let err = ContextBudgetPlanner::new(100)
        .with_input_utilization_percent(100)
        .plan_requirements(
            1_000,
            200,
            vec![LlmContextPackRequirement {
                pack_kind: "chapter".to_string(),
                required: true,
                stable_prefix: true,
                variants: vec![
                    pack_ref("chapter", COMPRESSION_FULL_RAW, 900),
                    pack_ref("chapter", COMPRESSION_MINIMAL, 800),
                ],
            }],
            150,
        )
        .expect_err("required minimal variant should overflow");

    assert!(err.to_string().contains("最小压缩变体"));
}

#[test]
fn budget_planner_rejects_reserved_token_sum_overflow() {
    let err = ContextBudgetPlanner::new(usize::MAX)
        .plan_requirements(usize::MAX, 1, vec![], 0)
        .expect_err("reserved token sum overflow must be visible");

    assert!(err.to_string().contains("token 预算计算溢出 usize"));
    assert!(err.to_string().contains("输出预留与安全余量之和"));
}

#[test]
fn budget_planner_rejects_required_suffix_sum_overflow() {
    let err = ContextBudgetPlanner::new(0)
        .with_input_utilization_percent(1)
        .plan_requirements(
            usize::MAX,
            0,
            vec![
                requirement("source", COMPRESSION_MINIMAL, usize::MAX, true),
                requirement("contract", COMPRESSION_MINIMAL, usize::MAX, true),
            ],
            0,
        )
        .expect_err("required suffix token sum overflow must be visible");

    assert!(err.to_string().contains("token 预算计算溢出 usize"));
    assert!(err.to_string().contains("后续必需上下文最小 token 之和"));
}
