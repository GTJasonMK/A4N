use super::*;

pub(super) async fn insert_galgame_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.galgame_scripts,
        script,
        galgame_script::ActiveModel,
        maps.galgame_script_ids,
        "导入 Galgame 剧本失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                script.project_id,
                "Galgame 剧本 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                script.root_project_id,
                "Galgame 剧本 root_project_id",
            )?);
            active.source_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                script.source_snapshot_id,
                "Galgame 剧本 source_snapshot_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.galgame_scenes,
        scene,
        galgame_scene::ActiveModel,
        maps.galgame_scene_ids,
        "导入 Galgame 场失败",
        |active| {
            active.script_id = Set(map_required(
                &maps.galgame_script_ids,
                scene.script_id,
                "Galgame 场 script_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_required(
                &maps.source_snapshot_chapter_ids,
                scene.source_snapshot_chapter_id,
                "Galgame 场 source_snapshot_chapter_id",
            )?);
            active.participants_json = Set(remap_galgame_participants_json(
                &scene.participants_json,
                maps,
            )?);
            active.next_scene_id = Set(None);
        }
    );
    for scene in &package.galgame_scenes {
        let Some(next_scene_id) = scene.next_scene_id else {
            continue;
        };
        let new_scene_id = map_required(&maps.galgame_scene_ids, scene.id, "Galgame 场 ID")?;
        let new_next_scene_id = map_required(
            &maps.galgame_scene_ids,
            next_scene_id,
            "Galgame 场 next_scene_id",
        )?;
        let stored = galgame_scene::Entity::find_by_id(new_scene_id)
            .one(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "读取导入 Galgame 场失败",
                source,
            })?
            .ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(format!(
                    "导入 Galgame 场不存在: {new_scene_id}"
                ))
            })?;
        let mut active: galgame_scene::ActiveModel = stored.into();
        active.next_scene_id = Set(Some(new_next_scene_id));
        active
            .update(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新 Galgame 场 next_scene_id 失败",
                source,
            })?;
    }
    insert_mapped_import_rows!(
        txn,
        &package.galgame_lines,
        line,
        galgame_line::ActiveModel,
        maps.galgame_line_ids,
        "导入 Galgame 行失败",
        |active| {
            active.scene_id = Set(map_required(
                &maps.galgame_scene_ids,
                line.scene_id,
                "Galgame 行 scene_id",
            )?);
            active.speaker_profile_id = Set(map_optional(
                &maps.character_profile_ids,
                line.speaker_profile_id,
                "Galgame 行 speaker_profile_id",
            )?);
            active.text_anchor_id = Set(map_optional(
                &maps.text_anchor_ids,
                line.text_anchor_id,
                "Galgame 行 text_anchor_id",
            )?);
        }
    );
    insert_import_rows!(
        txn,
        &package.galgame_choices,
        choice,
        galgame_choice::ActiveModel,
        "导入 Galgame 选项失败",
        |active| {
            active.line_id = Set(map_required(
                &maps.galgame_line_ids,
                choice.line_id,
                "Galgame 选项 line_id",
            )?);
            let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(
                    "Galgame 选项 target_scene_id 不能为空".to_string(),
                )
            })?;
            active.target_scene_id = Set(Some(map_required(
                &maps.galgame_scene_ids,
                target_scene_id,
                "Galgame 选项 target_scene_id",
            )?));
        }
    );
    Ok(())
}

fn remap_galgame_participants_json(
    value: &Value,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let Some(items) = value.as_array() else {
        return Err(ProjectFamilyImportError::InvalidPackage(
            "Galgame 场 participants_json 必须是数组".to_string(),
        ));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let mut object = item.as_object().cloned().ok_or_else(|| {
            ProjectFamilyImportError::InvalidPackage(
                "Galgame 场 participants_json 项必须是对象".to_string(),
            )
        })?;
        if let Some(old_id) = object.get("character_profile_id").and_then(Value::as_i64) {
            object.insert(
                "character_profile_id".to_string(),
                Value::from(map_required(
                    &maps.character_profile_ids,
                    old_id,
                    "Galgame 场 participants_json.character_profile_id",
                )?),
            );
        }
        out.push(Value::Object(object));
    }
    Ok(Value::Array(out))
}
