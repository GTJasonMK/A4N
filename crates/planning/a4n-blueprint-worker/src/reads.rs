use super::*;
use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary,
};
use a4n_db_models::models::project::{blueprint_version, novel_blueprint};
use serde_json::{Map, Value};

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
) -> Result<BlueprintVersionSummary, WorkerFailure> {
    Ok(BlueprintVersionSummary {
        id: version.id,
        version: version.version,
        change_description: persisted_version_text(
            version.change_description.as_deref(),
            version.id,
            "change_description",
        )?,
        created_at: version.created_at.and_utc().to_rfc3339(),
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
) -> Result<String, WorkerFailure> {
    let value = value.ok_or_else(|| {
        WorkerFailure::conflict(format!("blueprint_version {version_id} 缺少 {field}"))
    })?;
    if value.is_empty() || value.trim() != value {
        return Err(WorkerFailure::conflict(format!(
            "blueprint_version {version_id} 的 {field} 非规范"
        )));
    }
    Ok(value.to_string())
}

pub(super) fn blueprint_json_diff(left: &Value, right: &Value) -> Value {
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

impl WorkerState {
    pub(super) async fn summary(
        &self,
        request: WorkerEnvelope<BlueprintProjectInput>,
    ) -> Result<Option<BlueprintSummary>, WorkerFailure> {
        validate_request(&request, BLUEPRINT_SUMMARY_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let Some(blueprint) = a4n_db::project::novel_blueprint_for_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(WorkerFailure::database)?
        else {
            return Ok(None);
        };
        let needs_review = a4n_db::project::blueprint_sources_need_review(
            self.database.connection(),
            blueprint.project_id,
            blueprint.updated_at,
        )
        .await
        .map_err(WorkerFailure::database)?;
        Ok(Some(blueprint_summary(blueprint, needs_review)))
    }

    pub(super) async fn versions(
        &self,
        request: WorkerEnvelope<ListBlueprintVersionsInput>,
    ) -> Result<Vec<BlueprintVersionSummary>, WorkerFailure> {
        validate_request(&request, BLUEPRINT_VERSIONS_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::project::blueprint_versions_by_project(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(project_mutation_error)?
        .into_iter()
        .map(blueprint_version_summary)
        .collect()
    }

    pub(super) async fn diff(
        &self,
        request: WorkerEnvelope<DiffBlueprintVersionsInput>,
    ) -> Result<BlueprintDiffSummary, WorkerFailure> {
        validate_request(&request, BLUEPRINT_DIFF_OPERATION)?;
        validate_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        self.verify_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let left = a4n_db::project::blueprint_version_by_project_version(
            self.database.connection(),
            request.payload.project_id,
            &request.payload.version_a,
        )
        .await
        .map_err(project_mutation_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("版本 {} 不存在", request.payload.version_a))
        })?;
        let right = a4n_db::project::blueprint_version_by_project_version(
            self.database.connection(),
            request.payload.project_id,
            &request.payload.version_b,
        )
        .await
        .map_err(project_mutation_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("版本 {} 不存在", request.payload.version_b))
        })?;
        Ok(BlueprintDiffSummary {
            diff: blueprint_json_diff(&left.blueprint_data, &right.blueprint_data),
        })
    }
}
