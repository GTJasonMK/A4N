use super::*;

pub(super) fn parse_import_blueprint(root: &Value) -> Result<Option<ImportBlueprint>, String> {
    let Some(value) = optional_top_object(root, "blueprint", "导入文件")? else {
        return Ok(None);
    };
    Ok(Some(ImportBlueprint {
        title: required_json_text(value, "title", "导入蓝图标题")?.to_string(),
        genre: required_json_text(value, "genre", "导入蓝图题材")?.to_string(),
        style: optional_json_string(value, "style", "导入蓝图风格")?.map(ToString::to_string),
        tone: optional_json_string(value, "tone", "导入蓝图基调")?.map(ToString::to_string),
        target_audience: optional_json_string(value, "target_audience", "导入蓝图受众")?
            .map(ToString::to_string),
        one_sentence_summary: optional_json_string(
            value,
            "one_sentence_summary",
            "导入蓝图一句话概要",
        )?
        .map(ToString::to_string),
        full_synopsis: optional_json_string(value, "full_synopsis", "导入蓝图完整梗概")?
            .map(ToString::to_string),
        world_setting: optional_json_object_clone(value, "world_setting", "导入蓝图世界观")?,
        current_version: required_json_text(value, "current_version", "导入蓝图版本")?.to_string(),
        needs_part_outlines: required_json_bool(value, "needs_part_outlines", "导入蓝图")?,
        total_chapters: optional_json_positive_i32_field(
            value,
            "total_chapters",
            "导入蓝图总章节数",
        )?,
        chapters_per_part: optional_json_positive_i32_field(
            value,
            "chapters_per_part",
            "导入蓝图分部章节数",
        )?,
        avatar_svg: optional_json_string(value, "avatar_svg", "导入蓝图头像 SVG")?
            .map(ToString::to_string),
        avatar_animal: optional_json_string(value, "avatar_animal", "导入蓝图头像")?
            .map(ToString::to_string),
    }))
}

pub(super) fn parse_import_blueprint_versions(
    root: &Value,
) -> Result<Vec<ImportBlueprintVersion>, String> {
    optional_top_array(root, "blueprint_versions", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入蓝图版本{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportBlueprintVersion {
                version: required_json_text(value, "version", &label)?.to_string(),
                change_description: optional_json_string(value, "change_description", &label)?
                    .map(ToString::to_string),
                blueprint_data: required_json_object(value, "blueprint_data", &label)?.clone(),
                world_setting_snapshot: optional_json_value(value, "world_setting_snapshot"),
                characters_snapshot: optional_json_value(value, "characters_snapshot"),
                relationships_snapshot: optional_json_value(value, "relationships_snapshot"),
                synopsis_snapshot: optional_json_string(value, "synopsis_snapshot", &label)?
                    .map(ToString::to_string),
                created_by: optional_json_string(value, "created_by", &label)?
                    .map(ToString::to_string),
            })
        })
        .collect()
}

pub(super) fn parse_import_characters(root: &Value) -> Result<Vec<ImportCharacter>, String> {
    optional_top_array(root, "characters", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入角色{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportCharacter {
                name: required_json_text(value, "name", &label)?.to_string(),
                character_type: required_json_text(value, "character_type", &label)?.to_string(),
                gender: optional_json_string(value, "gender", &label)?.map(ToString::to_string),
                age: optional_json_string(value, "age", &label)?.map(ToString::to_string),
                appearance: optional_json_string(value, "appearance", &label)?
                    .map(ToString::to_string),
                personality: optional_json_string(value, "personality", &label)?
                    .map(ToString::to_string),
                background: optional_json_string(value, "background", &label)?
                    .map(ToString::to_string),
                motivation: optional_json_string(value, "motivation", &label)?
                    .map(ToString::to_string),
                arc_summary: optional_json_string(value, "arc_summary", &label)?
                    .map(ToString::to_string),
                relationship_to_protagonist: optional_json_string(
                    value,
                    "relationship_to_protagonist",
                    &label,
                )?
                .map(ToString::to_string),
                arc_stages: optional_json_value(value, "arc_stages"),
            })
        })
        .collect()
}

pub(super) fn parse_import_relationships(root: &Value) -> Result<Vec<ImportRelationship>, String> {
    optional_top_array(root, "relationships", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入角色关系{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportRelationship {
                relationship_type: required_json_text(value, "relationship_type", &label)?
                    .to_string(),
                character_a: required_json_text(value, "character_a", &label)?.to_string(),
                character_b: required_json_text(value, "character_b", &label)?.to_string(),
                description: optional_json_string(value, "description", &label)?
                    .map(ToString::to_string),
                strength: optional_json_positive_i32_field(value, "strength", &label)?,
                evolution: optional_json_value(value, "evolution"),
            })
        })
        .collect()
}
