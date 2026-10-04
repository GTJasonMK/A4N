//! Typed project creation and update commands owned by the project mutation worker.

pub use a4n_api_contract::project::ProjectSummary;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

use serde::{Deserialize, Serialize};

pub const PROJECT_MUTATION_WORKER_ID: &str = "project-mutation-worker";
pub const PROJECT_MUTATION_WORKER_CONTRACT_VERSION: &str = "project-mutation-v1";
pub const PROJECT_MUTATION_CREATE_OPERATION: &str = "project.mutation.create";
pub const PROJECT_MUTATION_CREATE_DERIVED_OPERATION: &str = "project.mutation.create_derived";
pub const PROJECT_MUTATION_CREATE_CANVAS_OPERATION: &str = "project.mutation.create_canvas";
pub const PROJECT_MUTATION_CREATE_STANDALONE_COMIC_OPERATION: &str =
    "project.mutation.create_standalone_comic";
pub const PROJECT_MUTATION_UPDATE_OPERATION: &str = "project.mutation.update";
pub const PROJECT_MUTATION_CAPABILITY: &str = "project.mutation";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectInput {
    pub title: String,
    pub description: Option<String>,
    pub project_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateDerivedProjectInput {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateCanvasProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateStandaloneComicProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectInput {
    pub project_id: i64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub fact_feedback_interval_chapters: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    #[test]
    fn update_scope_is_stable() {
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: PROJECT_MUTATION_UPDATE_OPERATION.into(),
            idempotency_key: "key".into(),
            project_id: Some(9),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: PROJECT_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: UpdateProjectInput {
                project_id: 9,
                title: Some("新标题".into()),
                description: None,
                fact_feedback_interval_chapters: None,
            },
        };
        request.validate().expect("project mutation validates");
    }
}
