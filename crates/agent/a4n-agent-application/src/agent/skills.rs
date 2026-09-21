use super::AgentApplication;
use a4n_api_contract::agent::{
    AGENT_CAPABILITIES, AgentSkillContent, AgentSkillDescriptor, AgentSkillProfileView,
    AgentSkillSearchResult, AgentSkillSettingsView, CANVAS_AGENT_CAPABILITY,
    CONTENT_OPTIMIZATION_AGENT_CAPABILITY, DIRECTOR_AGENT_CAPABILITY, GENERAL_AGENT_CAPABILITY,
    LoadAgentSkillInput, MAIN_AGENT_CAPABILITY, SearchAgentSkillsInput,
    UpdateAgentSkillConfigurationInput,
};
use a4n_error::AppError;
use a4n_worker_contract::{
    AgentWorkerRunLeaseRequest, AgentWorkerSkillLoadRequest, AgentWorkerSkillSearchRequest,
};
use std::collections::HashSet;

const MAX_SKILL_IDS: usize = 32;
const MAX_SKILL_CONTENT_BYTES: usize = 12 * 1024;

struct BuiltinSkill {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    version: &'static str,
    capabilities: &'static [&'static str],
    content: &'static str,
}

const BUILTIN_SKILLS: &[BuiltinSkill] = &[
    BuiltinSkill {
        id: "a4n-agent-planning",
        name: "Agent 任务规划",
        description: "拆解多步骤目标，按事实、动作、审批和验证组织执行。",
        version: "1.0.0",
        capabilities: &["*"],
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../prompts/skills/a4n-agent-planning/SKILL.md"
        )),
    },
    BuiltinSkill {
        id: "a4n-novel-writing",
        name: "小说写作",
        description: "围绕章节事实、人物声音、因果和叙事一致性进行创作检查。",
        version: "1.0.0",
        capabilities: &[
            MAIN_AGENT_CAPABILITY,
            GENERAL_AGENT_CAPABILITY,
            CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
        ],
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../prompts/skills/a4n-novel-writing/SKILL.md"
        )),
    },
    BuiltinSkill {
        id: "a4n-visual-direction",
        name: "视觉导演",
        description: "围绕镜头、空间轴线、动作连续性和可剪辑性设计视觉方案。",
        version: "1.0.0",
        capabilities: &[CANVAS_AGENT_CAPABILITY, DIRECTOR_AGENT_CAPABILITY],
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../prompts/skills/a4n-visual-direction/SKILL.md"
        )),
    },
];

impl AgentApplication {
    pub(crate) async fn list_skill_settings(
        &self,
        user_id: i64,
    ) -> Result<AgentSkillSettingsView, AppError> {
        super::validate_user_id(user_id)?;
        let mut profiles = Vec::with_capacity(AGENT_CAPABILITIES.len());
        for capability in AGENT_CAPABILITIES {
            profiles.push(self.skill_profile_view(user_id, capability).await?);
        }
        Ok(AgentSkillSettingsView { profiles })
    }

    pub(crate) async fn update_skill_configuration(
        &self,
        user_id: i64,
        input: UpdateAgentSkillConfigurationInput,
    ) -> Result<AgentSkillProfileView, AppError> {
        super::validate_user_id(user_id)?;
        let capability = super::required_capability(input.capability)?;
        if !AGENT_CAPABILITIES.contains(&capability.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "不支持配置的 Agent capability: {capability}"
            )));
        }
        let available = applicable_skills(&capability);
        let available_ids = available
            .iter()
            .map(|skill| skill.id)
            .collect::<HashSet<_>>();
        if input.enabled_skill_ids.len() > MAX_SKILL_IDS {
            return Err(AppError::InvalidArgument(format!(
                "Agent Skill 数量不能超过 {MAX_SKILL_IDS}"
            )));
        }
        let mut seen = HashSet::with_capacity(input.enabled_skill_ids.len());
        for id in &input.enabled_skill_ids {
            if id.trim().is_empty() || id.len() > 160 || !seen.insert(id.as_str()) {
                return Err(AppError::InvalidArgument(
                    "enabled_skill_ids 含有空白、重复或过长的 Skill ID".into(),
                ));
            }
            if !available_ids.contains(id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "Skill {id} 不属于 capability {capability}"
                )));
            }
        }
        a4n_db::agent::save_agent_skill_profile_ids(
            self.connection(),
            user_id,
            &capability,
            &input.enabled_skill_ids,
        )
        .await
        .map_err(super::agent_db_error_to_app)?;
        self.skill_profile_view(user_id, &capability).await
    }

    pub(crate) async fn search_skills(
        &self,
        user_id: i64,
        capability: &str,
        input: SearchAgentSkillsInput,
    ) -> Result<AgentSkillSearchResult, AppError> {
        super::validate_user_id(user_id)?;
        let capability = super::required_capability(capability.to_string())?;
        if input.query.as_deref().is_some_and(|query| {
            query.chars().count() > 200 || query.chars().any(|ch| ch.is_control())
        }) {
            return Err(AppError::InvalidArgument("Skill 搜索关键词无效".into()));
        }
        let enabled = self.enabled_skill_ids(user_id, &capability).await?;
        let query_terms = input
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|query| {
                query
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|terms| !terms.is_empty());
        let limit = super::bounded_limit(
            input.limit,
            a4n_api_contract::agent::AGENT_SKILLS_DEFAULT_LIMIT,
            a4n_api_contract::agent::AGENT_SKILLS_MAX_LIMIT,
            "limit",
        )? as usize;
        let mut skills = applicable_skills(&capability)
            .into_iter()
            .filter(|skill| enabled.contains(skill.id))
            .filter(|skill| {
                query_terms
                    .as_ref()
                    .is_none_or(|terms| skill_matches_query(skill, terms))
            })
            .map(|skill| skill_descriptor(skill, true))
            .collect::<Vec<_>>();
        let total = skills.len();
        skills.truncate(limit);
        Ok(AgentSkillSearchResult { skills, total })
    }

    pub(crate) async fn load_skill(
        &self,
        user_id: i64,
        capability: &str,
        input: LoadAgentSkillInput,
    ) -> Result<AgentSkillContent, AppError> {
        super::validate_user_id(user_id)?;
        let capability = super::required_capability(capability.to_string())?;
        let id = input.id.trim();
        if id.is_empty() || id.len() > 160 {
            return Err(AppError::InvalidArgument("Skill ID 无效".into()));
        }
        let enabled = self.enabled_skill_ids(user_id, &capability).await?;
        let skill = applicable_skills(&capability)
            .into_iter()
            .find(|skill| skill.id == id)
            .ok_or_else(|| {
                AppError::NotFound(format!("Skill 不存在或不适用于 {capability}: {id}"))
            })?;
        if !enabled.contains(skill.id) {
            return Err(AppError::Forbidden(format!(
                "Skill 未为 capability {capability} 启用: {id}"
            )));
        }
        if skill.content.len() > MAX_SKILL_CONTENT_BYTES {
            return Err(AppError::InvalidState {
                current: format!("Skill {id} 正文超过大小限制"),
                operation: "加载 Agent Skill".into(),
            });
        }
        Ok(AgentSkillContent {
            skill: skill_descriptor(skill, true),
            content: skill.content.to_string(),
        })
    }

    pub(crate) async fn search_skills_for_worker(
        &self,
        request: &AgentWorkerSkillSearchRequest,
    ) -> Result<AgentSkillSearchResult, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.search_skills(
            claimed.thread.owner_user_id,
            &claimed.thread.profile_id,
            request.input.clone(),
        )
        .await
    }

    pub(crate) async fn load_skill_for_worker(
        &self,
        request: &AgentWorkerSkillLoadRequest,
    ) -> Result<AgentSkillContent, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.load_skill(
            claimed.thread.owner_user_id,
            &claimed.thread.profile_id,
            request.input.clone(),
        )
        .await
    }

    async fn skill_profile_view(
        &self,
        user_id: i64,
        capability: &str,
    ) -> Result<AgentSkillProfileView, AppError> {
        let enabled = self.enabled_skill_ids(user_id, capability).await?;
        Ok(AgentSkillProfileView {
            capability: capability.to_string(),
            skills: applicable_skills(capability)
                .into_iter()
                .map(|skill| skill_descriptor(skill, enabled.contains(skill.id)))
                .collect(),
        })
    }

    async fn enabled_skill_ids(
        &self,
        user_id: i64,
        capability: &str,
    ) -> Result<HashSet<String>, AppError> {
        let available = applicable_skills(capability);
        let available_ids = available
            .iter()
            .map(|skill| skill.id)
            .collect::<HashSet<_>>();
        let stored = a4n_db::agent::agent_skill_profile_ids(self.connection(), user_id, capability)
            .await
            .map_err(super::agent_db_error_to_app)?;
        let ids =
            stored.unwrap_or_else(|| available.iter().map(|skill| skill.id.to_string()).collect());
        if ids.len() > MAX_SKILL_IDS
            || ids
                .iter()
                .any(|id| id.trim().is_empty() || !available_ids.contains(id.as_str()))
        {
            return Err(AppError::InvalidState {
                current: format!("Agent capability {capability} 的 Skill 配置无效"),
                operation: "读取 Agent Skill 配置".into(),
            });
        }
        let unique = ids.iter().cloned().collect::<HashSet<_>>();
        if unique.len() != ids.len() {
            return Err(AppError::InvalidState {
                current: format!("Agent capability {capability} 的 Skill 配置存在重复项"),
                operation: "读取 Agent Skill 配置".into(),
            });
        }
        Ok(unique)
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::agent::AgentSkillApplicationPort for AgentApplication {
    async fn list_skill_settings(&self, user_id: i64) -> Result<AgentSkillSettingsView, AppError> {
        AgentApplication::list_skill_settings(self, user_id).await
    }

    async fn update_skill_configuration(
        &self,
        user_id: i64,
        input: UpdateAgentSkillConfigurationInput,
    ) -> Result<AgentSkillProfileView, AppError> {
        AgentApplication::update_skill_configuration(self, user_id, input).await
    }

    async fn search_skills(
        &self,
        user_id: i64,
        capability: &str,
        input: SearchAgentSkillsInput,
    ) -> Result<AgentSkillSearchResult, AppError> {
        AgentApplication::search_skills(self, user_id, capability, input).await
    }

    async fn load_skill(
        &self,
        user_id: i64,
        capability: &str,
        input: LoadAgentSkillInput,
    ) -> Result<AgentSkillContent, AppError> {
        AgentApplication::load_skill(self, user_id, capability, input).await
    }
}

fn applicable_skills(capability: &str) -> Vec<&'static BuiltinSkill> {
    BUILTIN_SKILLS
        .iter()
        .filter(|skill| {
            skill.capabilities.contains(&"*") || skill.capabilities.contains(&capability)
        })
        .collect()
}

fn skill_descriptor(skill: &BuiltinSkill, enabled: bool) -> AgentSkillDescriptor {
    AgentSkillDescriptor {
        id: skill.id.into(),
        name: skill.name.into(),
        description: skill.description.into(),
        version: skill.version.into(),
        source: "builtin".into(),
        content_hash: a4n_db::agent::sha256_hex(skill.content),
        capabilities: skill
            .capabilities
            .iter()
            .map(|value| (*value).into())
            .collect(),
        enabled,
    }
}

fn skill_matches_query(skill: &BuiltinSkill, terms: &[String]) -> bool {
    terms.iter().any(|term| {
        skill.id.to_lowercase().contains(term)
            || skill.name.to_lowercase().contains(term)
            || skill.description.to_lowercase().contains(term)
    })
}

#[cfg(test)]
mod tests {
    use super::{BUILTIN_SKILLS, applicable_skills, skill_descriptor, skill_matches_query};

    #[test]
    fn builtin_skills_have_bounded_content_and_stable_metadata() {
        assert!(!BUILTIN_SKILLS.is_empty());
        for skill in BUILTIN_SKILLS {
            assert!(!skill.id.is_empty());
            assert!(!skill.name.is_empty());
            assert!(!skill.description.is_empty());
            assert!(!skill.content.is_empty());
            assert!(skill.content.len() <= super::MAX_SKILL_CONTENT_BYTES);
            let descriptor = skill_descriptor(skill, true);
            assert_eq!(descriptor.source, "builtin");
            assert_eq!(
                descriptor.content_hash,
                a4n_db::agent::sha256_hex(skill.content)
            );
        }
        assert_eq!(applicable_skills("director").len(), 2);
        assert_eq!(applicable_skills("main").len(), 2);
    }

    #[test]
    fn skill_search_matches_any_query_term() {
        let skill = BUILTIN_SKILLS
            .iter()
            .find(|skill| skill.id == "a4n-novel-writing")
            .expect("builtin novel skill");
        assert!(skill_matches_query(
            skill,
            &["novel".into(), "声音".into(), "missing".into()]
        ));
        assert!(!skill_matches_query(
            skill,
            &["canvas".into(), "director".into()]
        ));
    }
}
