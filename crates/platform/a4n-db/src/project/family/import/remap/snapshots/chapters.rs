use serde_json::Value;

use crate::project::family::import::{ImportIdMaps, ProjectFamilyImportError, map_required};

pub(in crate::project::family::import) fn remap_chapter_hashes_json(
    value: &Value,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let rows = value.as_array().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("源文本快照 chapter_hashes_json 必须是数组")
    })?;
    let mut remapped = Vec::with_capacity(rows.len());
    for row in rows {
        let mut object = row.as_object().cloned().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("源文本快照 chapter_hashes_json 项必须是对象")
        })?;
        let old_id = object
            .get("source_chapter_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                ProjectFamilyImportError::invalid_package(
                    "chapter_hashes_json.source_chapter_id 必须是整数",
                )
            })?;
        object.insert(
            "source_chapter_id".to_string(),
            Value::from(map_required(
                &maps.chapter_ids,
                old_id,
                "chapter_hashes_json.source_chapter_id",
            )?),
        );
        remapped.push(Value::Object(object));
    }
    Ok(Value::Array(remapped))
}
