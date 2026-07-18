use a4n_core::{error::AppError, models::galgame::galgame_line};
use a4n_db::galgame::InsertGalgameLineRecordInput;

use crate::galgame::{
    InsertGalgameLineInput, OrchestratorContext, galgame_line_record_error, normalize_line_type,
    normalize_optional_positive_id, required_text, validate_and_project_presentation,
    validate_line_semantics, validate_positive_id,
};

impl OrchestratorContext {
    pub async fn insert_galgame_line(
        &self,
        project_id: i64,
        input: InsertGalgameLineInput,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.scene_id, "scene_id")?;
        if let Some(after_line_id) = input.after_line_id {
            validate_positive_id(after_line_id, "after_line_id")?;
        }
        if let Some(anchor_id) = input.text_anchor_id {
            validate_positive_id(anchor_id, "text_anchor_id")?;
        }
        let context = self.load_galgame_context(project_id).await?;
        let scene = self
            .load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        if let Some(anchor_id) = input.text_anchor_id {
            self.validate_text_anchor(&context, anchor_id, Some(scene.source_snapshot_chapter_id))
                .await?;
        }
        let line_type = normalize_line_type(&input.line_type)?;
        let speaker_profile_id =
            normalize_optional_positive_id(input.speaker_profile_id, "speaker_profile_id")?;
        let (expression, pose, position) = validate_and_project_presentation(
            &input.presentation,
            speaker_profile_id,
            "Galgame 新增行 presentation",
        )?;
        validate_line_semantics(line_type, speaker_profile_id, expression.as_deref())?;
        if let Some(speaker_profile_id) = speaker_profile_id {
            self.validate_speaker_profile(&context, speaker_profile_id)
                .await?;
        }
        let presentation_json =
            serde_json::to_value(input.presentation).map_err(AppError::Serialization)?;

        let record_input = InsertGalgameLineRecordInput {
            scene_id: scene.id,
            after_line_id: input.after_line_id,
            line_type: line_type.to_string(),
            speaker_profile_id,
            text: required_text(&input.text, "line.text")?.to_string(),
            expression,
            pose,
            position,
            presentation_json,
            text_anchor_id: input.text_anchor_id,
        };
        let inserted = a4n_db::galgame::insert_galgame_line_record_transaction(
            self.db.connection(),
            record_input,
        )
        .await
        .map_err(galgame_line_record_error)?;
        Ok(inserted)
    }
}
