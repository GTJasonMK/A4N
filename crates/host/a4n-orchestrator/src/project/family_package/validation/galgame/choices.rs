use super::super::*;

pub(super) fn validate_galgame_choices(
    package: &ProjectFamilyPackage,
    lines: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameLineRecord>,
    scenes: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
) -> Result<(), String> {
    let mut choices_by_line =
        HashMap::<i64, Vec<&a4n_domain::project::ProjectFamilyGalgameChoiceRecord>>::new();
    for choice in &package.galgame_choices {
        choices_by_line
            .entry(choice.line_id)
            .or_default()
            .push(choice);
    }
    for line in package
        .galgame_lines
        .iter()
        .filter(|line| line.line_type == "choice")
    {
        let choices = choices_by_line
            .get(&line.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !(2..=3).contains(&choices.len()) {
            return Err(format!(
                "项目族导入包 Galgame choice 行选项数量必须是 2 到 3 个: line_id={}, choice_count={}",
                line.id,
                choices.len()
            ));
        }
        let mut option_indexes = BTreeSet::new();
        for choice in choices {
            if choice.option_text.trim().is_empty() {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_text 不能为空: choice_id={}, line_id={}",
                    choice.id, choice.line_id
                ));
            }
            if choice.option_index <= 0 {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_index 必须为正数: choice_id={}, line_id={}, option_index={}",
                    choice.id, choice.line_id, choice.option_index
                ));
            }
            if !option_indexes.insert(choice.option_index) {
                return Err(format!(
                    "项目族导入包 Galgame 选项 option_index 重复: choice_id={}, line_id={}, option_index={}",
                    choice.id, choice.line_id, choice.option_index
                ));
            }
        }
        let choice_count = i32::try_from(choices.len()).map_err(|_| {
            format!(
                "项目族导入包 Galgame 行 {} 的选项数量超过 i32 范围：{}",
                line.id,
                choices.len()
            )
        })?;
        let expected_indexes = (1..=choice_count).collect::<BTreeSet<_>>();
        if option_indexes != expected_indexes {
            return Err(format!(
                "项目族导入包 Galgame 选项 option_index 必须从 1 连续编号: line_id={}, option_indexes={:?}, expected={:?}",
                line.id, option_indexes, expected_indexes
            ));
        }
    }

    for choice in &package.galgame_choices {
        let line = lines.get(&choice.line_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项引用了不存在的行: choice_id={}, line_id={}",
                choice.id, choice.line_id
            )
        })?;
        if line.line_type != "choice" {
            return Err(format!(
                "项目族导入包 Galgame 选项必须挂在 choice 行下: choice_id={}, line_id={}",
                choice.id, choice.line_id
            ));
        }
        let line_scene = scenes.get(&line.scene_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项所属行引用了不存在的场: choice_id={}, line_id={}, scene_id={}",
                choice.id, line.id, line.scene_id
            )
        })?;
        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项缺少 target_scene_id: choice_id={}",
                choice.id
            )
        })?;
        let target_scene = scenes.get(&target_scene_id).ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 选项 target_scene_id 不存在: choice_id={}, target_scene_id={target_scene_id}",
                choice.id
            )
        })?;
        if target_scene.script_id != line_scene.script_id {
            return Err(format!(
                "项目族导入包 Galgame 选项 target_scene_id 不属于同一剧本: choice_id={}, line_id={}, target_scene_id={target_scene_id}",
                choice.id, line.id
            ));
        }
        if target_scene.entry_kind == "branch" {
            let return_scene_id = target_scene.next_scene_id.ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 选项 target_scene_id 指向未回收敛支线: choice_id={}, target_scene_id={target_scene_id}",
                    choice.id
                )
            })?;
            let return_scene = scenes.get(&return_scene_id).ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标不存在: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                )
            })?;
            if return_scene.script_id != line_scene.script_id {
                return Err(format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标不属于同一剧本: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                ));
            }
            if return_scene.entry_kind != "main" {
                return Err(format!(
                    "项目族导入包 Galgame 选项 target_scene_id 回收敛目标必须是主线场: choice_id={}, target_scene_id={target_scene_id}, next_scene_id={return_scene_id}",
                    choice.id
                ));
            }
            if return_scene.scene_index <= line_scene.scene_index {
                return Err(format!(
                    "项目族导入包 Galgame 选项 branch 回收敛场必须晚于选择所在主线场: choice_id={}, source_scene_index={}, return_scene_index={}",
                    choice.id, line_scene.scene_index, return_scene.scene_index
                ));
            }
        }
    }
    Ok(())
}
