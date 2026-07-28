use a4n_service_protocol::JS_SAFE_INTEGER_U64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Host-to-relay request for a temporary provider input.
///
/// `expires_in_seconds` is an upper bound requested by the caller. The relay
/// may apply a shorter server-side policy TTL, but must never exceed this
/// value.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TempMediaUploadRequest {
    #[schemars(range(min = 1))]
    pub owner_user_id: i64,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_image_id: i64,
    #[serde(default)]
    pub source_media_kind: Option<String>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub source_media_id: Option<i64>,
    #[schemars(range(min = 1, max = 86_400))]
    pub expires_in_seconds: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

/// Opaque temporary artifact returned by the relay.
///
/// The URL is a transport convenience for the provider input path. The
/// artifact id and ownership fields remain the audit identity; callers must
/// not infer a local filesystem path from this response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TempMediaUploadResponse {
    #[schemars(length(min = 1, max = 128))]
    pub artifact_id: String,
    #[schemars(length(min = 1, max = 2_048))]
    pub url: String,
    #[schemars(length(min = 1, max = 64))]
    pub expires_at: String,
    #[schemars(range(min = 1))]
    pub owner_user_id: i64,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_image_id: i64,
    #[serde(default)]
    pub source_media_kind: Option<String>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub source_media_id: Option<i64>,
    #[schemars(length(min = 1, max = 128))]
    pub content_type: String,
    #[schemars(length(equal = 64))]
    pub sha256: String,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_U64))]
    pub byte_size: u64,
}

/// Authenticated request to remove one opaque temporary artifact.
///
/// The artifact id stays in the URL path; the hash is sent separately so the
/// relay can refuse deleting a different file after an ownership/identity bug.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TempMediaDeleteRequest {
    #[schemars(length(equal = 64))]
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_media_contract_round_trips_owner_and_artifact_metadata() {
        let request = TempMediaUploadRequest {
            owner_user_id: 7,
            project_id: 11,
            source_image_id: 13,
            source_media_kind: None,
            source_media_id: None,
            expires_in_seconds: 86_400,
            file_name: "reference.png".into(),
            mime_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["owner_user_id"], 7);
        assert_eq!(value["expires_in_seconds"], 86_400);
        assert!(serde_json::from_value::<TempMediaUploadRequest>(value).is_ok());

        let response = TempMediaUploadResponse {
            artifact_id: "artifact-1".into(),
            url: "https://media.example.test/artifact-1.png".into(),
            expires_at: "2026-07-27T00:00:00Z".into(),
            owner_user_id: 7,
            project_id: 11,
            source_image_id: 13,
            source_media_kind: None,
            source_media_id: None,
            content_type: "image/png".into(),
            sha256: "a".repeat(64),
            byte_size: 8,
        };
        assert!(
            serde_json::from_value::<TempMediaUploadResponse>(
                serde_json::to_value(response).unwrap()
            )
            .is_ok()
        );
    }

    #[test]
    fn temp_media_contract_rejects_unknown_request_fields() {
        let value = serde_json::json!({
            "owner_user_id": 7,
            "project_id": 11,
            "source_image_id": 13,
            "expires_in_seconds": 3600,
            "file_name": "reference.png",
            "mime_type": "image/png",
            "data_base64": "iVBORw0KGgo=",
            "path": "/tmp/reference.png"
        });
        assert!(serde_json::from_value::<TempMediaUploadRequest>(value).is_err());
    }

    #[test]
    fn temp_media_delete_contract_only_carries_content_identity() {
        let value = serde_json::json!({"sha256": "a".repeat(64)});
        assert!(serde_json::from_value::<TempMediaDeleteRequest>(value).is_ok());
        assert!(
            serde_json::from_value::<TempMediaDeleteRequest>(serde_json::json!({
                "sha256": "a".repeat(64),
                "path": "/tmp/file"
            }))
            .is_err()
        );
    }
}
