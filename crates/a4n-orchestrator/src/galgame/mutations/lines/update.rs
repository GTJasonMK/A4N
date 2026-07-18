use a4n_core::{error::AppError, models::galgame::galgame_line};
use a4n_db::galgame::UpdateGalgameLineRecordInput;

use crate::galgame::{
    OrchestratorContext, UpdateGalgameLineInput, galgame_line_record_error, normalize_line_type,
    normalize_optional_positive_id, required_text, validate_and_project_presentation,
    validate_line_semantics, validate_positive_id,
};

impl OrchestratorContext {
    pub async fn update_galgame_line(
        &self,
        project_id: i64,
        input: UpdateGalgameLineInput,
    ) -> Result<galgame_line::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.line_id, "line_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, input.line_id)
            .await?;
        let next_line_type = match input.line_type.as_deref() {
            Some(line_type) => normalize_line_type(line_type)?.to_string(),
            None => line.line_type.clone(),
        };
        let next_speaker_profile_id = match input.speaker_profile_id {
            Some(speaker_profile_id) => {
                normalize_optional_positive_id(speaker_profile_id, "speaker_profile_id")?
            }
            None => line.speaker_profile_id,
        };
        if (input.line_type.is_some() || input.speaker_profile_id.is_some())
            && input.presentation.is_none()
        {
            return Err(AppError::InvalidArgument(
                "修改 Galgame 行类型或说话人时必须同时提交 presentation".to_string(),
            ));
        }
        let presentation_projection = input
            .presentation
            .as_ref()
            .map(|presentation| {
                validate_and_project_presentation(
                    presentation,
                    next_speaker_profile_id,
                    &format!("Galgame 行 {} presentation", input.line_id),
                )
            })
            .transpose()?;
        let next_expression = presentation_projection
            .as_ref()
            .map(|projection| projection.0.clone())
            .unwrap_or_else(|| line.expression.clone());
        validate_line_semantics(
            &next_line_type,
            next_speaker_profile_id,
            next_expression.as_deref(),
        )?;
        if let Some(speaker_profile_id) = next_speaker_profile_id {
            self.validate_speaker_profile(&context, speaker_profile_id)
                .await?;
        }
        let visual_generation_inputs_changed = input.line_type.is_some()
            || input.speaker_profile_id.is_some()
            || input.text.is_some()
            || input.presentation.is_some()
            || input.text_anchor_id.is_some();
        let line_scene_id = line.scene_id;
        let line_type = input.line_type.is_some().then_some(next_line_type);
        let speaker_profile_id = input
            .speaker_profile_id
            .is_some()
            .then_some(next_speaker_profile_id);
        let text = input
            .text
            .map(|text| required_text(&text, "line.text").map(str::to_string))
            .transpose()?;
        let expression = presentation_projection
            .as_ref()
            .map(|projection| projection.0.clone());
        let pose = presentation_projection
            .as_ref()
            .map(|projection| projection.1.clone());
        let position = presentation_projection
            .as_ref()
            .map(|projection| projection.2.clone());
        let presentation_json = input
            .presentation
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        let text_anchor_id = if let Some(text_anchor_id) = input.text_anchor_id {
            if let Some(anchor_id) = text_anchor_id {
                validate_positive_id(anchor_id, "text_anchor_id")?;
                let scene = self
                    .load_scene_in_project(context.project.id, line_scene_id)
                    .await?;
                self.validate_text_anchor(
                    &context,
                    anchor_id,
                    Some(scene.source_snapshot_chapter_id),
                )
                .await?;
            }
            Some(text_anchor_id)
        } else {
            None
        };
        let record_input = UpdateGalgameLineRecordInput {
            line_id: input.line_id,
            line_type,
            speaker_profile_id,
            text,
            expression,
            pose,
            position,
            presentation_json,
            text_anchor_id,
        };
        let updated = a4n_db::galgame::update_galgame_line_with_generation_cleanup_record(
            self.db.connection(),
            context.project.id,
            record_input,
            visual_generation_inputs_changed,
        )
        .await
        .map_err(galgame_line_record_error)?;
        Ok(updated)
    }
}
