use std::collections::HashSet;
use std::ops::Bound;

use tantivy::Term;
use tantivy::query::{BooleanQuery, Occur, Query, RangeQuery, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};

use super::collection::CollectionIndex;
use super::{FIELD_CHAPTER_NUMBER, FIELD_PROJECT_ID, QUERY_NGRAM_LIMIT};
use crate::rag::vector_store::SearchFilter;

pub(super) fn collection_excluded_by_filter(
    collection: &str,
    filters: Option<&SearchFilter>,
) -> bool {
    let Some(filter) = filters else {
        return false;
    };
    let Some(equals) = &filter.equals else {
        return false;
    };
    equals.get("project_id").is_some_and(|value| {
        let Some(project_id) = value.as_i64() else {
            return true;
        };
        collection
            .parse::<i64>()
            .is_ok_and(|collection_project_id| collection_project_id != project_id)
    })
}

pub(super) fn build_search_query(
    ci: &CollectionIndex,
    terms: Vec<Term>,
    filters: Option<&SearchFilter>,
) -> Option<Box<dyn Query>> {
    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(
        Occur::Must,
        Box::new(BooleanQuery::new_multiterms_query(terms)),
    )];

    if let Some(filter) = filters {
        if let Some(project_id) = project_id_filter_value(filter) {
            clauses.push((
                Occur::Must,
                Box::new(RangeQuery::new_i64_bounds(
                    FIELD_PROJECT_ID.to_string(),
                    Bound::Included(project_id),
                    Bound::Included(project_id),
                )),
            ));
        }

        if let Some(data_types) = data_type_filter_values(filter) {
            if data_types.is_empty() {
                return None;
            }
            clauses.push((
                Occur::Must,
                Box::new(disjunction_for_terms(
                    data_types
                        .iter()
                        .map(|data_type| Term::from_field_text(ci.field_data_type, data_type)),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some((min, max)) = chapter_number_bounds(filter) {
            if min > max {
                return None;
            }
            clauses.push((
                Occur::Must,
                Box::new(RangeQuery::new_i64_bounds(
                    FIELD_CHAPTER_NUMBER.to_string(),
                    Bound::Included(min),
                    Bound::Included(max),
                )),
            ));
        }
    }

    if clauses.len() == 1 {
        Some(clauses.pop().expect("non-empty clauses").1)
    } else {
        Some(Box::new(BooleanQuery::new(clauses)))
    }
}

fn disjunction_for_terms(
    terms: impl IntoIterator<Item = Term>,
    index_option: IndexRecordOption,
) -> BooleanQuery {
    BooleanQuery::new(
        terms
            .into_iter()
            .map(|term| {
                (
                    Occur::Should,
                    Box::new(TermQuery::new(term, index_option)) as Box<dyn Query>,
                )
            })
            .collect(),
    )
}

fn project_id_filter_value(filter: &SearchFilter) -> Option<i64> {
    filter
        .equals
        .as_ref()
        .and_then(|equals| equals.get("project_id"))
        .and_then(|value| value.as_i64())
}

fn data_type_filter_values(filter: &SearchFilter) -> Option<Vec<String>> {
    let mut values = filter.data_types.as_ref().cloned();

    let Some(equals) = &filter.equals else {
        return values;
    };
    let Some(value) = equals.get("data_type") else {
        return values;
    };
    let Some(required_data_type) = value.as_str().map(str::to_string) else {
        return Some(Vec::new());
    };

    match values.as_mut() {
        Some(existing) => {
            existing.retain(|data_type| data_type == &required_data_type);
            values
        }
        None => Some(vec![required_data_type]),
    }
}

fn chapter_number_bounds(filter: &SearchFilter) -> Option<(i64, i64)> {
    let mut bounds = filter
        .chapter_range
        .map(|(min, max)| (i64::from(min), i64::from(max)));

    let Some(equals) = &filter.equals else {
        return bounds;
    };
    let Some(value) = equals.get("chapter_number") else {
        return bounds;
    };
    let Some(required_chapter) = value.as_i64() else {
        return Some((1, 0));
    };

    match bounds.as_mut() {
        Some((min, max)) => {
            *min = (*min).max(required_chapter);
            *max = (*max).min(required_chapter);
            bounds
        }
        None => Some((required_chapter, required_chapter)),
    }
}

pub(super) fn normalized_search_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_space = true;
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            normalized.extend(ch.to_lowercase());
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }
    normalized.trim().to_string()
}

pub(super) fn ngram_query_terms(field: Field, query_text: &str) -> Vec<Term> {
    let normalized = normalized_search_text(query_text);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut seen = HashSet::new();
    let mut tokens = Vec::new();
    for segment in normalized.split_whitespace() {
        push_segment_ngrams(segment, &mut tokens, &mut seen);
    }

    if tokens.len() > QUERY_NGRAM_LIMIT {
        tokens = downsample_terms(tokens, QUERY_NGRAM_LIMIT);
    }

    tokens
        .into_iter()
        .map(|token| Term::from_field_text(field, &token))
        .collect()
}

fn push_segment_ngrams(segment: &str, tokens: &mut Vec<String>, seen: &mut HashSet<String>) {
    let chars = segment.chars().collect::<Vec<_>>();
    for n in (2..=3).rev() {
        if chars.len() < n {
            continue;
        }
        for window in chars.windows(n) {
            let token = window.iter().collect::<String>();
            if seen.insert(token.clone()) {
                tokens.push(token);
            }
        }
    }
}

fn downsample_terms(tokens: Vec<String>, limit: usize) -> Vec<String> {
    if tokens.len() <= limit {
        return tokens;
    }
    let len = tokens.len();
    (0..limit)
        .map(|index| {
            let source_index = index.saturating_mul(len) / limit;
            tokens[source_index.min(len - 1)].clone()
        })
        .collect()
}
