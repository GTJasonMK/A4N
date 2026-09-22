use serde::Serialize;
use serde_json::{Value, json};

pub(in crate::project::dependency_graph) fn db_record_result<T>(table: &str, record: &T) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
    })
}

pub(in crate::project::dependency_graph) fn db_record_result_with_related<T>(
    table: &str,
    record: &T,
    related_records: Vec<Value>,
) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
        "related_records": related_records,
    })
}

pub(in crate::project::dependency_graph) fn db_related_record<T>(table: &str, record: &T) -> Value
where
    T: Serialize,
{
    json!({
        "table": table,
        "record": db_record_value(record),
    })
}

pub(in crate::project::dependency_graph) fn db_record_value<T>(record: &T) -> Value
where
    T: Serialize,
{
    serde_json::to_value(record)
        .unwrap_or_else(|err| json!({ "serialization_error": err.to_string() }))
}
