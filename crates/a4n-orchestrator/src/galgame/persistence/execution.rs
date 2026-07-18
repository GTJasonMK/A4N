use a4n_core::error::AppError;

use super::{records::galgame_script_record_input, validation::GalgamePersistValidationScope};
use crate::galgame::{
    GalgameScriptDetail, GalgameScriptPersistPlan, OrchestratorContext, ensure_json_object,
    galgame_script_record_error, validate_line_semantics, validate_positive_id,
    validate_scene_entry_kind,
};

impl OrchestratorContext {
    pub(in crate::galgame) async fn persist_pipeline_script(
        &self,
        project_id: i64,
        plan: GalgameScriptPersistPlan,
    ) -> Result<GalgameScriptDetail, AppError> {
        validate_positive_id(project_id, "project_id")?;
        if plan.scenes.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame persist_script 至少需要一个场".to_string(),
            ));
        }
        let context = self.load_galgame_context(project_id).await?;
        let validation_scope = GalgamePersistValidationScope::load(self, &plan).await?;
        for scene in &plan.scenes {
            validate_positive_id(i64::from(scene.scene_index), "scene_index")?;
            validate_positive_id(
                scene.source_snapshot_chapter_id,
                "source_snapshot_chapter_id",
            )?;
            validation_scope
                .validate_source_snapshot_chapter(&context, scene.source_snapshot_chapter_id)?;
            validate_scene_entry_kind(&scene.entry_kind)?;
            if scene.next_scene_index == Some(scene.scene_index) {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame 场 next_scene_index 不能指向自身: scene_index={}",
                    scene.scene_index
                )));
            }
            validation_scope
                .validate_scene_participants_json(&context, &scene.participants_json)?;
            for line in &scene.lines {
                validate_positive_id(i64::from(line.line_index), "line_index")?;
                validate_line_semantics(
                    &line.line_type,
                    line.speaker_profile_id,
                    line.expression.as_deref(),
                )?;
                ensure_json_object(&line.presentation_json, "presentation_json")?;
                if let Some(speaker_profile_id) = line.speaker_profile_id {
                    validation_scope.validate_speaker_profile(&context, speaker_profile_id)?;
                }
                if let Some(text_anchor_id) = line.text_anchor_id {
                    validation_scope.validate_text_anchor(
                        &context,
                        text_anchor_id,
                        Some(scene.source_snapshot_chapter_id),
                    )?;
                }
            }
        }

        let record_input = galgame_script_record_input(&context, plan)?;
        a4n_db::galgame::replace_galgame_script_for_project_record(
            self.db.connection(),
            context.project.id,
            record_input,
        )
        .await
        .map_err(galgame_script_record_error)?;
        self.get_script_detail(project_id).await
    }
}
