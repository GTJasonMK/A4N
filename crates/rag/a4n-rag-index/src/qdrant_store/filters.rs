//! SearchFilter / ID selector → Qdrant Filter。

use a4n_domain::ProviderError as AppError;
use a4n_rag_core::vector_store::SearchFilter;
use qdrant_client::qdrant::{Condition, Filter, Range, r#match::MatchValue};
use serde_json::Value;

pub(super) const KEY_DATA_TYPE: &str = "data_type";
pub(super) const KEY_CHAPTER: &str = "chapter_number";
pub(super) const KEY_CHUNK_ID: &str = "chunk_id";
pub(super) const KEY_ID_PREFIX: &str = "id_prefix";

pub(super) fn search_filter(filter: Option<&SearchFilter>) -> Result<Filter, AppError> {
    let mut must = Vec::new();
    if let Some(f) = filter {
        if let Some(eq) = &f.equals {
            let mut entries = eq.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| key.as_str());
            for (key, value) in entries {
                must.push(metadata_equals_condition(key, value)?);
            }
        }
        if let Some((from, to)) = f.chapter_range {
            must.push(Condition::range(
                KEY_CHAPTER,
                Range {
                    gte: Some(f64::from(from)),
                    lte: Some(f64::from(to)),
                    ..Default::default()
                },
            ));
        }
        if let Some(types) = &f.data_types {
            if types.is_empty() {
                return Err(AppError::InvalidArgument(
                    "Qdrant SearchFilter.data_types 不能为空；空列表不能静默放宽为全部类型"
                        .to_string(),
                ));
            }
            let should: Vec<Condition> = types
                .iter()
                .map(|data_type| {
                    Condition::matches(KEY_DATA_TYPE, MatchValue::Keyword(data_type.clone()))
                })
                .collect();
            must.push(Condition::from(Filter {
                should,
                ..Default::default()
            }));
        }
    }
    Ok(Filter {
        must,
        ..Default::default()
    })
}

fn metadata_equals_condition(key: &str, value: &Value) -> Result<Condition, AppError> {
    let path = metadata_field_path(key)?;
    match value {
        Value::Null => Ok(Condition::is_null(path)),
        Value::Bool(value) => Ok(Condition::matches(path, *value)),
        Value::Number(value) => value
            .as_i64()
            .map(|value| Condition::matches(path, value))
            .ok_or_else(|| unsupported_equals_value(key, "仅支持 i64 范围内的整数")),
        Value::String(value) => Ok(Condition::matches(path, MatchValue::Keyword(value.clone()))),
        Value::Array(_) => Err(unsupported_equals_value(
            key,
            "数组在 Qdrant 中是任一元素匹配，无法表达 MemoryVectorStore 的整数组相等",
        )),
        Value::Object(_) => Err(unsupported_equals_value(
            key,
            "对象无法用 Qdrant field match 表达完整 JSON 相等",
        )),
    }
}

fn metadata_field_path(key: &str) -> Result<String, AppError> {
    let mut chars = key.chars();
    let valid_start = chars
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic() || value == '_');
    let valid_rest = chars.all(|value| value.is_ascii_alphanumeric() || value == '_');
    if !valid_start || !valid_rest {
        return Err(AppError::InvalidArgument(format!(
            "Qdrant SearchFilter.equals metadata key `{key}` 无法安全映射；仅支持 [A-Za-z_][A-Za-z0-9_]*"
        )));
    }
    Ok(format!("metadata.{key}"))
}

fn unsupported_equals_value(key: &str, reason: &str) -> AppError {
    AppError::InvalidArgument(format!(
        "Qdrant SearchFilter.equals `{key}` 无法安全表达：{reason}"
    ))
}

pub(super) fn delete_filter(ids: &[String], prefixes: &[String]) -> Filter {
    if ids.is_empty() && prefixes.is_empty() {
        return Filter::default();
    }
    let should = ids
        .iter()
        .map(|id| Condition::matches(KEY_CHUNK_ID, id.clone()))
        .chain(
            prefixes
                .iter()
                .map(|prefix| Condition::matches(KEY_ID_PREFIX, prefix.clone())),
        )
        .collect();
    Filter {
        must: vec![Condition::from(Filter {
            should,
            ..Default::default()
        })],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use qdrant_client::qdrant::{condition::ConditionOneOf, r#match::MatchValue};
    use serde_json::json;

    use super::*;

    fn equals_filter(entries: impl IntoIterator<Item = (&'static str, Value)>) -> SearchFilter {
        SearchFilter {
            equals: Some(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.to_string(), value))
                    .collect::<HashMap<_, _>>(),
            ),
            chapter_range: None,
            data_types: None,
        }
    }

    fn field_match<'a>(filter: &'a Filter, key: &str) -> Option<&'a MatchValue> {
        filter.must.iter().find_map(|condition| {
            let ConditionOneOf::Field(field) = condition.condition_one_of.as_ref()? else {
                return None;
            };
            (field.key == key)
                .then(|| field.r#match.as_ref()?.match_value.as_ref())
                .flatten()
        })
    }

    fn has_null_condition(filter: &Filter, key: &str) -> bool {
        filter.must.iter().any(|condition| {
            matches!(
                condition.condition_one_of.as_ref(),
                Some(ConditionOneOf::IsNull(condition)) if condition.key == key
            )
        })
    }

    #[test]
    fn equals_maps_identity_snapshot_and_root_scope_to_nested_metadata() {
        let filter = equals_filter([
            ("project_id", json!(42)),
            ("data_type", json!("chapter")),
            ("root_project_id", json!(42)),
            ("source_snapshot_id", Value::Null),
            ("derived_project_id", Value::Null),
            ("embedding_backend", json!("LocalHash")),
            ("embedding_model", json!("model with spaces")),
            ("embedding_dimension", json!(768)),
        ]);

        let qdrant = search_filter(Some(&filter)).expect("supported equals filter");

        assert_eq!(
            field_match(&qdrant, "metadata.project_id"),
            Some(&MatchValue::Integer(42))
        );
        assert_eq!(
            field_match(&qdrant, "metadata.data_type"),
            Some(&MatchValue::Keyword("chapter".to_string()))
        );
        assert_eq!(
            field_match(&qdrant, "metadata.root_project_id"),
            Some(&MatchValue::Integer(42))
        );
        assert_eq!(
            field_match(&qdrant, "metadata.embedding_backend"),
            Some(&MatchValue::Keyword("LocalHash".to_string()))
        );
        assert_eq!(
            field_match(&qdrant, "metadata.embedding_model"),
            Some(&MatchValue::Keyword("model with spaces".to_string()))
        );
        assert_eq!(
            field_match(&qdrant, "metadata.embedding_dimension"),
            Some(&MatchValue::Integer(768))
        );
        assert!(has_null_condition(&qdrant, "metadata.source_snapshot_id"));
        assert!(has_null_condition(&qdrant, "metadata.derived_project_id"));
    }

    #[test]
    fn equals_rejects_values_or_keys_that_would_widen_the_query() {
        for filter in [
            equals_filter([("analysis.tags", json!("悬疑"))]),
            equals_filter([("related_characters", json!(["林岚"]))]),
            equals_filter([("analysis", json!({"tone": "紧张"}))]),
            equals_filter([("score", json!(0.5))]),
        ] {
            let error = search_filter(Some(&filter))
                .expect_err("unrepresentable equals must fail instead of being ignored");
            assert!(error.to_string().contains("Qdrant SearchFilter.equals"));
        }
    }

    #[test]
    fn empty_data_types_is_rejected_instead_of_matching_all_types() {
        let filter = SearchFilter {
            equals: None,
            chapter_range: None,
            data_types: Some(Vec::new()),
        };

        let error =
            search_filter(Some(&filter)).expect_err("empty data type scope must not be ignored");
        assert!(error.to_string().contains("data_types 不能为空"));
    }

    #[test]
    fn exact_ids_and_prefixes_share_one_union_group() {
        let filter = delete_filter(&["chapter:42:1".to_string()], &["outline:42:".to_string()]);
        let Some(ConditionOneOf::Filter(selectors)) = filter.must[0].condition_one_of.as_ref()
        else {
            panic!("id selectors must be grouped in one nested filter");
        };
        let keys = selectors
            .should
            .iter()
            .map(|condition| match condition.condition_one_of.as_ref() {
                Some(ConditionOneOf::Field(field)) => field.key.as_str(),
                _ => panic!("id selector must be a field condition"),
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, vec![KEY_CHUNK_ID, KEY_ID_PREFIX]);
    }
}
