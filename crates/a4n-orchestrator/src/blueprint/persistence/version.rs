use a4n_core::blueprint::increment_blueprint_version;
use a4n_core::error::AppError;
use a4n_core::event::bridge::{EventBus, InternalEvent};
use a4n_db::blueprint::{InsertBlueprintVersionInput, insert_blueprint_version_record};
use chrono::Utc;
use serde_json::Value;

use crate::blueprint::{OrchestratorContext, blueprint_db_error, project_mutation_error};

impl OrchestratorContext {
    pub(in crate::blueprint) async fn save_blueprint_version(
        &self,
        project_id: i64,
        version: String,
        description: Option<String>,
        data: &Value,
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        insert_blueprint_version_record(
            self.db.connection(),
            InsertBlueprintVersionInput {
                project_id,
                version: version.clone(),
                change_description: description,
                blueprint_data: data.clone(),
                now,
                created_by: Some("system".to_string()),
            },
        )
        .await
        .map_err(blueprint_db_error)?;
        self.event_bus
            .publish(InternalEvent::BlueprintUpdated {
                project_id,
                version,
            })
            .await?;
        Ok(())
    }

    pub(in crate::blueprint) async fn next_blueprint_version(
        &self,
        project_id: i64,
        current_version: Option<&str>,
    ) -> Result<String, AppError> {
        let mut candidate = match current_version {
            Some(current_version) => increment_blueprint_version(current_version)?,
            None => "1.0".to_string(),
        };
        loop {
            let exists = a4n_db::project::blueprint_version_by_project_version(
                self.db.connection(),
                project_id,
                &candidate,
            )
            .await
            .map_err(project_mutation_error)?
            .is_some();
            if !exists {
                return Ok(candidate);
            }
            candidate = increment_blueprint_version(&candidate)?;
        }
    }
}
