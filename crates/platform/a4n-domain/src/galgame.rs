mod kinds;

pub mod pipeline;
pub mod presentation;
pub mod records;
pub mod validation;

pub use kinds::*;
pub use pipeline::GalgamePipelineStartStage;
pub use presentation::*;
pub use records::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineBeatRefV4 {
    pub source_chapter_id: i64,
    pub unit_id: String,
    pub beat_id: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GalgameAdaptationUnitRef {
    pub source_chapter_id: i64,
    pub unit_id: String,
}

#[cfg(test)]
mod tests {
    use super::GalgameLinePresentationDocumentV3;
    use serde_json::json;

    #[test]
    fn presentation_document_requires_nullable_wire_fields() {
        let canonical = json!({
            "schema_version": "galgame_line_presentation_v3",
            "origin": "added", "beat_ref": null,
            "visible_cast": [],
            "enter": [],
            "exit": [],
            "focus_character_profile_id": null,
            "hold": false,
            "player_knowledge": [],
            "cg": null,
            "cg_note": null,
            "cg_importance": null,
            "transition": null,
            "branch": null,
            "branch_id": null,
            "returns_to_scene_index": null,
            "after_beat_ref": null
        });

        serde_json::from_value::<GalgameLinePresentationDocumentV3>(canonical.clone())
            .expect("canonical presentation");
        for field in [
            "origin",
            "beat_ref",
            "focus_character_profile_id",
            "cg",
            "cg_note",
            "cg_importance",
            "transition",
            "branch",
            "branch_id",
            "returns_to_scene_index",
            "after_beat_ref",
        ] {
            let mut missing = canonical.clone();
            missing
                .as_object_mut()
                .expect("presentation object")
                .remove(field);
            assert!(
                serde_json::from_value::<GalgameLinePresentationDocumentV3>(missing).is_err(),
                "missing {field} must fail"
            );
        }
    }
}
