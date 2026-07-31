use a4n_core::blueprint::increment_blueprint_version;
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
    BlueprintRestoreResult, blueprint_restore_error_to_app, project_mutation_error_to_app_error,
    required_text,
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
    ) -> Result<BlueprintRestoreResult, AppError> {
        a4n_db::project::restore_blueprint_version_record(self.connection(), project_id, version)
            .await
            .map_err(blueprint_restore_error_to_app)
    }

    async fn update_world_setting(
        &self,
        project_id: i64,
        world_setting: Value,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        a4n_db::project::update_blueprint_world_setting(
            self.connection(),
            project_id,
            world_setting,
        )
        .await
        .map_err(project_mutation_error_to_app_error)
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

    async fn blueprint_version_exists(
        &self,
        project_id: i64,
        version: &str,
    ) -> Result<bool, AppError> {
        Ok(a4n_db::project::blueprint_version_by_project_version(
            self.connection(),
            project_id,
            version,
        )
        .await
        .map_err(project_mutation_error_to_app_error)?
        .is_some())
    }
}

impl OrchestratorContext {
    pub async fn novel_blueprint_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        self.novel_blueprint_for_project(project_id).await
    }

    pub async fn novel_blueprint_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        self.project_blueprint.current(project_id).await
    }

    pub async fn blueprint_versions_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<blueprint_version::Model>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        self.blueprint_versions_by_project(project_id).await
    }

    pub async fn blueprint_versions_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_version::Model>, AppError> {
        self.project_blueprint.versions(project_id).await
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
    ) -> Result<BlueprintRestoreResult, AppError> {
        let result = self.project_blueprint.restore(project_id, version).await?;
        // 事件在 DB 成功后由 context 发布，runtime 不再旁路 orchestrator 发事件。
        if let Some(status_change) = &result.status_change {
            self.publish_internal_event(InternalEvent::ProjectStatusChanged {
                project_id,
                from: status_change.from.clone(),
                to: status_change.to.clone(),
                timestamp: status_change.changed_at,
            })?;
        }
        self.publish_internal_event(InternalEvent::BlueprintUpdated {
            project_id,
            version: result.version.clone(),
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
        let left = self
            .blueprint_version_by_project_version(project_id, version_a)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("版本 {version_a} 不存在")))?;
        let right = self
            .blueprint_version_by_project_version(project_id, version_b)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("版本 {version_b} 不存在")))?;
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
    ) -> Result<BlueprintRestoreResult, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let version = required_text(version, "版本号").map_err(AppError::InvalidArgument)?;
        self.restore_blueprint_version_record(project_id, version)
            .await
    }

    pub async fn update_blueprint_world_setting_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        world_setting: Value,
    ) -> Result<Value, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let world_setting = required_world_setting_value(world_setting)?;
        self.project_blueprint
            .update_world_setting(project_id, world_setting.clone())
            .await?
            .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
        Ok(world_setting)
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

    pub(crate) async fn next_blueprint_version(
        &self,
        project_id: i64,
        current_version: Option<&str>,
    ) -> Result<String, AppError> {
        let mut candidate = match current_version {
            Some(current_version) => increment_blueprint_version(current_version)?,
            None => "1.0".to_string(),
        };
        loop {
            if !self
                .project_blueprint
                .blueprint_version_exists(project_id, &candidate)
                .await?
            {
                return Ok(candidate);
            }
            candidate = increment_blueprint_version(&candidate)?;
        }
    }
}

fn required_world_setting_value(value: Value) -> Result<Value, AppError> {
    if value.is_null() {
        return Err(AppError::InvalidArgument(
            "world_setting 不能为 null".to_string(),
        ));
    }
    Ok(value)
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
mod world_setting_tests {
    use super::required_world_setting_value;

    #[test]
    fn rejects_null_world_setting() {
        let error = required_world_setting_value(serde_json::Value::Null)
            .expect_err("null world setting should be rejected");
        assert!(error.to_string().contains("world_setting 不能为 null"));
    }
}

#[cfg(test)]
mod version_diff_tests {
    use super::blueprint_json_diff;
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
}
