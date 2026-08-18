use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApiErrorResponse {
    #[schemars(length(min = 1, max = 128))]
    pub code: String,
    #[schemars(length(min = 1, max = 4096))]
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "crate::serde_update::deserialize_required_nullable_field")]
    #[schemars(
        required,
        schema_with = "crate::serde_update::nullable_trace_id_schema"
    )]
    pub trace_id: Option<String>,
    pub details: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::ApiErrorResponse;
    use serde_json::json;

    #[test]
    fn error_contract_requires_nullable_trace_id_and_details_fields() {
        assert!(
            serde_json::from_value::<ApiErrorResponse>(json!({
                "code": "invalid_request",
                "message": "failed",
                "retryable": false,
                "details": {}
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ApiErrorResponse>(json!({
                "code": "invalid_request",
                "message": "failed",
                "retryable": false,
                "trace_id": null,
                "details": {}
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<ApiErrorResponse>(json!({
                "code": "invalid_request",
                "message": "failed",
                "retryable": false,
                "trace_id": null,
                "details": []
            }))
            .is_err()
        );
    }
}
