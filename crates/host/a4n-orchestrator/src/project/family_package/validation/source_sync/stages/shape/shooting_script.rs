use serde_json::{Map, Value};

pub(in super::super::super) fn validate_regenerate_shooting_script_projection_stage_output(
    object: &Map<String, Value>,
    expected_chapter_number: i32,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".shooting_script_projection") {
        return Ok(());
    }
    let projection =
        crate::galgame::shooting_script_projection_from_value(Value::Object(object.clone()), path)
            .map_err(|error| format!("项目族导入包 {owner_kind}_id={owner_id}: {error}"))?;
    ensure_package!(
        projection.scenes.len() == 1 && projection.scenes[0].index == 1,
        "项目族导入包 regenerate_scene {path}.scenes 必须且只能包含一个 index=1 拍摄剧本投影场: {owner_kind}_id={}, scene_count={}",
        owner_id,
        projection.scenes.len()
    );
    ensure_package!(
        projection.scenes[0].chapter_number == expected_chapter_number,
        "项目族导入包 regenerate_scene {path}.scenes[0].chapter_number 与重建目标章节不一致: chapter_number={}, expected={}",
        projection.scenes[0].chapter_number,
        expected_chapter_number
    );
    Ok(())
}
