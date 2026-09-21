use super::*;

impl OrchestratorContext {
    pub async fn update_galgame_choices(
        &self,
        project_id: i64,
        input: UpdateGalgameChoicesInput,
    ) -> Result<Vec<galgame_choice::Model>, AppError> {
        validate_positive_id(project_id, "project_id")?;
        validate_positive_id(input.line_id, "line_id")?;
        if !(2..=3).contains(&input.choices.len()) {
            return Err(AppError::InvalidArgument(
                "Galgame choice 选项数量必须是 2 到 3 个".to_string(),
            ));
        }
        let context = self.load_galgame_context(project_id).await?;
        let line = self
            .load_line_in_project(context.project.id, input.line_id)
            .await?;
        if line.line_type != LINE_TYPE_CHOICE {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 行 {} 不是 choice 类型，不能编辑选择支",
                line.id
            )));
        }
        let line_scene = self
            .load_scene_in_project(context.project.id, line.scene_id)
            .await?;
        let mut seen_targets = BTreeSet::new();
        let mut normalized = Vec::with_capacity(input.choices.len());
        for (index, choice) in input.choices.into_iter().enumerate() {
            validate_positive_id(choice.target_scene_id, "target_scene_id")?;
            let target_scene = self
                .load_scene_in_project(context.project.id, choice.target_scene_id)
                .await?;
            if target_scene.script_id != line_scene.script_id {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame choice target_scene_id {} 不属于同一剧本",
                    target_scene.id
                )));
            }
            if target_scene.entry_kind == SCENE_ENTRY_BRANCH {
                let return_scene_id = target_scene.next_scene_id.ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 指向未回收敛支线",
                        target_scene.id
                    ))
                })?;
                let return_scene = self
                    .load_scene_in_project(context.project.id, return_scene_id)
                    .await?;
                if return_scene.script_id != line_scene.script_id {
                    return Err(AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 的回收敛目标不属于同一剧本",
                        target_scene.id
                    )));
                }
                if return_scene.entry_kind != SCENE_ENTRY_MAIN {
                    return Err(AppError::InvalidArgument(format!(
                        "Galgame choice target_scene_id {} 的回收敛目标必须是主线场",
                        target_scene.id
                    )));
                }
            }
            let option_text = required_text(&choice.option_text, "choice.option_text")?.to_string();
            normalized.push(ReplaceGalgameChoiceRecordInput {
                option_index: to_i32(index + 1, "option_index")?,
                option_text,
                target_scene_id: target_scene.id,
            });
            seen_targets.insert(target_scene.id);
        }
        if seen_targets.is_empty() {
            return Err(AppError::InvalidArgument(
                "Galgame choice 至少需要一个有效目标场".to_string(),
            ));
        }

        let choices = a4n_db::galgame::replace_galgame_choice_records_transaction(
            self.database.connection(),
            line.id,
            normalized,
        )
        .await
        .map_err(AppError::Database)?;
        Ok(choices)
    }
}
