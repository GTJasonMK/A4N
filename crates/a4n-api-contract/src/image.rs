use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GeneratedImageSummary {
    pub id: i64,
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    #[schemars(extend("x-a4n-required" = true))]
    pub file_size: Option<i64>,
    #[schemars(extend("x-a4n-required" = true))]
    pub ai_score: Option<f32>,
    pub is_favorite: bool,
    pub is_system_managed: bool,
    #[schemars(extend("x-a4n-required" = true))]
    pub chapter_id: Option<i64>,
    #[schemars(extend("x-a4n-required" = true))]
    pub panel_id: Option<String>,
    #[schemars(extend("x-a4n-required" = true))]
    pub metadata_json: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListImagesByProjectInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListImagesByChapterInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListVideoReferenceImagesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(length(min = 1))]
    pub image_project_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedImageInput {
    #[schemars(range(min = 1))]
    pub image_id: i64,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub image_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<i64>")]
    pub chapter_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<String>")]
    pub panel_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<f32>")]
    pub ai_score: Option<Option<f32>>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "bool")]
    pub is_favorite: Option<bool>,
}

impl UpdateGeneratedImageInput {
    pub fn is_empty(&self) -> bool {
        self.prompt.is_none()
            && self.image_type.is_none()
            && self.chapter_id.is_none()
            && self.panel_id.is_none()
            && self.ai_score.is_none()
            && self.is_favorite.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeleteImageInput {
    #[schemars(range(min = 1))]
    pub image_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UploadImageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub image_type: String,
    pub prompt: String,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Map<String, Value>>,
}

#[cfg(test)]
mod tests {
    use super::{
        GeneratedImageSummary, ListVideoReferenceImagesInput, UpdateGeneratedImageInput,
        UploadImageInput,
    };
    use serde_json::json;

    #[test]
    fn generated_image_update_has_one_strict_patch_contract() {
        let input: UpdateGeneratedImageInput = serde_json::from_value(json!({
            "image_id": 1,
            "chapter_id": null,
            "panel_id": "panel-2",
            "ai_score": 0.8,
            "is_favorite": true
        }))
        .expect("valid patch");
        assert_eq!(input.chapter_id, Some(None));
        assert_eq!(input.panel_id, Some(Some("panel-2".to_string())));
        assert_eq!(input.ai_score, Some(Some(0.8)));
        assert_eq!(input.is_favorite, Some(true));
        assert!(!input.is_empty());

        for value in [
            json!({"image_id": 1, "prompt": null}),
            json!({"image_id": 1, "image_type": null}),
            json!({"image_id": 1, "is_favorite": null}),
            json!({"image_id": 1, "clear_chapter_id": true}),
            json!({"image_id": 1, "clear_panel_id": true}),
            json!({"image_id": 1, "clear_ai_score": true}),
        ] {
            assert!(serde_json::from_value::<UpdateGeneratedImageInput>(value).is_err());
        }
        let empty: UpdateGeneratedImageInput =
            serde_json::from_value(json!({"image_id": 1})).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn uploaded_image_metadata_must_be_an_object_and_model_is_not_caller_controlled() {
        let input: UploadImageInput = serde_json::from_value(json!({
            "project_id": 1,
            "file_name": "reference.png",
            "mime_type": "image/png",
            "data_base64": "AA==",
            "image_type": "project_reference",
            "prompt": "reference",
            "metadata_json": {"manual_upload": true}
        }))
        .expect("valid upload input");
        assert_eq!(input.metadata_json.unwrap()["manual_upload"], true);
        assert!(
            serde_json::from_value::<UploadImageInput>(json!({
                "project_id": 1,
                "file_name": "reference.png",
                "mime_type": "image/png",
                "data_base64": "AA==",
                "image_type": "project_reference",
                "prompt": "reference",
                "metadata_json": ["legacy"]
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UploadImageInput>(json!({
                "project_id": 1,
                "file_name": "reference.png",
                "mime_type": "image/png",
                "data_base64": "AA==",
                "image_type": "project_reference",
                "prompt": "reference",
                "model": "spoofed"
            }))
            .is_err()
        );
    }

    #[test]
    fn video_reference_project_ids_are_required() {
        assert!(
            serde_json::from_value::<ListVideoReferenceImagesInput>(json!({
                "project_id": 1,
                "chapter_id": 2
            }))
            .is_err()
        );
    }

    #[test]
    fn generated_image_created_at_is_required() {
        let value = serde_json::to_value(GeneratedImageSummary {
            id: 1,
            project_id: 2,
            image_type: "project_reference".to_string(),
            file_path: "images/1.png".to_string(),
            prompt: "reference".to_string(),
            model: a4n_domain::EXTERNAL_IMPORT_MODEL.to_string(),
            width: 1,
            height: 1,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            is_system_managed: true,
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: "2026-08-10 00:00:00".to_string(),
        })
        .unwrap();
        assert_eq!(value["created_at"], "2026-08-10 00:00:00");
    }
}
