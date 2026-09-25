use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::inspiration::InspirationResponse;

/// Start one inspiration conversation turn through the canonical generation path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartInspirationGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 32768))]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InspirationHistoryInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreviewInspirationRequestInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 32768))]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InspirationMessageSummary {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Host-facing inspiration conversation boundary.
///
/// Authorization, history limits, persistence reads, and DTO projection stay
/// with the inspiration owner; Runtime only routes the typed query.
#[async_trait::async_trait]
pub trait InspirationHistoryReadApplicationPort: Send + Sync {
    async fn history(
        &self,
        user_id: i64,
        input: InspirationHistoryInput,
    ) -> Result<Vec<InspirationMessageSummary>, a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn inspiration_inputs_reject_retired_flexibility_fields() {
        serde_json::from_value::<InspirationHistoryInput>(json!({
            "project_id": 7,
            "limit": 80
        }))
        .expect_err("history limit is fixed by the Host contract");
        serde_json::from_value::<PreviewInspirationRequestInput>(json!({
            "project_id": 7,
            "user_input": "继续",
            "variables": null
        }))
        .expect_err("preview must reject retired aliases and variables");
        serde_json::from_value::<StartInspirationGenerationInput>(json!({
            "meta": {
                "start_request_id": "request-1",
                "retry_of_task_id": null
            },
            "project_id": 7,
            "message": "继续",
            "variables": null
        }))
        .expect_err("generation input must reject retired variables");
    }
}
