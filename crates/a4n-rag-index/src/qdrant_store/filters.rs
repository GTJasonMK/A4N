//! SearchFilter / DeleteFilter → Qdrant Filter。

use a4n_core::error::AppError;
use a4n_rag_core::vector_store::{DeleteFilter, SearchFilter};
use qdrant_client::qdrant::{Condition, Filter, Range, r#match::MatchValue};
use serde_json::Value;

/// 逻辑 collection 名（业务侧多为 project_id 字符串，测试可任意）。
pub const KEY_LOGICAL_COLLECTION: &str = "logical_collection";
pub const KEY_PROJECT_ID: &str = "project_id";
pub const KEY_DATA_TYPE: &str = "data_type";
pub const KEY_CHAPTER: &str = "chapter_number";
pub const KEY_CHUNK_ID: &str = "chunk_id";
pub const KEY_ID_PREFIX: &str = "id_prefix";

fn collection_condition(collection: &str) -> Condition {
    Condition::matches(KEY_LOGICAL_COLLECTION, collection.to_string())
}

/// 检索过滤：强制逻辑 collection + 可选 SearchFilter。
pub fn search_filter(collection: &str, filter: Option<&SearchFilter>) -> Result<Filter, AppError> {
    let mut must = vec![collection_condition(collection)];
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

/// 删除过滤基础。
pub fn delete_filter_base(collection: &str, filter: &DeleteFilter) -> Filter {
    let mut must = vec![collection_condition(collection)];
    // 若 collection 即 project_id 字符串，与 filter.project_id 对齐校验在 store 层
    must.push(Condition::matches(KEY_PROJECT_ID, filter.project_id));
    if let Some(ref dt) = filter.data_type {
        must.push(Condition::matches(KEY_DATA_TYPE, dt.clone()));
    }
    if let Some(ch) = filter.chapter_number {
        must.push(Condition::matches(KEY_CHAPTER, i64::from(ch)));
    }
    Filter {
        must,
        ..Default::default()
    }
}

pub fn with_chunk_ids(base: Filter, ids: &[String]) -> Filter {
    if ids.is_empty() {
        return base;
    }
    let should: Vec<Condition> = ids
        .iter()
        .map(|id| Condition::matches(KEY_CHUNK_ID, id.clone()))
        .collect();
    let mut must = base.must;
    must.push(Condition::from(Filter {
        should,
        ..Default::default()
    }));
    Filter {
        must,
        ..Default::default()
    }
}

pub fn with_id_prefixes(base: Filter, prefixes: &[String]) -> Filter {
    if prefixes.is_empty() {
        return base;
    }
    let should: Vec<Condition> = prefixes
        .iter()
        .map(|p| Condition::matches(KEY_ID_PREFIX, p.clone()))
        .collect();
    let mut must = base.must;
    must.push(Condition::from(Filter {
        should,
        ..Default::default()
    }));
    Filter {
        must,
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

        let qdrant = search_filter("42", Some(&filter)).expect("supported equals filter");

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
            let error = search_filter("42", Some(&filter))
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

        let error = search_filter("42", Some(&filter))
            .expect_err("empty data type scope must not be ignored");
        assert!(error.to_string().contains("data_types 不能为空"));
    }
}
