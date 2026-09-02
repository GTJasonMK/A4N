use a4n_api_contract::agent::{AgentResourceRef, AgentScope};
use a4n_core::error::AppError;

pub(crate) mod canvas;
pub(crate) mod content_optimization;

pub(crate) use a4n_api_contract::agent::{
    CANVAS_AGENT_CAPABILITY as CANVAS_CAPABILITY,
    CONTENT_OPTIMIZATION_AGENT_CAPABILITY as CONTENT_OPTIMIZATION_CAPABILITY,
};

pub(crate) fn tools_for_capability(capability: &str) -> &'static [&'static str] {
    a4n_api_contract::agent::agent_tools_for_capability(capability)
}

/// These actions mutate user-owned source or start a billable workflow and
/// must remain behind the durable approval boundary.
pub(crate) fn requires_user_approval(tool: &str) -> bool {
    matches!(
        tool,
        "apply_optimization_patches" | "save_canvas_graph" | "run_canvas_graph" | "run_canvas_node"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalSubagentBinding {
    /// Keep the capability label supplied by the user.  Registered domains
    /// use the same stable labels as before, while unregistered labels are
    /// intentionally valid generic Agent profiles.
    pub capability: String,
    pub key: String,
    pub project_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub bound_resource: Option<AgentResourceRef>,
    pub allowed_tools: Vec<String>,
}

impl CanonicalSubagentBinding {
    pub(crate) fn scope(&self) -> AgentScope {
        let (resource_kind, resource_id) = self
            .bound_resource
            .as_ref()
            .map(|resource| (Some(resource.kind.clone()), Some(resource.id.clone())))
            .unwrap_or((None, None));
        let allowed_capabilities =
            if a4n_api_contract::agent::capability_has_restricted_tool_catalog(&self.capability) {
                vec![self.capability.clone()]
            } else {
                // Generic/user-defined children may delegate further. An
                // explicit parent allowlist is inherited by the orchestrator.
                Vec::new()
            };
        AgentScope {
            resource_kind,
            resource_id,
            project_id: self.project_id,
            chapter_id: self.chapter_id,
            allowed_capabilities,
            allowed_tools: self.allowed_tools.clone(),
        }
    }
}

pub(crate) fn canonical_subagent_binding(
    capability: &str,
    project_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<CanonicalSubagentBinding, AppError> {
    let capability = capability.trim().to_string();
    if capability.is_empty() {
        return Err(AppError::InvalidArgument(
            "delegate capability 不能为空".into(),
        ));
    }
    if capability.len() > 128 || capability.chars().any(|ch| ch.is_control()) {
        return Err(AppError::InvalidArgument(
            "delegate capability 长度或字符无效".into(),
        ));
    }
    match capability.as_str() {
        CONTENT_OPTIMIZATION_CAPABILITY => {
            let project_id = positive_id(project_id, "content_optimization projectId")?;
            let chapter_id = positive_id(chapter_id, "content_optimization chapterId")?;
            let key = a4n_api_contract::agent::canonical_subagent_binding_key(
                &capability,
                "chapter",
                &chapter_id.to_string(),
            );
            Ok(CanonicalSubagentBinding {
                capability,
                key,
                project_id: Some(project_id),
                chapter_id: Some(chapter_id),
                bound_resource: Some(AgentResourceRef {
                    kind: "chapter".into(),
                    id: chapter_id.to_string(),
                    label: None,
                    revision: None,
                    hash: None,
                }),
                allowed_tools: a4n_api_contract::agent::CONTENT_OPTIMIZATION_AGENT_TOOLS
                    .iter()
                    .map(|tool| (*tool).to_string())
                    .collect(),
            })
        }
        CANVAS_CAPABILITY => {
            let project_id = positive_id(project_id, "canvas projectId")?;
            if chapter_id.is_some() {
                return Err(AppError::InvalidArgument(
                    "canvas 子 Agent 只能绑定项目，不能绑定章节".into(),
                ));
            }
            let key = a4n_api_contract::agent::canonical_subagent_binding_key(
                &capability,
                "project",
                &project_id.to_string(),
            );
            Ok(CanonicalSubagentBinding {
                capability,
                key,
                project_id: Some(project_id),
                chapter_id: None,
                bound_resource: Some(AgentResourceRef {
                    kind: "project".into(),
                    id: project_id.to_string(),
                    label: None,
                    revision: None,
                    hash: None,
                }),
                allowed_tools: a4n_api_contract::agent::CANVAS_AGENT_TOOLS
                    .iter()
                    .map(|tool| (*tool).to_string())
                    .collect(),
            })
        }
        _ => {
            // Generic profiles are deliberately resource-aware.  A chapter
            // inherits its project fence; a project gets a project fence;
            // without either, the profile is user-global and may only use
            // tools that perform their own per-user authorization.
            let (project_id, chapter_id, bound_resource, key) = match (project_id, chapter_id) {
                (Some(project_id), Some(chapter_id)) => (
                    Some(positive_id(Some(project_id), "generic projectId")?),
                    Some(positive_id(Some(chapter_id), "generic chapterId")?),
                    Some(AgentResourceRef {
                        kind: "chapter".into(),
                        id: chapter_id.to_string(),
                        label: None,
                        revision: None,
                        hash: None,
                    }),
                    a4n_api_contract::agent::canonical_subagent_binding_key(
                        &capability,
                        "chapter",
                        &chapter_id.to_string(),
                    ),
                ),
                (Some(project_id), None) => {
                    let project_id = positive_id(Some(project_id), "generic projectId")?;
                    (
                        Some(project_id),
                        None,
                        Some(AgentResourceRef {
                            kind: "project".into(),
                            id: project_id.to_string(),
                            label: None,
                            revision: None,
                            hash: None,
                        }),
                        a4n_api_contract::agent::canonical_subagent_binding_key(
                            &capability,
                            "project",
                            &project_id.to_string(),
                        ),
                    )
                }
                (None, Some(chapter_id)) => {
                    let _ = positive_id(Some(chapter_id), "generic chapterId")?;
                    return Err(AppError::InvalidArgument(
                        "generic chapter capability 必须同时提供 projectId".into(),
                    ));
                }
                (None, None) => (None, None, None, format!("subagent:{capability}:global")),
            };
            // Generic/user-defined profiles deliberately keep an empty
            // allowlist: an empty list means "use manifest-discoverable
            // tools" in the shared Agent scope contract.  Only registered
            // domain adapters receive a fixed catalog.
            let allowed_tools = Vec::new();
            Ok(CanonicalSubagentBinding {
                capability,
                key,
                project_id,
                chapter_id,
                bound_resource,
                allowed_tools,
            })
        }
    }
}

fn positive_id(value: Option<i64>, field: &str) -> Result<i64, AppError> {
    value
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须提供正整数")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bindings_are_stable_and_domain_scoped() {
        let optimize =
            canonical_subagent_binding(CONTENT_OPTIMIZATION_CAPABILITY, Some(7), Some(11)).unwrap();
        assert_eq!(optimize.key, "subagent:content_optimization:chapter:11");
        assert_eq!(optimize.bound_resource.as_ref().unwrap().kind, "chapter");
        assert!(
            optimize
                .allowed_tools
                .contains(&"apply_optimization_patches".into())
        );

        let canvas = canonical_subagent_binding(CANVAS_CAPABILITY, Some(7), None).unwrap();
        assert_eq!(canvas.key, "subagent:canvas:project:7");
        assert_eq!(canvas.bound_resource.as_ref().unwrap().kind, "project");
        assert!(canvas.allowed_tools.contains(&"run_canvas_graph".into()));
    }

    #[test]
    fn canonical_bindings_reject_missing_or_cross_domain_scope() {
        assert!(
            canonical_subagent_binding(CONTENT_OPTIMIZATION_CAPABILITY, Some(7), None).is_err()
        );
        assert!(canonical_subagent_binding(CANVAS_CAPABILITY, Some(7), Some(11)).is_err());
        let generic = canonical_subagent_binding("research", Some(7), None).unwrap();
        assert_eq!(generic.key, "subagent:research:project:7");
        assert!(
            generic
                .bound_resource
                .as_ref()
                .is_some_and(|resource| resource.kind == "project")
        );
        assert!(generic.scope().allowed_capabilities.is_empty());
        assert!(canonical_subagent_binding("research", None, None).is_ok());
        assert!(canonical_subagent_binding("research", None, Some(11)).is_err());
    }

    #[test]
    fn domain_tool_catalogs_are_isolated() {
        assert!(tools_for_capability("main").contains(&"delegate"));
        assert!(!tools_for_capability(CANVAS_CAPABILITY).contains(&"delegate"));
        assert!(
            tools_for_capability(CONTENT_OPTIMIZATION_CAPABILITY)
                .contains(&"list_optimize_patch_sets")
        );
        assert!(
            !tools_for_capability(CONTENT_OPTIMIZATION_CAPABILITY).contains(&"save_canvas_graph")
        );
        assert!(requires_user_approval("run_canvas_graph"));
        assert!(!requires_user_approval("send_content_optimization_message"));
    }
}
