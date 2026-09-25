use a4n_api_contract::agent::{
    AgentResourceRef, AgentScope, CANVAS_AGENT_CAPABILITY, CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
    DIRECTOR_AGENT_CAPABILITY,
};
use a4n_error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalSubagentBinding {
    pub(crate) key: String,
    pub(crate) project_id: Option<i64>,
    pub(crate) chapter_id: Option<i64>,
    pub(crate) bound_resource: Option<AgentResourceRef>,
}

impl CanonicalSubagentBinding {
    pub(crate) fn scope(&self, capability: &str) -> AgentScope {
        let (resource_kind, resource_id) = self
            .bound_resource
            .as_ref()
            .map(|resource| (Some(resource.kind.clone()), Some(resource.id.clone())))
            .unwrap_or((None, None));
        let allowed_capabilities =
            if a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability) {
                vec![capability.to_string()]
            } else {
                Vec::new()
            };
        AgentScope {
            resource_kind,
            resource_id,
            project_id: self.project_id,
            chapter_id: self.chapter_id,
            chapter_number: None,
            allowed_capabilities,
            allowed_tools: Vec::new(),
        }
    }
}

pub(crate) fn canonical_subagent_binding(
    capability: &str,
    project_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<CanonicalSubagentBinding, AppError> {
    let capability = capability.trim();
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

    let (project_id, chapter_id, bound_resource, key) = match capability {
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY => {
            let project_id = positive_id(project_id, "content_optimization projectId")?;
            let chapter_id = positive_id(chapter_id, "content_optimization chapterId")?;
            (
                Some(project_id),
                Some(chapter_id),
                Some(resource("chapter", chapter_id)),
                a4n_api_contract::agent::canonical_subagent_binding_key(
                    capability,
                    "chapter",
                    &chapter_id.to_string(),
                ),
            )
        }
        CANVAS_AGENT_CAPABILITY => {
            let project_id = positive_id(project_id, "canvas projectId")?;
            if chapter_id.is_some() {
                return Err(AppError::InvalidArgument(
                    "canvas 子 Agent 只能绑定项目，不能绑定章节".into(),
                ));
            }
            (
                Some(project_id),
                None,
                Some(resource("project", project_id)),
                a4n_api_contract::agent::canonical_subagent_binding_key(
                    capability,
                    "project",
                    &project_id.to_string(),
                ),
            )
        }
        DIRECTOR_AGENT_CAPABILITY => {
            let project_id = positive_id(project_id, "director projectId")?;
            let chapter_id = positive_id(chapter_id, "director chapterId")?;
            (
                Some(project_id),
                Some(chapter_id),
                Some(resource("chapter", chapter_id)),
                a4n_api_contract::agent::canonical_subagent_binding_key(
                    capability,
                    "chapter",
                    &chapter_id.to_string(),
                ),
            )
        }
        _ => match (project_id, chapter_id) {
            (Some(project_id), Some(chapter_id)) => {
                let project_id = positive_id(Some(project_id), "generic projectId")?;
                let chapter_id = positive_id(Some(chapter_id), "generic chapterId")?;
                (
                    Some(project_id),
                    Some(chapter_id),
                    Some(resource("chapter", chapter_id)),
                    a4n_api_contract::agent::canonical_subagent_binding_key(
                        capability,
                        "chapter",
                        &chapter_id.to_string(),
                    ),
                )
            }
            (Some(project_id), None) => {
                let project_id = positive_id(Some(project_id), "generic projectId")?;
                (
                    Some(project_id),
                    None,
                    Some(resource("project", project_id)),
                    a4n_api_contract::agent::canonical_subagent_binding_key(
                        capability,
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
        },
    };

    Ok(CanonicalSubagentBinding {
        key,
        project_id,
        chapter_id,
        bound_resource,
    })
}

fn resource(kind: &str, id: i64) -> AgentResourceRef {
    AgentResourceRef {
        kind: kind.into(),
        id: id.to_string(),
        label: None,
        revision: None,
        hash: None,
    }
}

fn positive_id(value: Option<i64>, field: &str) -> Result<i64, AppError> {
    value
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须提供正整数")))
}
