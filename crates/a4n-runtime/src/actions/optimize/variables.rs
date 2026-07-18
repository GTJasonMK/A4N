use std::collections::HashMap;

use a4n_orchestrator::optimize::OptimizeAgentPermissions;
use serde_json::Value;

pub(super) fn optimization_agent_variables(
    permissions: Option<&OptimizeAgentPermissions>,
) -> HashMap<String, Value> {
    let permissions = permissions.cloned().unwrap_or_default();
    HashMap::from([("agent_permissions".to_string(), permissions.as_value())])
}

pub(super) fn optimization_analysis_variables(
    mode: &str,
    scope: Option<&str>,
    permissions: Option<&OptimizeAgentPermissions>,
    agent_instruction: Option<&str>,
) -> HashMap<String, Value> {
    let mut variables = optimization_agent_variables(permissions);
    variables.insert("mode".to_string(), Value::String(mode.to_string()));
    if let Some(scope) = scope {
        variables.insert("scope".to_string(), Value::String(scope.to_string()));
    }
    if let Some(instruction) = agent_instruction
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        variables.insert(
            "agent_instruction".to_string(),
            Value::String(instruction.to_string()),
        );
    }
    variables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_variables_keep_required_mode_and_default_permissions() {
        let variables = optimization_analysis_variables("review", None, None, None);

        assert_eq!(variables.get("mode"), Some(&Value::String("review".into())));
        assert!(variables.contains_key("agent_permissions"));
        assert!(!variables.contains_key("scope"));
        assert!(!variables.contains_key("agent_instruction"));
    }

    #[test]
    fn analysis_variables_trim_instruction_but_preserve_scope() {
        let variables = optimization_analysis_variables(
            "rewrite",
            Some(" chapter "),
            None,
            Some("  保留叙事视角  "),
        );

        assert_eq!(
            variables.get("scope"),
            Some(&Value::String(" chapter ".into()))
        );
        assert_eq!(
            variables.get("agent_instruction"),
            Some(&Value::String("保留叙事视角".into()))
        );
    }

    #[test]
    fn analysis_variables_drop_blank_instruction() {
        let variables = optimization_analysis_variables("review", None, None, Some("  "));

        assert!(!variables.contains_key("agent_instruction"));
    }
}
