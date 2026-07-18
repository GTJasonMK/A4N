use super::records::{chapter_outline_summary, required_chapter_title};
use a4n_core::models::chapter::chapter;
use serde_json::Value;

pub fn collect_analysis_strings(
    chapters: &[chapter::Model],
    metadata_key: &str,
    limit: usize,
) -> Vec<String> {
    let mut values = Vec::<String>::new();
    for ch in chapters {
        let Some(arr) = ch
            .analysis_data
            .as_ref()
            .and_then(|analysis| analysis.get("metadata"))
            .and_then(|metadata| metadata.get(metadata_key))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for item in arr {
            let Some(text) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            if !values.iter().any(|existing| existing == text) {
                values.push(text.to_string());
                if values.len() >= limit {
                    return values;
                }
            }
        }
    }
    values
}

pub fn collect_chapter_summaries(
    chapters: &[chapter::Model],
    limit: usize,
) -> Result<Vec<String>, String> {
    chapters
        .iter()
        .take(limit)
        .map(|ch| {
            Ok(format!(
                "第{}章《{}》：{}",
                ch.chapter_number,
                required_chapter_title(ch)?,
                chapter_outline_summary(ch)?
            ))
        })
        .collect()
}

pub fn collect_part_array_values<F>(
    chapters: &[&chapter::Model],
    limit: usize,
    mut extract: F,
) -> Result<Vec<Value>, String>
where
    F: FnMut(&chapter::Model) -> Result<Option<Value>, String>,
{
    let mut values = Vec::new();
    for chapter in chapters {
        let Some(value) = extract(chapter)? else {
            continue;
        };
        let Value::Array(items) = value else {
            return Err(format!(
                "第{}章分部聚合字段必须是数组",
                chapter.chapter_number
            ));
        };
        for item in items {
            if values.len() >= limit {
                return Ok(values);
            }
            values.push(item);
        }
    }
    Ok(values)
}
