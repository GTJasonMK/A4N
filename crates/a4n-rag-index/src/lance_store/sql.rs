use a4n_rag_core::vector_store::{DeleteFilter, SearchFilter};

/// SQL 字符串转义：单引号变两个单引号，避免注入。
pub(super) fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}

/// SearchFilter -> SQL WHERE 子句。
pub(super) fn search_filter_sql(filter: Option<&SearchFilter>) -> Option<String> {
    let f = filter?;
    let mut parts: Vec<String> = Vec::new();

    if let Some(eq) = &f.equals {
        for (k, v) in eq {
            match k.as_str() {
                "project_id" => {
                    if let Some(n) = v.as_i64() {
                        parts.push(format!("project_id = {n}"));
                    }
                }
                "data_type" => {
                    if let Some(s) = v.as_str() {
                        parts.push(format!("data_type = '{}'", escape_sql(s)));
                    }
                }
                "chapter_number" => {
                    if let Some(n) = v.as_i64() {
                        parts.push(format!("chapter_number = {n}"));
                    }
                }
                _ => {}
            }
        }
    }
    if let Some((from, to)) = f.chapter_range {
        parts.push(format!(
            "chapter_number >= {from} AND chapter_number <= {to}"
        ));
    }
    if let Some(types) = &f.data_types
        && !types.is_empty()
    {
        let quoted: Vec<String> = types
            .iter()
            .map(|t| format!("'{}'", escape_sql(t)))
            .collect();
        parts.push(format!("data_type IN ({})", quoted.join(", ")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" AND "))
    }
}

pub(super) fn search_filter_fully_expressed_by_sql(filter: &SearchFilter) -> bool {
    if filter.data_types.as_ref().is_some_and(Vec::is_empty) {
        return false;
    }

    let Some(eq) = &filter.equals else {
        return true;
    };

    eq.iter().all(|(key, value)| match key.as_str() {
        "project_id" | "chapter_number" => value.as_i64().is_some(),
        "data_type" => value.as_str().is_some(),
        _ => false,
    })
}

/// DeleteFilter -> SQL WHERE 子句。
pub(super) fn delete_filter_sql(filter: &DeleteFilter) -> String {
    let mut id_clauses = Vec::new();
    if !filter.ids.is_empty() {
        id_clauses.push(ids_in_clause(&filter.ids));
    }
    let prefix_clauses = filter
        .id_prefixes
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| format!("id LIKE '{}%'", escape_sql(p)))
        .collect::<Vec<_>>();
    if !prefix_clauses.is_empty() {
        id_clauses.push(format!("({})", prefix_clauses.join(" OR ")));
    }
    if !id_clauses.is_empty() {
        return id_clauses.join(" OR ");
    }

    let mut metadata_parts = vec![format!("project_id = {}", filter.project_id)];
    if let Some(dt) = &filter.data_type {
        metadata_parts.push(format!("data_type = '{}'", escape_sql(dt)));
    }
    if let Some(cn) = filter.chapter_number {
        metadata_parts.push(format!("chapter_number = {cn}"));
    }
    format!("({})", metadata_parts.join(" AND "))
}

/// 通过 SQL IN 子句构造 id 列表。
pub(super) fn ids_in_clause(ids: &[String]) -> String {
    let quoted: Vec<String> = ids
        .iter()
        .map(|id| format!("'{}'", escape_sql(id)))
        .collect();
    format!("id IN ({})", quoted.join(", "))
}
