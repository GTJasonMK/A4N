//! 角色冲突检查输出解析纯规则。

use a4n_api_contract::character::CharacterConflictSummary;
use a4n_core::error::AppError;
use serde_json::Value;
use std::collections::BTreeSet;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

pub fn parse_conflict_output_with_source_ids(
    content: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<CharacterConflictSummary>, AppError> {
    let mut conflicts = parse_conflict_output(content)?;
    for (index, conflict) in conflicts.iter_mut().enumerate() {
        conflict.field = normalized_required_string(index, "field", &conflict.field)?;
        conflict.expected_source_id =
            normalized_required_string(index, "expected_source_id", &conflict.expected_source_id)?;
        conflict.actual_source_id =
            normalized_required_string(index, "actual_source_id", &conflict.actual_source_id)?;
        validate_conflict_source_id(
            index,
            "expected_source_id",
            &conflict.expected_source_id,
            allowed_source_ids,
        )?;
        validate_conflict_source_id(
            index,
            "actual_source_id",
            &conflict.actual_source_id,
            allowed_source_ids,
        )?;
    }
    Ok(conflicts)
}

pub fn restored_character_conflict_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    call_key: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<Vec<CharacterConflictSummary>, AppError> {
    let reparsed = parse_conflict_output_with_source_ids(response_content, allowed_source_ids)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &serde_json::to_value(&reparsed)?,
        &format!("角色冲突检查 {call_key}"),
    )?;
    Ok(reparsed)
}

pub fn parse_conflict_output(content: &str) -> Result<Vec<CharacterConflictSummary>, AppError> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        AppError::InvalidArgument(format!("角色冲突检查结果必须是 JSON 数组: {err}"))
    })?;
    let conflicts = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("角色冲突检查结果必须是 JSON 数组".to_string()))?;
    conflicts
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, conflict)| {
            serde_json::from_value(conflict).map_err(|error| {
                AppError::InvalidArgument(format!(
                    "conflicts[{index}] 不符合角色冲突结果合同: {error}"
                ))
            })
        })
        .collect()
}

fn validate_conflict_source_id(
    index: usize,
    field: &str,
    source_id: &str,
    allowed_source_ids: &BTreeSet<String>,
) -> Result<(), AppError> {
    if allowed_source_ids.contains(source_id) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].{field} 必须来自本次输入 source_id: {source_id}"
        )))
    }
}

fn normalized_required_string(index: usize, field: &str, value: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        Err(AppError::InvalidArgument(format!(
            "conflicts[{index}].{field} 必须是非空字符串"
        )))
    } else {
        Ok(normalized.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    #[test]
    fn parse_conflict_output_accepts_empty_and_rejects_embedded() {
        assert!(parse_conflict_output("[]").unwrap().is_empty());
        assert!(parse_conflict_output(r#"结果如下：[{"chapter_number":1}]"#).is_err());
    }

    #[test]
    fn parse_conflict_output_validates_item_contract() {
        let invalid_severity = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"critical"}]"#;
        assert!(parse_conflict_output(invalid_severity).is_err());
        let missing_source = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","severity":"major"}]"#;
        assert!(
            parse_conflict_output(missing_source)
                .unwrap_err()
                .to_string()
                .contains("expected_source_id")
        );
        let unknown = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"major","notes":"多余说明"}]"#;
        assert!(
            parse_conflict_output(unknown)
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );

        let ok = r#"[{"chapter_number":3,"field":"goal","expected":"a","actual":"b","expected_source_id":"profile","actual_source_id":"chapter:3","severity":"major"}]"#;
        let allowed = ["profile".into(), "chapter:3".into()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            parse_conflict_output_with_source_ids(ok, &allowed)
                .unwrap()
                .len(),
            1
        );
        assert!(
            parse_conflict_output_with_source_ids(ok, &BTreeSet::from(["profile".into()])).is_err()
        );
    }

    #[test]
    fn restored_checkpoint_revalidates_and_rejects_mismatch() {
        let allowed = ["profile".to_string(), "chapter:3".to_string()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let raw = r#"[{"chapter_number":3,"field":"goal","expected":"留在旧城","actual":"已经离开旧城","expected_source_id":"profile","actual_source_id":"chapter:3","severity":"major"}]"#;
        let output = parse_conflict_output_with_source_ids(raw, &allowed).unwrap();
        let restored = restored_character_conflict_checkpoint(
            &json!(output),
            raw,
            "character_conflict_check.profile.1",
            &allowed,
        )
        .unwrap();
        assert_eq!(restored.len(), 1);

        let err = restored_character_conflict_checkpoint(
            &json!([]),
            raw,
            "character_conflict_check.profile.1",
            &allowed,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }
}
