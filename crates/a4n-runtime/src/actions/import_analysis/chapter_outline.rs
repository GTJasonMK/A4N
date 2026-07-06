use a4n_core::models::chapter::{chapter, chapter_outline};
use a4n_services::chapter::analysis::summary_text;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::Value;

pub(super) async fn importable_chapters(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<chapter::Model>, String> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(|e| e.to_string())
        .map(|chapters| {
            chapters
                .into_iter()
                .filter(|ch| !ch.content.trim().is_empty())
                .collect()
        })
}

pub(super) fn required_chapter_title(ch: &chapter::Model) -> Result<&str, String> {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            format!(
                "第{}章缺少章节标题，无法生成导入章节大纲",
                ch.chapter_number
            )
        })
}

fn is_generated_summary_placeholder(summary: &str, chapter_number: i32) -> bool {
    let summary = summary.trim();
    summary.is_empty()
        || summary.contains("摘要待生成")
        || summary.contains("待分析")
        || summary == format!("第{chapter_number}章")
        || (summary.starts_with(&format!("第{chapter_number}章：")) && summary.ends_with('字'))
}

pub(super) fn chapter_outline_summary(ch: &chapter::Model) -> Result<String, String> {
    ch.analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !is_generated_summary_placeholder(summary, ch.chapter_number))
                .map(str::to_string)
        })
        .ok_or_else(|| {
            format!(
                "第{}章缺少结构化摘要，无法生成导入章节大纲",
                ch.chapter_number
            )
        })
}

pub(super) fn analysis_metadata_array(
    analysis: Option<&Value>,
    key: &str,
    chapter_number: i32,
) -> Result<Option<Value>, String> {
    let Some(analysis) = analysis else {
        return Ok(None);
    };
    let Some(metadata) = analysis.get("metadata") else {
        return Ok(None);
    };
    let metadata = metadata
        .as_object()
        .ok_or_else(|| format!("第{chapter_number}章 analysis_data.metadata 必须是对象"))?;
    let Some(value) = metadata.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let values = value
        .as_array()
        .ok_or_else(|| format!("第{chapter_number}章 analysis_data.metadata.{key} 必须是数组"))?;
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Array(values.clone())))
    }
}

pub(super) fn analysis_value_path(
    analysis: Option<&Value>,
    path: &[&str],
    chapter_number: i32,
) -> Result<Option<Value>, String> {
    let Some(mut current) = analysis else {
        return Ok(None);
    };
    for key in path {
        let Some(next) = current.get(*key) else {
            return Ok(None);
        };
        current = next;
    }
    if current.is_null() {
        return Ok(None);
    }
    let values = current.as_array().ok_or_else(|| {
        format!(
            "第{chapter_number}章 analysis_data.{} 必须是数组",
            path.join(".")
        )
    })?;
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Array(values.clone())))
    }
}

pub(super) async fn upsert_outline_from_chapter(
    db: &impl ConnectionTrait,
    ch: &chapter::Model,
) -> Result<chapter_outline::Model, String> {
    let now = Utc::now().naive_utc();
    let title = required_chapter_title(ch)?.to_string();
    let summary = chapter_outline_summary(ch)?;

    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(ch.project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(ch.chapter_number))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    let analysis = ch.analysis_data.as_ref();
    if let Some(existing) = existing {
        let mut active: chapter_outline::ActiveModel = existing.into();
        active.title = Set(Some(title));
        active.summary = Set(Some(summary));
        active.key_scenes = Set(analysis_value_path(
            analysis,
            &["key_events"],
            ch.chapter_number,
        )?);
        active.foreshadowing_setup = Set(analysis_value_path(
            analysis,
            &["foreshadowing", "planted"],
            ch.chapter_number,
        )?);
        active.foreshadowing_payoff = Set(analysis_value_path(
            analysis,
            &["foreshadowing", "resolved"],
            ch.chapter_number,
        )?);
        active.characters_involved = Set(analysis_metadata_array(
            analysis,
            "characters",
            ch.chapter_number,
        )?);
        active.status = Set("completed".to_string());
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())
    } else {
        chapter_outline::ActiveModel {
            project_id: Set(ch.project_id),
            part_id: Set(None),
            chapter_number: Set(ch.chapter_number),
            title: Set(Some(title)),
            summary: Set(Some(summary)),
            beats: Set(None),
            key_scenes: Set(analysis_value_path(
                analysis,
                &["key_events"],
                ch.chapter_number,
            )?),
            pacing_score: Set(None),
            foreshadowing_setup: Set(analysis_value_path(
                analysis,
                &["foreshadowing", "planted"],
                ch.chapter_number,
            )?),
            foreshadowing_payoff: Set(analysis_value_path(
                analysis,
                &["foreshadowing", "resolved"],
                ch.chapter_number,
            )?),
            characters_involved: Set(analysis_metadata_array(
                analysis,
                "characters",
                ch.chapter_number,
            )?),
            word_count_target: Set(Some(ch.word_count)),
            status: Set("completed".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())
    }
}

pub(super) fn collect_analysis_strings(
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

pub(super) fn collect_chapter_summaries(
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

pub(super) fn collect_part_array_values<F>(
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

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    pub(super) fn sample_chapter() -> chapter::Model {
        let now = NaiveDate::from_ymd_opt(2026, 5, 26)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        chapter::Model {
            id: 11,
            project_id: 7,
            outline_id: None,
            chapter_number: 3,
            title: Some("旧城回声".to_string()),
            content: "林岚在旧城发现星潮线索。".to_string(),
            word_count: 13,
            status: "imported".to_string(),
            summary: None,
            style_tags: None,
            analysis_data: Some(sample_analysis()),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_analysis() -> Value {
        json!({
            "metadata": {
                "characters": ["林岚"],
                "locations": ["旧城"],
                "items": ["星潮残片"],
                "tags": ["转折"]
            },
            "summaries": {
                "compressed": "林岚在旧城发现星潮线索。",
                "keywords": ["旧城", "星潮"]
            },
            "key_events": ["发现星潮线索"],
            "foreshadowing": {
                "planted": ["残片发光"],
                "resolved": []
            }
        })
    }

    #[test]
    fn required_chapter_title_rejects_missing_value() {
        let mut chapter = sample_chapter();
        chapter.title = None;

        let err = required_chapter_title(&chapter).expect_err("missing title must fail");

        assert!(err.contains("缺少章节标题"));
    }

    #[test]
    fn chapter_outline_summary_rejects_missing_summary() {
        let mut chapter = sample_chapter();
        chapter.analysis_data = None;
        chapter.summary = None;

        let err = chapter_outline_summary(&chapter).expect_err("missing summary must fail");

        assert!(err.contains("缺少结构化摘要"));
    }

    #[test]
    fn chapter_outline_summary_rejects_generated_word_count_summary() {
        let mut chapter = sample_chapter();
        chapter.analysis_data = None;
        chapter.summary = Some("第3章：13字".to_string());

        let err =
            chapter_outline_summary(&chapter).expect_err("generated word-count summary must fail");

        assert!(err.contains("缺少结构化摘要"));
    }

    #[test]
    fn chapter_outline_summary_accepts_analysis_summary() {
        let chapter = sample_chapter();

        assert_eq!(
            chapter_outline_summary(&chapter).unwrap(),
            "林岚在旧城发现星潮线索。"
        );
    }

    #[test]
    fn collect_chapter_summaries_rejects_missing_title() {
        let mut chapter = sample_chapter();
        chapter.title = Some("   ".to_string());

        let err = collect_chapter_summaries(&[chapter], 80).expect_err("blank title must fail");

        assert!(err.contains("缺少章节标题"));
    }

    #[test]
    fn analysis_value_path_rejects_non_array_leaf() {
        let analysis = json!({
            "key_events": "发现星潮线索"
        });

        let err = analysis_value_path(Some(&analysis), &["key_events"], 3)
            .expect_err("non-array key_events must fail");

        assert!(err.contains("key_events 必须是数组"));
    }

    #[test]
    fn analysis_metadata_array_rejects_non_array_leaf() {
        let analysis = json!({
            "metadata": {
                "characters": "林岚"
            }
        });

        let err = analysis_metadata_array(Some(&analysis), "characters", 3)
            .expect_err("non-array metadata.characters must fail");

        assert!(err.contains("metadata.characters 必须是数组"));
    }

    #[test]
    fn collect_part_array_values_preserves_items_until_limit() {
        let chapter = sample_chapter();
        let values = collect_part_array_values(&[&chapter], 1, |chapter| {
            analysis_value_path(
                chapter.analysis_data.as_ref(),
                &["key_events"],
                chapter.chapter_number,
            )
        })
        .expect("valid key_events should collect");

        assert_eq!(values, vec![json!("发现星潮线索")]);
    }
}
