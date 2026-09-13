pub use a4n_domain::character::{ForeshadowingRecordInput, ForeshadowingStatus};
use a4n_domain::{
    DownstreamConflictStrategy,
    profile_material::{
        ProfileMaterialFilter, ProfileMaterialImageStaleness, ProfileMaterialKind,
        ProfileMaterialKindFilter, ProfileMaterialSort, ProfileMaterialState,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListObjectProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    #[serde(default)]
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateObjectProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub profile: ObjectProfileInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateObjectProfileInput {
    #[schemars(range(min = 1))]
    pub object_profile_id: i64,
    pub profile: ObjectProfileInput,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteObjectProfileInput {
    #[schemars(range(min = 1))]
    pub object_profile_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ObjectProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub ownership_kind: String,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ObjectProfileMutationResult {
    pub profile: ObjectProfileSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ObjectProfileDeleteResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListSceneProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SyncSceneProfilesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileInput {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub scene_type: String,
    pub importance: String,
    #[serde(default)]
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    #[serde(default)]
    pub entry_points_json: Option<Value>,
    #[serde(default)]
    pub rules_json: Option<Value>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateSceneProfileInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub profile: SceneProfileInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateSceneProfileInput {
    #[schemars(range(min = 1))]
    pub scene_profile_id: i64,
    pub profile: SceneProfileInput,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteSceneProfileInput {
    #[schemars(range(min = 1))]
    pub scene_profile_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SceneProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub ownership_kind: String,
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points_json: Value,
    pub rules_json: Value,
    pub current_state_json: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
    pub occurrence_count: u64,
    pub generated_image_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SceneProfileMutationResult {
    pub profile: SceneProfileSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SceneProfileDeleteResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Internal application outcome. The image ids are passed to the media owner
/// for post-commit cleanup and are intentionally not part of the wire result.
#[derive(Debug)]
pub struct ObjectProfileMutationOutcome {
    pub result: ObjectProfileMutationResult,
    pub downstream_image_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct ObjectProfileDeleteOutcome {
    pub result: ObjectProfileDeleteResult,
    pub downstream_image_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct SceneProfileMutationOutcome {
    pub result: SceneProfileMutationResult,
    pub downstream_image_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct SceneProfileDeleteOutcome {
    pub result: SceneProfileDeleteResult,
    pub project_id: i64,
    pub downstream_image_ids: Vec<i64>,
}

/// Host-facing object profile mutation boundary.
///
/// Authorization, conflict policy, persistence and DTO projection stay with
/// the object profile owner. Runtime only executes media cleanup returned by
/// the committed mutation.
#[async_trait::async_trait]
pub trait ObjectProfileMutationApplicationPort: Send + Sync {
    async fn create(
        &self,
        user_id: i64,
        input: CreateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        input: UpdateObjectProfileInput,
    ) -> Result<ObjectProfileMutationOutcome, a4n_error::AppError>;

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteObjectProfileInput,
    ) -> Result<ObjectProfileDeleteOutcome, a4n_error::AppError>;
}

/// Host-facing scene profile mutation boundary.
#[async_trait::async_trait]
pub trait SceneProfileMutationApplicationPort: Send + Sync {
    async fn create(
        &self,
        user_id: i64,
        input: CreateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        input: UpdateSceneProfileInput,
    ) -> Result<SceneProfileMutationOutcome, a4n_error::AppError>;

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteSceneProfileInput,
    ) -> Result<SceneProfileDeleteOutcome, a4n_error::AppError>;
}

/// Host-facing object profile read boundary.
///
/// Object persistence, project visibility and material-count projection stay
/// with the asset owner; Runtime only routes the typed read.
#[async_trait::async_trait]
pub trait ObjectProfileReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileSummary>, a4n_error::AppError>;
}

/// Host-facing scene profile read boundary.
///
/// Scene persistence, project visibility and occurrence/material projections
/// stay with the scene owner; Runtime only routes the typed read.
#[async_trait::async_trait]
pub trait SceneProfileReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SceneProfileSummary>, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListProfileMaterialsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub kind: ProfileMaterialKindFilter,
    pub query: Option<String>,
    pub filter: ProfileMaterialFilter,
    pub sort: ProfileMaterialSort,
    #[schemars(range(min = 1, max = 32))]
    pub image_limit_per_profile: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetProfilePrimaryMaterialInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub profile_kind: ProfileMaterialKind,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub image_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UploadProfileMaterialImageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub profile_kind: ProfileMaterialKind,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub set_primary: bool,
    pub prompt_snapshot: String,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProfileMaterialProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub category: String,
    pub importance: String,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub primary_text: String,
    pub secondary_text: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProfileMaterialImageSummary {
    pub image: crate::image::GeneratedImageSummary,
    pub is_primary: bool,
    pub staleness: ProfileMaterialImageStaleness,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ProfileMaterialGroup {
    pub kind: ProfileMaterialKind,
    pub profile: ProfileMaterialProfileSummary,
    pub material_state: ProfileMaterialState,
    pub current_profile_content_sha256: String,
    pub image_count: usize,
    pub primary_image_id: Option<i64>,
    pub primary_image: Option<ProfileMaterialImageSummary>,
    pub images: Vec<ProfileMaterialImageSummary>,
}

/// Host-facing profile-material read boundary.
///
/// Authorization, persistence and group projection stay with the media
/// owner; Runtime only routes the stable API contract.
#[async_trait::async_trait]
pub trait ProfileMaterialReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, a4n_error::AppError>;
}

/// Host-facing profile-material mutation boundary.
#[async_trait::async_trait]
pub trait ProfileMaterialMutationApplicationPort: Send + Sync {
    async fn set_primary(
        &self,
        user_id: i64,
        input: SetProfilePrimaryMaterialInput,
    ) -> Result<ProfileMaterialGroup, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListForeshadowingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateForeshadowingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub record: ForeshadowingRecordInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateForeshadowingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub foreshadowing_id: i64,
    pub expected_updated_at: String,
    pub record: ForeshadowingRecordInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteForeshadowingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub foreshadowing_id: i64,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ForeshadowingSummary {
    pub id: i64,
    pub project_id: i64,
    pub description: String,
    pub setup_chapter: i32,
    pub payoff_chapter: Option<i32>,
    pub status: ForeshadowingStatus,
    pub related_characters: Vec<String>,
    pub importance: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ForeshadowingWriteResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Host-facing foreshadowing read boundary.
///
/// Authorization, persistence, RAG synchronization and API projection stay
/// with the foreshadowing owner; Runtime only routes the typed operations.
#[async_trait::async_trait]
pub trait ForeshadowingReadApplicationPort: Send + Sync {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ForeshadowingSummary>, a4n_error::AppError>;
}

/// Host-facing foreshadowing mutation boundary.
#[async_trait::async_trait]
pub trait ForeshadowingMutationApplicationPort: Send + Sync {
    async fn create(
        &self,
        user_id: i64,
        input: CreateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        input: UpdateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, a4n_error::AppError>;

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use super::{
        CreateForeshadowingInput, CreateObjectProfileInput, ListProfileMaterialsInput,
        SyncSceneProfilesInput, UpdateSceneProfileInput, UploadProfileMaterialImageInput,
    };
    use serde_json::json;

    #[test]
    fn profile_contracts_reject_legacy_split_fields_and_defaults() {
        assert!(
            serde_json::from_value::<CreateObjectProfileInput>(json!({
                "project_id": 1,
                "profile": {
                    "name": "钥匙",
                    "object_type": "prop",
                    "importance": "primary",
                    "visual_description": "铜制",
                    "material_and_state": "旧",
                    "story_usage": "开门",
                    "visual_usage": "特写",
                    "generation_usage": "素材",
                    "first_seen_chapter": 1,
                    "last_seen_chapter": 1,
                    "image_prompt_seed": "钥匙",
                    "downstream_strategy": "reject"
                }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateSceneProfileInput>(json!({
                "scene_profile_id": 1,
                "profile": {
                    "name": "大厅",
                    "scene_type": "room",
                    "importance": "primary",
                    "base_description": "大厅",
                    "visual_description": "宽阔",
                    "spatial_layout": "方形",
                    "atmosphere": "安静",
                    "first_seen_chapter": 1,
                    "last_seen_chapter": 1,
                    "image_prompt_seed": "大厅"
                }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ListProfileMaterialsInput>(json!({
                "project_id": 1,
                "kind": null,
                "query": null,
                "filter": null,
                "sort": null
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UploadProfileMaterialImageInput>(json!({
                "project_id": 1,
                "profile_kind": "scene",
                "profile_id": 1,
                "file_name": "scene.png",
                "mime_type": "image/png",
                "data_base64": "AA==",
                "set_primary": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SyncSceneProfilesInput>(json!({
                "projectId": 1,
                "chapterNumber": 2,
                "userId": 9,
                "taskId": "legacy-task"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CreateForeshadowingInput>(json!({
                "project_id": 1,
                "record": {
                    "description": "门后的脚步声",
                    "setup_chapter": 2,
                    "payoff_chapter": null,
                    "status": "legacy_pending",
                    "related_characters": [],
                    "importance": 4
                }
            }))
            .is_err()
        );
    }
}
