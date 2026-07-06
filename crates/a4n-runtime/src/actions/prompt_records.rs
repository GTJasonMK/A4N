use crate::actions::template_variables;
use crate::prompt_overrides::{
    MissingProjectMessage, ProjectOverrideIdentity, ensure_project_exists,
    normalize_project_override_identity, parse_project_override_key, project_override_key,
    validate_project_override_content, validate_project_override_project_id,
};
use crate::{AppState, actions::access};
use a4n_core::models::ai::system_settings;
use a4n_engine::prompt::engine::{PromptEntry, PromptVersion};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEntrySummary {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub dependencies: Vec<String>,
    pub is_modified: bool,
    pub version: String,
    pub modified_at: String,
    #[serde(default)]
    pub used_by: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersionSummary {
    pub version: String,
    pub timestamp: String,
    pub author: String,
    pub message: String,
    pub content_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphOutput {
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbRenderResult {
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePromptInput {
    pub name: String,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetOverrideInput {
    pub project_id: i64,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveOverrideInput {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionDiffInput {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateVarsInput {
    pub template_name: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbRenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub key: String,
    pub label: Option<String>,
    pub order: i32,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPromptOverrideSummary {
    pub project_id: i64,
    pub prompt_name: String,
    pub content: Value,
    pub updated_at: String,
}

pub async fn prompt_render(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    if let Some(project_id) = input.project_id {
        access::verify_project_editor(state, user_id, project_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    let variables = template_variables::parse_template_variables(input.variables.as_ref())?;
    state
        .engine
        .render(&input.template_name, &variables, input.project_id)
        .map_err(|e| e.to_string())
}

pub fn prompt_list(state: &AppState) -> Vec<PromptEntrySummary> {
    state
        .engine
        .list_all()
        .iter()
        .map(to_entry_summary)
        .collect()
}

pub async fn prompt_load_defaults(state: &AppState) -> Result<usize, String> {
    let existing_prompt_names = active_prompt_names(state.engine.as_ref());
    preflight_project_override_keys_for_retired_cleanup(state.db.as_ref()).await?;
    let loaded = state
        .engine
        .load_defaults()
        .await
        .map_err(|e| e.to_string())?;
    let active_prompt_names = active_prompt_names(state.engine.as_ref());
    let retired_prompt_names: HashSet<String> = existing_prompt_names
        .difference(&active_prompt_names)
        .cloned()
        .collect();
    if !retired_prompt_names.is_empty()
        && let Err(err) =
            delete_project_overrides_for_prompt_names(state.db.as_ref(), &retired_prompt_names)
                .await
    {
        return Err(format!(
            "提示词默认模板已重载，但退役提示词项目覆盖清理失败: {err}"
        ));
    }
    Ok(loaded)
}

pub async fn prompt_update(
    state: &AppState,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    let entry = state
        .engine
        .update(&input.name, input.content, &input.author)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_entry_summary(&entry))
}

pub async fn prompt_reset(state: &AppState, name: &str) -> Result<(), String> {
    state.engine.reset(name).await.map_err(|e| e.to_string())
}

pub fn prompt_version_history(
    state: &AppState,
    name: &str,
) -> Result<Vec<PromptVersionSummary>, String> {
    let versions = state
        .engine
        .version_history(name)
        .map_err(|e| e.to_string())?;
    Ok(versions.iter().map(to_version_summary).collect())
}

pub async fn prompt_set_project_override(
    state: &AppState,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let identity = normalize_project_override_identity(input.project_id, &input.name)?;
    validate_project_override_content(&input.content)?;
    state
        .engine
        .get(&identity.name)
        .map_err(|e| e.to_string())?;
    let content = input.content;
    let previous_content =
        load_project_override_persisted_content(state.db.as_ref(), &identity).await?;
    state
        .engine
        .set_project_override(identity.project_id, &identity.name, content.clone())
        .await
        .map_err(|e| e.to_string())?;

    if let Err(err) = persist_project_override(
        state.db.as_ref(),
        identity.project_id,
        &identity.name,
        &content,
    )
    .await
    {
        return Err(rollback_project_override_runtime_after_persist_error(
            state.engine.as_ref(),
            &identity,
            previous_content,
            format!("提示词覆盖持久化失败: {err}"),
            "已回滚运行时覆盖",
            "运行时覆盖已更新但回滚失败：",
        )
        .await);
    }

    Ok(())
}

pub async fn prompt_remove_project_override(
    state: &AppState,
    user_id: i64,
    input: RemoveOverrideInput,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let identity = normalize_project_override_identity(input.project_id, &input.name)?;
    let previous_content =
        load_project_override_persisted_content(state.db.as_ref(), &identity).await?;
    state
        .engine
        .remove_project_override(identity.project_id, &identity.name)
        .await
        .map_err(|e| e.to_string())?;

    if let Err(err) = delete_project_override_by_identity(state.db.as_ref(), &identity).await {
        return Err(rollback_project_override_runtime_after_persist_error(
            state.engine.as_ref(),
            &identity,
            previous_content,
            format!("提示词覆盖删除持久化失败: {err}"),
            "已恢复运行时覆盖",
            "运行时覆盖已移除但恢复失败：",
        )
        .await);
    }

    Ok(())
}

pub fn prompt_version_diff(state: &AppState, input: VersionDiffInput) -> Result<String, String> {
    state
        .engine
        .diff(&input.name, &input.version_a, &input.version_b)
        .map_err(|e| e.to_string())
}

pub fn prompt_validate_variables(
    state: &AppState,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    let variables: HashSet<String> = input.variables.into_iter().collect();
    state
        .engine
        .validate_variables(&input.template_name, &variables)
        .map_err(|e| e.to_string())
}

pub fn prompt_dependency_graph(state: &AppState) -> DependencyGraphOutput {
    let graph = state.engine.dependency_graph();
    DependencyGraphOutput {
        nodes: graph
            .nodes
            .iter()
            .map(|node| {
                serde_json::json!({
                    "name": node.name,
                    "category": node.category,
                })
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| {
                serde_json::json!({
                    "from": edge.from,
                    "to": edge.to,
                })
            })
            .collect(),
    }
}

pub async fn prompt_render_ab(
    state: &AppState,
    user_id: i64,
    input: AbRenderInput,
) -> Result<AbRenderResult, String> {
    if let Some(project_id) = input.project_id {
        access::verify_project_editor(state, user_id, project_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    let variables = template_variables::parse_template_variables(input.variables.as_ref())?;
    let (version_a, version_b) = state
        .engine
        .render_ab(&input.template_name, &variables, input.project_id)
        .map_err(|e| e.to_string())?;
    Ok(AbRenderResult {
        version_a,
        version_b,
    })
}

pub fn prompt_get_dependencies(state: &AppState, name: &str) -> Result<Vec<String>, String> {
    let entry = state.engine.get(name).map_err(|e| e.to_string())?;
    Ok(entry.dependencies)
}

pub fn prompt_get_reverse_dependencies(
    state: &AppState,
    name: &str,
) -> Result<Vec<String>, String> {
    let entries = state.engine.list_all();
    reverse_dependencies_for_entries(&entries, name)
}

pub async fn prompt_list_project_overrides(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let active_prompt_names = active_prompt_names(state.engine.as_ref());
    prompt_list_project_overrides_data(state.db.as_ref(), project_id, &active_prompt_names).await
}

pub fn prompt_list_categories(state: &AppState) -> Result<Vec<CategoryInfo>, String> {
    let registry = state
        .engine
        .registry()
        .ok_or_else(|| "提示词注册表未加载".to_string())?;
    let registry = registry.read();
    let mut categories = Vec::with_capacity(registry.categories().len());
    for (key, value) in registry.categories() {
        categories.push(CategoryInfo {
            key: key.clone(),
            label: value.label.clone(),
            order: value
                .order
                .ok_or_else(|| format!("提示词分类 {key} 缺少 order"))?,
            description: value.description.clone(),
        });
    }
    categories.sort_by_key(|category| (category.order, category.key.clone()));
    Ok(categories)
}

async fn persist_project_override(
    db: &DatabaseConnection,
    project_id: i64,
    name: &str,
    content: &str,
) -> Result<ProjectOverrideIdentity, String> {
    let identity = normalize_project_override_identity(project_id, name)?;
    validate_project_override_content(content)?;
    ensure_project_exists(db, identity.project_id, MissingProjectMessage::Command).await?;

    let key = project_override_key(&identity);
    let now = Utc::now().naive_utc();
    let value = Value::String(content.to_string());
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&key))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(record) = existing {
        let mut active: system_settings::ActiveModel = record.into();
        active.value = Set(value);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())?;
    } else {
        system_settings::ActiveModel {
            key: Set(key),
            value: Set(value),
            category: Set(system_settings::PROMPT_OVERRIDE_CATEGORY.to_string()),
            description: Set(Some(format!(
                "项目 {} 提示词覆盖：{}",
                identity.project_id, identity.name
            ))),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(identity)
}

async fn load_project_override_persisted_content(
    db: &DatabaseConnection,
    identity: &ProjectOverrideIdentity,
) -> Result<Option<String>, String> {
    ensure_project_exists(db, identity.project_id, MissingProjectMessage::Command).await?;
    let key = project_override_key(identity);
    let record = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&key))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(record) = record else {
        return Ok(None);
    };
    let content = record
        .value
        .as_str()
        .ok_or_else(|| format!("提示词覆盖记录 value 必须是字符串: {key}"))?;
    validate_project_override_content(content)?;
    Ok(Some(content.to_string()))
}

async fn delete_project_override_by_identity(
    db: &DatabaseConnection,
    identity: &ProjectOverrideIdentity,
) -> Result<(), String> {
    let key = project_override_key(identity);
    system_settings::Entity::delete_many()
        .filter(system_settings::Column::Key.eq(key))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn delete_project_overrides_for_prompt_names(
    db: &DatabaseConnection,
    prompt_names: &HashSet<String>,
) -> Result<u64, String> {
    if prompt_names.is_empty() {
        return Ok(0);
    }

    let records = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let ids = retired_project_override_record_ids(&records, prompt_names)?;
    if ids.is_empty() {
        return Ok(0);
    }

    let result = system_settings::Entity::delete_many()
        .filter(system_settings::Column::Id.is_in(ids))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.rows_affected)
}

async fn preflight_project_override_keys_for_retired_cleanup(
    db: &DatabaseConnection,
) -> Result<(), String> {
    let records = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    validate_project_override_record_keys(&records)
}

fn validate_project_override_record_keys(records: &[system_settings::Model]) -> Result<(), String> {
    for record in records {
        parse_project_override_key(&record.key)?;
    }
    Ok(())
}

fn retired_project_override_record_ids(
    records: &[system_settings::Model],
    prompt_names: &HashSet<String>,
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for record in records {
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
        if prompt_names.contains(&identity.name) {
            ids.push(record.id);
        }
    }
    Ok(ids)
}

async fn rollback_project_override_runtime_after_persist_error(
    engine: &dyn a4n_engine::prompt::engine::PromptEngine,
    identity: &ProjectOverrideIdentity,
    previous_content: Option<String>,
    operation_error: String,
    rollback_success_message: &str,
    rollback_failure_prefix: &str,
) -> String {
    let rollback_result = match previous_content {
        Some(content) => {
            engine
                .set_project_override(identity.project_id, &identity.name, content)
                .await
        }
        None => {
            engine
                .remove_project_override(identity.project_id, &identity.name)
                .await
        }
    };

    match rollback_result {
        Ok(()) => format!("{operation_error}；{rollback_success_message}"),
        Err(err) => format!("{operation_error}；{rollback_failure_prefix}{err}"),
    }
}

fn to_entry_summary(entry: &PromptEntry) -> PromptEntrySummary {
    PromptEntrySummary {
        name: entry.name.clone(),
        title: entry.title.clone(),
        content: entry.content.clone(),
        description: entry.description.clone(),
        tags: entry.tags.clone(),
        category: entry.category.clone(),
        dependencies: entry.dependencies.clone(),
        is_modified: entry.is_modified,
        version: entry.version.clone(),
        modified_at: entry.modified_at.to_string(),
        used_by: entry.used_by.clone(),
        path: entry.path.clone(),
    }
}

fn to_version_summary(version: &PromptVersion) -> PromptVersionSummary {
    PromptVersionSummary {
        version: version.version.clone(),
        timestamp: version.timestamp.to_string(),
        author: version.author.clone(),
        message: version.message.clone(),
        content_snapshot: version.content_snapshot.clone(),
    }
}

fn active_prompt_names(engine: &dyn a4n_engine::prompt::engine::PromptEngine) -> HashSet<String> {
    engine
        .list_all()
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

fn reverse_dependencies_for_entries(
    entries: &[PromptEntry],
    name: &str,
) -> Result<Vec<String>, String> {
    if !entries.iter().any(|entry| entry.name == name) {
        return Err(format!("提示词不存在：{name}"));
    }

    Ok(entries
        .iter()
        .filter(|entry| {
            entry
                .dependencies
                .iter()
                .any(|dependency| dependency == name)
        })
        .map(|entry| entry.name.clone())
        .collect())
}

async fn prompt_list_project_overrides_data(
    db: &DatabaseConnection,
    project_id: i64,
    active_prompt_names: &HashSet<String>,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    let project_id = validate_project_override_project_id(project_id)?;
    ensure_project_exists(db, project_id, MissingProjectMessage::Command).await?;
    let all_records = system_settings::Entity::find()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    project_override_summaries_from_records(project_id, all_records, active_prompt_names)
}

fn project_override_summaries_from_records(
    project_id: i64,
    records: Vec<system_settings::Model>,
    active_prompt_names: &HashSet<String>,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    let mut overrides = Vec::new();
    for record in records {
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
        if identity.project_id != project_id {
            continue;
        }
        let content = record
            .value
            .as_str()
            .ok_or_else(|| format!("提示词覆盖记录 value 必须是字符串: {}", record.key))?;
        validate_project_override_content(content)?;
        if !active_prompt_names.contains(&identity.name) {
            continue;
        }
        overrides.push(ProjectPromptOverrideSummary {
            project_id,
            prompt_name: identity.name,
            content: record.value,
            updated_at: record.updated_at.to_string(),
        });
    }

    Ok(overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_engine::prompt::engine::PromptEngine;
    use a4n_engine::prompt::tera_engine::TeraPromptEngine;
    use chrono::Utc;
    use std::collections::HashMap;

    fn prompt_entry(name: &str, content: &str, dependencies: Vec<&str>) -> PromptEntry {
        PromptEntry {
            name: name.to_string(),
            title: name.to_string(),
            content: content.to_string(),
            description: Some(format!("{name} description")),
            tags: vec!["test".to_string()],
            category: "writing".to_string(),
            dependencies: dependencies.into_iter().map(ToString::to_string).collect(),
            is_modified: false,
            version: "1.0".to_string(),
            modified_at: Utc::now().naive_utc(),
            used_by: Vec::new(),
            path: Some(format!("{name}.md")),
        }
    }

    fn setting_record(id: i64, key: &str, value: Value) -> system_settings::Model {
        system_settings::Model {
            id,
            key: key.to_string(),
            value,
            category: system_settings::PROMPT_OVERRIDE_CATEGORY.to_string(),
            description: None,
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn parse_template_variables_allows_missing_null_or_object_values() {
        assert!(
            template_variables::parse_template_variables(None)
                .unwrap()
                .is_empty()
        );
        assert!(
            template_variables::parse_template_variables(Some(&serde_json::Value::Null))
                .unwrap()
                .is_empty()
        );
        let variables = template_variables::parse_template_variables(Some(
            &serde_json::json!({"tone": "warm"}),
        ))
        .expect("object variables should be accepted");
        assert_eq!(variables["tone"], serde_json::json!("warm"));
    }

    #[test]
    fn parse_template_variables_rejects_explicit_non_object_values() {
        let err = template_variables::parse_template_variables(Some(&serde_json::json!(["tone"])))
            .expect_err("arrays are not valid variables");
        assert!(err.contains("variables 必须是对象或 null"));
    }

    #[test]
    fn reverse_dependencies_requires_existing_prompt_name() {
        let entries = vec![prompt_entry("outline", "大纲", vec!["blueprint"])];
        let err = reverse_dependencies_for_entries(&entries, "missing")
            .expect_err("unknown prompt should fail");
        assert!(err.contains("提示词不存在：missing"));
    }

    #[test]
    fn reverse_dependencies_returns_dependents_for_existing_prompt_name() {
        let entries = vec![
            prompt_entry("outline", "大纲", vec!["blueprint"]),
            prompt_entry("writing", "正文", vec!["outline"]),
            prompt_entry("blueprint", "蓝图", vec![]),
        ];
        let reverse = reverse_dependencies_for_entries(&entries, "outline").unwrap();
        assert_eq!(reverse, vec!["writing".to_string()]);
    }

    #[test]
    fn project_override_list_filters_records_for_retired_prompt_names() {
        let mut active_prompt_names = HashSet::new();
        active_prompt_names.insert("active_prompt".to_string());
        let records = vec![
            setting_record(
                1,
                &system_settings::prompt_override_key(42, "active_prompt"),
                serde_json::json!("active override"),
            ),
            setting_record(
                2,
                &system_settings::prompt_override_key(42, "retired_prompt"),
                serde_json::json!("retired override"),
            ),
            setting_record(
                3,
                &system_settings::prompt_override_key(7, "active_prompt"),
                serde_json::json!("other project"),
            ),
        ];

        let overrides =
            project_override_summaries_from_records(42, records, &active_prompt_names).unwrap();

        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].prompt_name, "active_prompt");
        assert_eq!(overrides[0].content, serde_json::json!("active override"));
    }

    #[test]
    fn project_override_list_rejects_invalid_override_key() {
        let mut active_prompt_names = HashSet::new();
        active_prompt_names.insert("active_prompt".to_string());
        let records = vec![setting_record(
            1,
            system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
            serde_json::json!("bad"),
        )];

        let err = project_override_summaries_from_records(42, records, &active_prompt_names)
            .expect_err("invalid prompt override key should not be hidden by list filtering");

        assert!(err.contains("提示词覆盖 key 格式无效"));
    }

    #[test]
    fn project_override_list_rejects_non_string_content_before_active_filter() {
        let mut active_prompt_names = HashSet::new();
        active_prompt_names.insert("active_prompt".to_string());
        let records = vec![setting_record(
            1,
            &system_settings::prompt_override_key(42, "retired_prompt"),
            serde_json::json!({"content": "bad"}),
        )];

        let err = project_override_summaries_from_records(42, records, &active_prompt_names)
            .expect_err("invalid persisted override content should be explicit");

        assert!(err.contains("提示词覆盖记录 value 必须是字符串"));
    }

    #[test]
    fn retired_project_override_cleanup_matches_parsed_prompt_name() {
        let mut retired_prompt_names = HashSet::new();
        retired_prompt_names.insert("bar".to_string());
        let records = vec![
            setting_record(
                1,
                &system_settings::prompt_override_key(42, "bar"),
                serde_json::json!("retired"),
            ),
            setting_record(
                2,
                &system_settings::prompt_override_key(42, "foo_bar"),
                serde_json::json!("still active"),
            ),
            setting_record(3, "unrelated_key", serde_json::json!("ignored")),
        ];

        let ids = retired_project_override_record_ids(&records, &retired_prompt_names).unwrap();

        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn retired_project_override_cleanup_rejects_invalid_override_key() {
        let mut retired_prompt_names = HashSet::new();
        retired_prompt_names.insert("bar".to_string());
        let records = vec![setting_record(
            1,
            system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
            serde_json::json!("bad"),
        )];

        let err = retired_project_override_record_ids(&records, &retired_prompt_names)
            .expect_err("invalid prompt override key should be explicit");

        assert!(err.contains("提示词覆盖 key 格式无效"));
    }

    #[test]
    fn project_override_key_preflight_accepts_valid_override_and_unrelated_settings() {
        let records = vec![
            setting_record(
                1,
                &system_settings::prompt_override_key(42, "active_prompt"),
                serde_json::json!("override"),
            ),
            setting_record(2, "llm_config", serde_json::json!({"model": "x"})),
        ];

        validate_project_override_record_keys(&records).unwrap();
    }

    #[test]
    fn project_override_key_preflight_rejects_invalid_override_key_before_reload() {
        let records = vec![setting_record(
            1,
            system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
            serde_json::json!("bad"),
        )];

        let err = validate_project_override_record_keys(&records)
            .expect_err("load_defaults preflight should catch invalid persisted override keys");

        assert!(err.contains("提示词覆盖 key 格式无效"));
    }

    #[tokio::test]
    async fn rollback_project_override_runtime_restores_previous_content() {
        let engine = TeraPromptEngine::with_defaults(vec![prompt_entry(
            "greeting",
            "默认 {{ name }}",
            vec![],
        )])
        .unwrap();
        let identity = ProjectOverrideIdentity {
            project_id: 42,
            name: "greeting".to_string(),
        };
        engine
            .set_project_override(42, "greeting", "旧覆盖 {{ name }}".to_string())
            .await
            .unwrap();
        engine
            .set_project_override(42, "greeting", "新覆盖 {{ name }}".to_string())
            .await
            .unwrap();

        let message = rollback_project_override_runtime_after_persist_error(
            &engine,
            &identity,
            Some("旧覆盖 {{ name }}".to_string()),
            "提示词覆盖持久化失败: db down".to_string(),
            "已回滚运行时覆盖",
            "运行时覆盖已更新但回滚失败：",
        )
        .await;

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), serde_json::json!("小明"));
        assert!(message.contains("已回滚运行时覆盖"));
        assert_eq!(
            engine.render("greeting", &variables, Some(42)).unwrap(),
            "旧覆盖 小明"
        );
    }

    #[tokio::test]
    async fn rollback_project_override_runtime_removes_new_override_without_snapshot() {
        let engine = TeraPromptEngine::with_defaults(vec![prompt_entry(
            "greeting",
            "默认 {{ name }}",
            vec![],
        )])
        .unwrap();
        let identity = ProjectOverrideIdentity {
            project_id: 42,
            name: "greeting".to_string(),
        };
        engine
            .set_project_override(42, "greeting", "新覆盖 {{ name }}".to_string())
            .await
            .unwrap();

        let message = rollback_project_override_runtime_after_persist_error(
            &engine,
            &identity,
            None,
            "提示词覆盖持久化失败: db down".to_string(),
            "已回滚运行时覆盖",
            "运行时覆盖已更新但回滚失败：",
        )
        .await;

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), serde_json::json!("小明"));
        assert!(message.contains("已回滚运行时覆盖"));
        assert_eq!(
            engine.render("greeting", &variables, Some(42)).unwrap(),
            "默认 小明"
        );
    }
}
