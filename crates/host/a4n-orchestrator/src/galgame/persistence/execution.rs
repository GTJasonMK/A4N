use a4n_error::AppError;

use super::{records::galgame_script_record_input, validation::GalgamePersistValidationScope};
use crate::galgame::{
    GalgameChapterAdaptation, GalgameScriptDetail, GalgameScriptPersistPlan, OrchestratorContext,
    galgame_script_record_error, parse_and_validate_presentation_projection,
    validate_line_semantics, validate_positive_id, validate_scene_entry_kind,
};

impl OrchestratorContext {
    pub(in crate::galgame) async fn validate_line_adaptation_source(
        &self,
        project_id: i64,
        scene: &a4n_domain::galgame::GalgameSceneRecord,
        presentation: &crate::galgame::GalgameLinePresentationDocumentV3,
        text_anchor_id: Option<i64>,
    ) -> Result<(), AppError> {
        presentation
            .validate_source_identity(text_anchor_id)
            .map_err(AppError::InvalidArgument)?;
        if presentation.beat_ref.is_none() && presentation.after_beat_ref.is_none() {
            return Ok(());
        }
        let plan_id = scene
            .adaptation_plan_id
            .ok_or_else(|| AppError::InvalidArgument("场景缺少本地改编来源".into()))?;
        let unit_id = scene
            .adaptation_unit_id
            .as_deref()
            .ok_or_else(|| AppError::InvalidArgument("场景缺少本地单元".into()))?;
        let record = a4n_db::media_authoring::authoring_document(
            self.database.executor(),
            project_id,
            plan_id,
        )
        .await
        .map_err(crate::media_authoring::authoring_error)?;
        let source = record
            .inputs
            .sources
            .first()
            .filter(|source| source.source_snapshot_chapter_id == scene.source_snapshot_chapter_id)
            .ok_or_else(|| AppError::InvalidArgument("场景改编版本的原文不匹配".into()))?;
        let plan = serde_json::from_value(record.document)?;
        presentation
            .validate_adaptation(&plan, source.source_chapter_id, unit_id, text_anchor_id)
            .map_err(AppError::InvalidArgument)
    }

    pub(in crate::galgame) async fn persist_pipeline_script(
        &self,
        project_id: i64,
        plan: GalgameScriptPersistPlan,
        adaptations: &[GalgameChapterAdaptation],
        branch_plan: a4n_domain::media_authoring::AuthoringDocumentRef,
    ) -> Result<GalgameScriptDetail, AppError> {
        validate_positive_id(project_id, "project_id")?;
        if plan.scenes.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame persist_script 至少需要一个场".to_string(),
            ));
        }
        let (context_result, validation_scope_result) = futures::join!(
            self.load_galgame_context(project_id),
            GalgamePersistValidationScope::load(self, &plan),
        );
        let context = context_result?;
        let validation_scope = validation_scope_result?;
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
                    line.pose.as_deref(),
                )?;
                parse_and_validate_presentation_projection(
                    &line.presentation_json,
                    line.speaker_profile_id,
                    line.expression.as_deref(),
                    line.pose.as_deref(),
                    line.position.as_deref(),
                    &format!("Galgame persist 第 {} 行", line.line_index),
                )?;
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

        let record_input = galgame_script_record_input(&context, plan, adaptations, branch_plan)?;
        a4n_db::galgame::replace_galgame_script_for_project_record(
            self.database.executor(),
            context.project.id,
            record_input,
        )
        .await
        .map_err(galgame_script_record_error)?;
        self.get_script_detail(project_id).await
    }
}
