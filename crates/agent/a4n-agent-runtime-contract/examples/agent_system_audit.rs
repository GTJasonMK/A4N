//! Read-only Agent audit probes and regression checks for fixed defects.
use a4n_agent_runtime_contract::{
    child_budget_within_parent, deterministic_context_summary, enforce_delegation_limits,
};
use a4n_api_contract::agent::{
    AgentBudget, ControlAgentRunInput, agent_tools_for_capability, capability_allows_tool,
};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut blocked = serde_json::Map::new();
    for capability in ["director", "content_optimization", "canvas"] {
        let tools = agent_tools_for_capability(capability)
            .iter()
            .filter(|tool| !capability_allows_tool(capability, tool))
            .copied()
            .collect::<Vec<_>>();
        assert!(!tools.contains(&"load_agent_skill"));
        blocked.insert(capability.into(), json!(tools));
    }
    for tool in [
        "list_images_by_projects",
        "preview_visual_plan_point_generation",
        "read_visual_plan_prompt_candidate",
        "start_visual_plan_prompt_generation",
        "start_image_generation",
        "wait_generation_task",
    ] {
        assert!(capability_allows_tool("director", tool), "{tool}");
    }
    let budget_change = serde_json::from_value::<ControlAgentRunInput>(json!({
        "thread_id": "1", "run_id": "1", "action": "resume",
        "budget": {"max_steps": 64}
    }))?;
    assert_eq!(budget_change.budget.unwrap().max_steps, Some(64));

    let mut previous: Option<Value> = None;
    for sequence in 1..=64 {
        previous = Some(serde_json::to_value(deterministic_context_summary(
            previous.as_ref(),
            Some("old goal"),
            &format!("fact {sequence}"),
            sequence,
        )?)?);
    }
    let bounded = deterministic_context_summary(previous.as_ref(), None, "fact 65", 65)?;
    assert_eq!(bounded.facts_resources.len(), 64);
    assert_eq!(
        bounded.facts_resources.last().map(String::as_str),
        Some("fact 65")
    );

    let initial = deterministic_context_summary(None, Some("old goal"), "old fact", 1)?;
    let changed_goal = deterministic_context_summary(
        Some(&serde_json::to_value(initial)?),
        Some("new goal"),
        "new fact",
        2,
    )?;
    assert_eq!(changed_goal.current_goal, "new goal");

    let source = format!("{}DO_NOT_DELETE_CHARACTERS", "x".repeat(2_000));
    let truncated = deterministic_context_summary(None, Some("task"), &source, 1)?;
    assert!(serde_json::to_string(&truncated)?.contains("DO_NOT_DELETE_CHARACTERS"));
    assert!(truncated.constraints.is_empty());

    let parent = AgentBudget {
        max_delegation_depth: Some(2),
        ..AgentBudget::default()
    };
    let child = child_budget_within_parent(AgentBudget::default(), &parent, &json!({}))?;
    assert_eq!(child.max_delegation_depth, Some(2));
    let grandchild = enforce_delegation_limits(&[], 2, 1, &child);
    grandchild?;
    let too_deep = enforce_delegation_limits(&[], 3, 2, &child);
    assert!(too_deep.is_err());

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "meaning": "audit probes and regression checks; not a full acceptance suite",
            "declared_but_blocked_tools": blocked,
            "resume_budget_supported": true,
            "summary_65_fact_count": bounded.facts_resources.len(),
            "goal_after_new_goal": changed_goal.current_goal,
            "summary_retains_source_tail": serde_json::to_string(&truncated)?.contains("DO_NOT_DELETE_CHARACTERS"),
            "depth_2_run_cannot_delegate": too_deep.unwrap_err().to_string()
        }))?
    );
    Ok(())
}
