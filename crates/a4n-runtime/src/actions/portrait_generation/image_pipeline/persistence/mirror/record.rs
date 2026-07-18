use chrono::NaiveDateTime;
use serde_json::Value;

use super::PortraitImageMirror;
use a4n_orchestrator::video::CreateGeneratedImageRecordDraftInput as CreateImageRecord;

pub(super) fn portrait_image_record(
    input: &PortraitImageMirror<'_>,
    metadata_json: Option<Value>,
    created_at: NaiveDateTime,
) -> CreateImageRecord {
    CreateImageRecord {
        project_id: input.project_id,
        image_type: "portrait".to_string(),
        file_path: input.image_path.to_string(),
        prompt: input.prompt.to_string(),
        model: input.model.to_string(),
        width: input.width,
        height: input.height,
        file_size: Some(input.file_size),
        chapter_id: None,
        panel_id: None,
        metadata_json,
        created_at: Some(created_at),
    }
}
