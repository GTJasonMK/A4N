use a4n_api_contract::blueprint::{BlueprintSummary, BlueprintVersionSummary, SaveBlueprintInput};
use a4n_core::event::bridge::InternalEvent;
use a4n_core::{
    error::AppError,
    models::project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
    },
};
use a4n_db::DbHandle;
use serde_json::{Map, Value};

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::project::{
    blueprint_restore_error_to_app, project_mutation_error_to_app_error, required_text,
};

/// 项目蓝图读模型与生成/精炼写路径 owner。
#[derive(Clone)]
pub(super) struct ProjectBlueprintApplication {
    database: DbHandle,
}

impl ProjectBlueprintApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    // --- 读模型（runtime records） ---

    async fn current(&self, project_id: i64) -> Result<Option<novel_blueprint::Model>, AppError> {
        a4n_db::project::novel_blueprint_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn sources_need_review(
        &self,
        blueprint: &novel_blueprint::Model,
    ) -> Result<bool, AppError> {
        a4n_db::project::blueprint_sources_need_review(
            self.connection(),
            blueprint.project_id,
            blueprint.updated_at,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn versions(&self, project_id: i64) -> Result<Vec<blueprint_version::Model>, AppError> {
        a4n_db::project::blueprint_versions_by_project(self.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)
    }

    async fn version(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<Option<blueprint_version::Model>, AppError> {
        a4n_db::project::blueprint_version_by_project_version(
            self.connection(),
            project_id,
            version,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
    }

    async fn restore(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        a4n_db::project::restore_blueprint_version_record(self.connection(), project_id, version)
            .await
            .map_err(blueprint_restore_error_to_app)
    }

    // --- 生成/精炼读 ---

    async fn blueprint_characters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::project::blueprint_characters_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn blueprint_relationships_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        a4n_db::project::blueprint_relationships_for_project(self.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}

impl OrchestratorContext {
    pub async fn blueprint_summary_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<BlueprintSummary>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        let Some(blueprint) = self.project_blueprint.current(project_id).await? else {
            return Ok(None);
        };
        let needs_review = self
            .project_blueprint
            .sources_need_review(&blueprint)
            .await?;
        Ok(Some(blueprint_summary(blueprint, needs_review)))
    }

    pub async fn novel_blueprint_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        self.project_blueprint.current(project_id).await
    }

    pub async fn blueprint_version_summaries_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<BlueprintVersionSummary>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        self.project_blueprint
            .versions(project_id)
            .await?
            .into_iter()
            .map(blueprint_version_summary)
            .collect()
    }

    pub(crate) async fn blueprint_version_by_project_version(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<Option<blueprint_version::Model>, AppError> {
        self.project_blueprint.version(project_id, version).await
    }

    pub(crate) async fn restore_blueprint_version_record(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        let result = self.project_blueprint.restore(project_id, version).await?;
        // 事件在 DB 成功后由 context 发布，runtime 不再旁路 orchestrator 发事件。
        if let Some(status_change) = &result.status_change {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id,
                from: status_change.from,
                to: status_change.to,
                timestamp: status_change.changed_at,
            })?;
        }
        self.publish_internal_event(InternalEvent::BlueprintUpdated {
            project_id,
            version: result.blueprint.current_version.clone(),
        })?;
        Ok(result)
    }

    pub async fn diff_blueprint_versions_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        version_a: &str,
        version_b: &str,
    ) -> Result<Value, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let version_a = required_text(version_a, "版本 A").map_err(AppError::InvalidArgument)?;
        let version_b = required_text(version_b, "版本 B").map_err(AppError::InvalidArgument)?;
        let (left_result, right_result) = futures::join!(
            self.blueprint_version_by_project_version(project_id, version_a),
            self.blueprint_version_by_project_version(project_id, version_b),
        );
        let left =
            left_result?.ok_or_else(|| AppError::NotFound(format!("版本 {version_a} 不存在")))?;
        let right =
            right_result?.ok_or_else(|| AppError::NotFound(format!("版本 {version_b} 不存在")))?;
        Ok(blueprint_json_diff(
            &left.blueprint_data,
            &right.blueprint_data,
        ))
    }

    pub async fn restore_blueprint_version_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        version: &str,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.restore_blueprint_version_record(project_id, version)
            .await
    }

    pub async fn update_blueprint_world_setting_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        world_setting: Value,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let result = a4n_db::blueprint::update_blueprint_world_setting_records_transaction(
            self.project_blueprint.connection(),
            a4n_db::blueprint::UpdateBlueprintWorldSettingInput {
                project_id,
                world_setting,
                created_by: format!("user:{user_id}"),
            },
        )
        .await
        .map_err(crate::blueprint::blueprint_db_error)?;
        self.publish_versioned_blueprint_events(&result)?;
        Ok(result)
    }

    pub async fn save_blueprint_for_user_validated(
        &self,
        user_id: i64,
        input: SaveBlueprintInput,
    ) -> Result<a4n_db::blueprint::VersionedBlueprintMutationResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let result = a4n_db::blueprint::save_manual_blueprint_records_transaction(
            self.project_blueprint.connection(),
            a4n_db::blueprint::SaveManualBlueprintInput {
                project_id: input.project_id,
                genre: input.genre,
                style: input.style,
                tone: input.tone,
                target_audience: input.target_audience,
                one_sentence_summary: input.one_sentence_summary,
                full_synopsis: input.full_synopsis,
                total_chapters: input.total_chapters,
                created_by: format!("user:{user_id}"),
            },
        )
        .await
        .map_err(crate::blueprint::blueprint_db_error)?;
        self.publish_versioned_blueprint_events(&result)?;
        Ok(result)
    }

    // --- 生成/精炼：编排层委托 ---

    pub(crate) async fn load_blueprint(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        self.project_blueprint.current(project_id).await
    }

    pub(crate) async fn load_blueprint_characters(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        self.project_blueprint
            .blueprint_characters_for_project(project_id)
            .await
    }

    pub(crate) async fn load_blueprint_relationships(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        self.project_blueprint
            .blueprint_relationships_for_project(project_id)
            .await
    }

    fn publish_versioned_blueprint_events(
        &self,
        result: &a4n_db::blueprint::VersionedBlueprintMutationResult,
    ) -> Result<(), AppError> {
        if let Some(status_change) = result.status_change {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id: result.blueprint.project_id,
                from: status_change.from,
                to: status_change.to,
                timestamp: status_change.changed_at,
            })?;
        }
        self.publish_internal_event(InternalEvent::BlueprintUpdated {
            project_id: result.blueprint.project_id,
            version: result.blueprint.current_version.clone(),
        })?;
        Ok(())
    }
}

fn blueprint_summary(
    blueprint: novel_blueprint::Model,
    sources_need_review: bool,
) -> BlueprintSummary {
    BlueprintSummary {
        id: blueprint.id,
        project_id: blueprint.project_id,
        title: blueprint.title,
        genre: blueprint.genre,
        style: blueprint.style,
        tone: blueprint.tone,
        target_audience: blueprint.target_audience,
        one_sentence_summary: blueprint.one_sentence_summary,
        full_synopsis: blueprint.full_synopsis,
        world_setting: blueprint.world_setting,
        current_version: blueprint.current_version,
        total_chapters: blueprint.total_chapters,
        chapters_per_part: blueprint.chapters_per_part,
        source_review_status: if sources_need_review {
            "needs_review"
        } else {
            "not_required"
        }
        .to_string(),
    }
}

fn blueprint_version_summary(
    version: blueprint_version::Model,
) -> Result<BlueprintVersionSummary, AppError> {
    Ok(BlueprintVersionSummary {
        id: version.id,
        version: version.version,
        change_description: persisted_version_text(
            version.change_description.as_deref(),
            version.id,
            "change_description",
        )?,
        created_at: version.created_at.to_string(),
        created_by: persisted_version_text(
            version.created_by.as_deref(),
            version.id,
            "created_by",
        )?,
    })
}

fn persisted_version_text(
    value: Option<&str>,
    version_id: i64,
    field: &str,
) -> Result<String, AppError> {
    let value = value.ok_or_else(|| AppError::InvalidState {
        current: format!("blueprint_version {version_id} 缺少 {field}"),
        operation: "读取蓝图版本".to_string(),
    })?;
    if value.is_empty() || value.trim() != value {
        return Err(AppError::InvalidState {
            current: format!("blueprint_version {version_id} 的 {field} 非规范"),
            operation: "读取蓝图版本".to_string(),
        });
    }
    Ok(value.to_string())
}

fn blueprint_json_diff(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let mut changes = Map::new();
            for (key, right_value) in right {
                match left.get(key) {
                    Some(left_value) if left_value != right_value => {
                        changes.insert(key.clone(), blueprint_json_diff(left_value, right_value));
                    }
                    None => {
                        changes.insert(key.clone(), serde_json::json!({"added": right_value}));
                    }
                    _ => {}
                }
            }
            for (key, left_value) in left {
                if !right.contains_key(key) {
                    changes.insert(key.clone(), serde_json::json!({"removed": left_value}));
                }
            }
            Value::Object(changes)
        }
        _ if left != right => serde_json::json!({"old": left, "new": right}),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod version_diff_tests {
    use a4n_core::models::project::blueprint_version;
    use chrono::NaiveDate;

    use super::{blueprint_json_diff, blueprint_version_summary};
    use serde_json::json;

    #[test]
    fn reports_nested_changes_additions_and_removals() {
        let diff = blueprint_json_diff(
            &json!({"world":{"tone":"old","removed":true}}),
            &json!({"world":{"tone":"new","added":1}}),
        );

        assert_eq!(
            diff,
            json!({
                "world": {
                    "tone": {"old":"old","new":"new"},
                    "added": {"added":1},
                    "removed": {"removed":true}
                }
            })
        );
    }

    #[test]
    fn blueprint_version_summary_rejects_missing_or_noncanonical_metadata() {
        let version = blueprint_version::Model {
            id: 7,
            project_id: 3,
            version: "1.0".to_string(),
            change_description: Some("自动生成首版蓝图".to_string()),
            blueprint_data: json!({}),
            created_at: NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            created_by: Some("system".to_string()),
        };
        let summary = blueprint_version_summary(version.clone()).unwrap();
        assert_eq!(summary.change_description, "自动生成首版蓝图");
        assert_eq!(summary.created_by, "system");

        for (change_description, created_by, expected) in [
            (None, version.created_by.clone(), "缺少 change_description"),
            (
                Some(" 自动生成首版蓝图 ".to_string()),
                version.created_by.clone(),
                "change_description 非规范",
            ),
            (version.change_description.clone(), None, "缺少 created_by"),
            (
                version.change_description.clone(),
                Some(" system ".to_string()),
                "created_by 非规范",
            ),
        ] {
            let mut invalid = version.clone();
            invalid.change_description = change_description;
            invalid.created_by = created_by;
            assert!(
                blueprint_version_summary(invalid)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains(expected)
            );
        }
    }
}
