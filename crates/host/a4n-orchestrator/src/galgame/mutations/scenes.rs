use super::*;

impl OrchestratorContext {
    pub async fn update_galgame_scene(
        &self,
        project_id: i64,
        input: UpdateGalgameSceneInput,
    ) -> Result<galgame_scene::Model, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.scene_id, "scene_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let scene = self
            .load_scene_in_project(context.project.id, input.scene_id)
            .await?;
        let scene_id = scene.id;
        let script_id = scene.script_id;
        let current_entry_kind = scene.entry_kind.clone();
        let background_generation_inputs_changed = input.title.is_some()
            || input.location.is_some()
            || input.time_of_day.is_some()
            || input.background_note.is_some();
        let line_cg_generation_inputs_changed =
            input.title.is_some() || input.location.is_some() || input.time_of_day.is_some();
        let title = input
            .title
            .map(|title| required_text(&title, "scene.title").map(str::to_string))
            .transpose()?;
        let location = input.location.map(normalize_optional_text);
        let time_of_day = input.time_of_day.map(normalize_optional_text);
        let background_note = input.background_note.map(normalize_optional_text);
        let next_scene_id = if let Some(next_scene_id) = input.next_scene_id {
            Some(if let Some(next_scene_id) = next_scene_id {
                validate_positive_id(next_scene_id, "next_scene_id")?;
                if next_scene_id == scene_id {
                    return Err(AppError::InvalidArgument(
                        "Galgame 场 next_scene_id 不能指向自身".to_string(),
                    ));
                }
                let target_scene = self
                    .load_scene_in_project(context.project.id, next_scene_id)
                    .await?;
                if target_scene.script_id != script_id {
                    return Err(AppError::InvalidArgument(
                        "Galgame 场 next_scene_id 不属于同一剧本".to_string(),
                    ));
                }
                if current_entry_kind == SCENE_ENTRY_BRANCH
                    && target_scene.entry_kind != SCENE_ENTRY_MAIN
                {
                    return Err(AppError::InvalidArgument(
                        "Galgame 支线场 next_scene_id 必须指向主线场".to_string(),
                    ));
                }
                Some(next_scene_id)
            } else {
                None
            })
        } else {
            None
        };
        let record_input = UpdateGalgameSceneRecordInput {
            scene_id,
            title,
            location,
            time_of_day,
            background_note,
            next_scene_id,
        };
        let updated = a4n_db::galgame::update_galgame_scene_with_generation_cleanup_record(
            &self.database,
            context.project.id,
            record_input,
            background_generation_inputs_changed,
            line_cg_generation_inputs_changed,
        )
        .await
        .map_err(galgame_scene_record_error)?;
        Ok(updated)
    }
}
