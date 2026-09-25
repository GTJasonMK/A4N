use super::{
    AGENT_CAPABILITIES, AGENT_PHASE_APPLY, AGENT_PHASE_VERIFY, AgentBudget, AgentToolExposure,
    CANVAS_AGENT_CAPABILITY, CONTENT_OPTIMIZATION_AGENT_CAPABILITY, ControlAgentRunInput,
    DIRECTOR_AGENT_CAPABILITY, LocateWritingTargetInput, MAIN_AGENT_CAPABILITY,
    SearchAgentToolsInput, agent_phase_for_tool, agent_tool_descriptor,
    agent_tool_requires_user_approval, agent_tools_for_capability, capability_allows_tool,
    initial_agent_tools, scope_agent_tool_search, synthetic_agent_tool_descriptor,
};
use crate::command_manifest::command_descriptor;
use serde_json::json;

#[test]
fn control_agent_run_accepts_optional_budget_updates() {
    let existing_request = serde_json::from_value::<ControlAgentRunInput>(json!({
        "thread_id": "thread-1",
        "run_id": "run-1",
        "action": "resume"
    }))
    .expect("existing requests remain valid");
    assert!(existing_request.budget.is_none());

    let update_request = serde_json::from_value::<ControlAgentRunInput>(json!({
        "thread_id": "thread-1",
        "run_id": "run-1",
        "action": "resume",
        "budget": {"max_steps": 64}
    }))
    .expect("resume requests can carry a new budget");
    assert_eq!(
        update_request.budget,
        Some(AgentBudget {
            max_steps: Some(64),
            ..AgentBudget::default()
        })
    );
}

#[test]
fn capability_catalogs_are_non_empty_and_every_tool_is_resolvable() {
    for capability in AGENT_CAPABILITIES {
        let tools = agent_tools_for_capability(capability);
        assert!(!tools.is_empty(), "{capability} must expose tools");
        assert!(tools.contains(&"search_agent_tools"));
        assert!(tools.contains(&"describe_agent_tool"));

        for name in tools {
            let descriptor = synthetic_agent_tool_descriptor(name)
                .or_else(|| command_descriptor(name))
                .unwrap_or_else(|| panic!("{capability} tool {name} has no descriptor"));
            assert_eq!(descriptor.name, *name);
            assert!(
                !descriptor.args_schema.is_null(),
                "{name} args schema missing"
            );
            assert!(
                !descriptor.result_schema.is_null(),
                "{name} result schema missing"
            );
            assert_eq!(
                descriptor.result_schema["x-a4n-schema-quality"], "typed",
                "{name} must expose a typed result schema"
            );
            if synthetic_agent_tool_descriptor(name).is_some() {
                assert_eq!(
                    descriptor.args_schema["additionalProperties"], false,
                    "synthetic tool {name} must reject unknown arguments"
                );
            }
        }
    }
}

#[test]
fn ordinary_manifest_agent_tools_use_the_shared_descriptor_gate() {
    let descriptor = agent_tool_descriptor("read_chapter_visual_plan")
        .expect("director manifest tool should be exposed to Agent");
    assert_eq!(descriptor.name, "read_chapter_visual_plan");
    assert_eq!(
        descriptor.runtime_config_policy,
        crate::command_manifest::RuntimeConfigPolicy::None
    );
    assert!(agent_tool_descriptor("create_llm_config").is_none());
    for name in ["read_image", "web_search", "web_fetch"] {
        let descriptor = agent_tool_descriptor(name)
            .unwrap_or_else(|| panic!("{name} web tool should be exposed to Agent"));
        assert_eq!(
            descriptor.agent_policy.unwrap().exposure,
            AgentToolExposure::Discoverable
        );
    }
    assert!(agent_tool_descriptor("list_agent_skills").is_none());
    assert!(agent_tool_descriptor("update_agent_skill_configuration").is_none());
}

#[test]
fn restricted_tool_search_is_pinned_without_misreading_capability_as_domain() {
    let mut input = SearchAgentToolsInput {
        query: Some("edit article".into()),
        domain: Some(CONTENT_OPTIMIZATION_AGENT_CAPABILITY.into()),
        effect: Some("read".into()),
        capability: Some(MAIN_AGENT_CAPABILITY.into()),
        limit: Some(20),
    };

    scope_agent_tool_search(CONTENT_OPTIMIZATION_AGENT_CAPABILITY, &mut input);

    assert_eq!(
        input.capability.as_deref(),
        Some(CONTENT_OPTIMIZATION_AGENT_CAPABILITY)
    );
    assert_eq!(input.domain, None);
}

#[test]
fn restricted_capabilities_allow_their_complete_catalog_not_just_initial_tools() {
    for capability in [
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
        CANVAS_AGENT_CAPABILITY,
        DIRECTOR_AGENT_CAPABILITY,
    ] {
        let tools = initial_agent_tools(capability);
        assert!(!tools.is_empty());
        assert!(tools.contains(&"load_agent_skill"));
        assert!(
            agent_tools_for_capability(capability)
                .iter()
                .all(|tool| capability_allows_tool(capability, tool))
        );
        assert!(!capability_allows_tool(capability, "delete_project"));
    }
    assert!(capability_allows_tool(
        MAIN_AGENT_CAPABILITY,
        "describe_agent_tool"
    ));
    for capability in [
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
        CANVAS_AGENT_CAPABILITY,
    ] {
        assert!(!initial_agent_tools(capability).contains(&"describe_agent_tool"));
    }

    for tool in [
        "search_agent_tools",
        "describe_agent_tool",
        "list_images_by_projects",
        "preview_visual_plan_point_generation",
        "read_visual_plan_prompt_candidate",
        "start_visual_plan_prompt_generation",
        "start_image_generation",
        "wait_generation_task",
    ] {
        assert!(
            capability_allows_tool(DIRECTOR_AGENT_CAPABILITY, tool),
            "Director must allow {tool}"
        );
    }
}

#[test]
fn phases_describe_operations_without_replacing_permissions() {
    assert_eq!(agent_phase_for_tool("read_project_context"), "context_read");
    assert_eq!(
        agent_phase_for_tool("apply_optimization_patches"),
        AGENT_PHASE_APPLY
    );
    assert_eq!(
        agent_phase_for_tool("read_agent_execution_result"),
        AGENT_PHASE_VERIFY
    );
    assert!(capability_allows_tool(
        DIRECTOR_AGENT_CAPABILITY,
        "save_chapter_visual_plan_draft"
    ));
}

#[test]
fn approval_policy_is_shared_by_agent_runtimes() {
    assert!(agent_tool_requires_user_approval(
        "delete_project",
        "destructive"
    ));
    assert!(agent_tool_requires_user_approval(
        "run_canvas_graph",
        "write"
    ));
    assert!(!agent_tool_requires_user_approval(
        "create_project",
        "write"
    ));
}

#[test]
fn locate_writing_target_accepts_model_camel_case_arguments() {
    let input: LocateWritingTargetInput = serde_json::from_value(serde_json::json!({
        "projectId": 7,
        "chapterId": 11,
        "includeEmpty": true,
    }))
    .expect("camelCase Agent tool arguments should deserialize");
    assert_eq!(input.project_id, Some(7));
    assert_eq!(input.chapter_id, Some(11));
    assert!(input.include_empty);
    assert!(
        serde_json::from_value::<LocateWritingTargetInput>(serde_json::json!({
            "project_id": 7,
        }))
        .is_err()
    );
}

#[test]
fn synthetic_descriptors_have_explicit_host_metadata() {
    let descriptor = synthetic_agent_tool_descriptor("locate_writing_target")
        .expect("locate tool must have a descriptor");
    assert_eq!(descriptor.kind, "agent_tool");
    assert_eq!(descriptor.auth, "user");
    assert!(descriptor.page_scopes.is_empty());
    assert!(descriptor.required_ui_context.is_empty());
    assert!(descriptor.navigation_hint.is_none());
    assert_eq!(
        descriptor.runtime_config_policy,
        crate::command_manifest::RuntimeConfigPolicy::None
    );
    assert_eq!(
        descriptor.invoke,
        "internal agent tool: locate_writing_target"
    );
}
