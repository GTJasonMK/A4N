//! SearchFilter / DeleteFilter → Qdrant Filter。

use a4n_rag_core::vector_store::{DeleteFilter, SearchFilter};
use qdrant_client::qdrant::{Condition, Filter, Range};

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
pub fn search_filter(collection: &str, filter: Option<&SearchFilter>) -> Filter {
    let mut must = vec![collection_condition(collection)];
    if let Some(f) = filter {
        if let Some(eq) = &f.equals {
            for (k, v) in eq {
                match k.as_str() {
                    "project_id" => {
                        if let Some(n) = v.as_i64() {
                            must.push(Condition::matches(KEY_PROJECT_ID, n));
                        }
                    }
                    "data_type" => {
                        if let Some(s) = v.as_str() {
                            must.push(Condition::matches(KEY_DATA_TYPE, s.to_string()));
                        }
                    }
                    "chapter_number" => {
                        if let Some(n) = v.as_i64() {
                            must.push(Condition::matches(KEY_CHAPTER, n));
                        }
                    }
                    _ => {}
                }
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
        if let Some(types) = &f.data_types
            && !types.is_empty()
        {
            let should: Vec<Condition> = types
                .iter()
                .map(|t| Condition::matches(KEY_DATA_TYPE, t.clone()))
                .collect();
            must.push(Condition::from(Filter {
                should,
                ..Default::default()
            }));
        }
    }
    Filter {
        must,
        ..Default::default()
    }
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

/// 仅逻辑 collection 范围。
pub fn collection_filter(collection: &str) -> Filter {
    Filter {
        must: vec![collection_condition(collection)],
        ..Default::default()
    }
}
