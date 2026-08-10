use std::collections::HashMap;

use a4n_core::{
    models::{
        chapter::chapter,
        scene::{scene_occurrence, scene_profile, scene_transition},
    },
    rag::RagDataType,
    source_spans::source_text_hash,
};
use a4n_rag_core::{
    metadata::{content_hash, metadata_content_hash},
    vector_store::SearchResult,
};
use chrono::NaiveDateTime;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::chapter_generation::nearest_later_available_chapter;
use crate::rag::{
    RagRecord, scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};

const SOURCE_CHAPTER_ID: &str = "source_chapter_id";
const SOURCE_CHAPTER_CONTENT_SHA256: &str = "source_chapter_content_sha256";
const SOURCE_CHAPTER_UPDATED_AT: &str = "source_chapter_updated_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChapterRagSourceVersion {
    pub(crate) content_sha256: String,
    pub(crate) updated_at: String,
}

pub(crate) fn current_chapter_rag_source_versions(
    chapters: &[chapter::Model],
) -> HashMap<i64, ChapterRagSourceVersion> {
    chapters
        .iter()
        .filter(|chapter| chapter.has_available_content())
        .map(|chapter| {
            (
                chapter.id,
                ChapterRagSourceVersion {
                    content_sha256: source_text_hash(&chapter.content),
                    updated_at: chapter.updated_at.to_string(),
                },
            )
        })
        .collect()
}

pub(crate) fn current_scene_rag_source_hashes(
    project_id: i64,
    profiles: &[scene_profile::Model],
    occurrences: &[scene_occurrence::Model],
    transitions: &[scene_transition::Model],
) -> Result<HashMap<String, String>, String> {
    let names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut sources = HashMap::new();
    for profile in profiles {
        insert_scene_rag_source(&mut sources, scene_profile_rag_record(project_id, profile)?)?;
    }
    for occurrence in occurrences {
        let scene_name = names
            .get(&occurrence.scene_profile_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "场景出场 {} 找不到 scene_profile_id={} 对应场景",
                    occurrence.id, occurrence.scene_profile_id
                )
            })?;
        insert_scene_rag_source(
            &mut sources,
            scene_occurrence_rag_record(project_id, occurrence, scene_name)?,
        )?;
    }
    for transition in transitions {
        let to_scene_name = names
            .get(&transition.to_scene_profile_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "场景位移 {} 找不到 to_scene_profile_id={} 对应场景",
                    transition.id, transition.to_scene_profile_id
                )
            })?;
        let from_scene_name = transition
            .from_scene_profile_id
            .map(|profile_id| {
                names.get(&profile_id).copied().ok_or_else(|| {
                    format!(
                        "场景位移 {} 找不到 from_scene_profile_id={} 对应场景",
                        transition.id, profile_id
                    )
                })
            })
            .transpose()?;
        insert_scene_rag_source(
            &mut sources,
            scene_transition_rag_record(project_id, transition, from_scene_name, to_scene_name)?,
        )?;
    }
    Ok(sources)
}

fn insert_scene_rag_source(
    sources: &mut HashMap<String, String>,
    record: RagRecord,
) -> Result<(), String> {
    let source_hash = content_hash(&record.text).map_err(|error| error.to_string())?;
    if sources
        .insert(record.chunk_id.clone(), source_hash)
        .is_some()
    {
        return Err(format!(
            "当前场景 RAG 来源 chunk_id 重复: {}",
            record.chunk_id
        ));
    }
    Ok(())
}

pub(crate) fn chapter_generation_rag_source_fingerprint(
    chapters: &[chapter::Model],
    chapter_number: i32,
) -> String {
    let mut sources = chapters
        .iter()
        .filter(|chapter| {
            chapter.has_available_content()
                && chapter.chapter_number >= 1
                && chapter.chapter_number < chapter_number
        })
        .collect::<Vec<_>>();
    sources.sort_by_key(|chapter| (chapter.chapter_number, chapter.id));

    let mut hasher = Sha256::new();
    hasher.update(b"a4n:chapter-generation-rag-sources:v1\0");
    hasher.update(chapter_number.to_le_bytes());
    for chapter in sources {
        hasher.update(chapter.id.to_le_bytes());
        hasher.update(chapter.chapter_number.to_le_bytes());
        hasher.update(source_text_hash(&chapter.content).as_bytes());
        hasher.update(chapter.updated_at.to_string().as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

pub(crate) fn chapter_generation_rag_freshness_fingerprint(
    chapters: &[chapter::Model],
    chapter_number: i32,
    query_fingerprint: &str,
) -> String {
    let source_fingerprint = chapter_generation_rag_source_fingerprint(chapters, chapter_number);
    let mut hasher = Sha256::new();
    hasher.update(b"a4n:chapter-generation-rag-freshness:v1\0");
    hasher.update(source_fingerprint.as_bytes());
    hasher.update(query_fingerprint.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Fingerprints every canonical text source that is actually assembled into
/// the chapter prompt, independently of whether optional RAG was available.
/// The nearest later written chapter is bound by its full version even though
/// only a bounded opening excerpt is included in the prompt.
pub(crate) fn chapter_generation_source_fingerprint(
    chapters: &[chapter::Model],
    project_id: i64,
    chapter_number: i32,
    outline_id: Option<i64>,
    outline_updated_at: Option<NaiveDateTime>,
    outline_text: &str,
    context_before: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"a4n:chapter-generation-sources:v2\0");
    hasher.update(project_id.to_le_bytes());
    hasher.update(chapter_number.to_le_bytes());
    hash_optional_i64(&mut hasher, outline_id);
    hash_optional_text(
        &mut hasher,
        outline_updated_at.map(|value| value.to_string()).as_deref(),
    );
    hash_text(&mut hasher, outline_text);
    hash_optional_text(&mut hasher, context_before);
    hash_text(
        &mut hasher,
        &chapter_generation_rag_source_fingerprint(chapters, chapter_number),
    );
    hash_nearest_later_chapter_source(&mut hasher, chapters, chapter_number);
    format!("{:x}", hasher.finalize())
}

fn hash_nearest_later_chapter_source(
    hasher: &mut Sha256,
    chapters: &[chapter::Model],
    chapter_number: i32,
) {
    let Some(chapter) = nearest_later_available_chapter(chapters, chapter_number) else {
        hasher.update([0]);
        return;
    };
    hasher.update([1]);
    hasher.update(chapter.id.to_le_bytes());
    hasher.update(chapter.chapter_number.to_le_bytes());
    hash_text(hasher, &source_text_hash(&chapter.content));
    hash_text(hasher, &chapter.updated_at.to_string());
}

fn hash_optional_i64(hasher: &mut Sha256, value: Option<i64>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hasher.update(value.to_le_bytes());
        }
        None => hasher.update([0]),
    }
}

fn hash_optional_text(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hash_text(hasher, value);
        }
        None => hasher.update([0]),
    }
}

fn hash_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

pub(crate) fn rag_source_version_from_metadata(
    metadata: &Value,
) -> Result<Option<(i64, ChapterRagSourceVersion)>, String> {
    let content_sha256 = metadata.get(SOURCE_CHAPTER_CONTENT_SHA256);
    let updated_at = metadata.get(SOURCE_CHAPTER_UPDATED_AT);
    if content_sha256.is_none() && updated_at.is_none() {
        return Ok(None);
    }
    let chapter_id = metadata_positive_i64(metadata, SOURCE_CHAPTER_ID)
        .ok_or_else(|| "RAG 来源版本缺少有效 source_chapter_id".to_string())?;
    let content_sha256 = required_metadata_str(metadata, SOURCE_CHAPTER_CONTENT_SHA256)?;
    let updated_at = required_metadata_str(metadata, SOURCE_CHAPTER_UPDATED_AT)?;
    Ok(Some((
        chapter_id,
        ChapterRagSourceVersion {
            content_sha256,
            updated_at,
        },
    )))
}

pub(crate) fn expected_chapter_rag_sources<'a>(
    metadata: impl IntoIterator<Item = &'a Value>,
) -> Result<HashMap<i64, ChapterRagSourceVersion>, String> {
    let mut expected = HashMap::new();
    for metadata in metadata {
        let Some((chapter_id, source)) = rag_source_version_from_metadata(metadata)? else {
            continue;
        };
        if let Some(previous) = expected.insert(chapter_id, source.clone())
            && previous != source
        {
            return Err(format!(
                "同一批 RAG 记录包含冲突的章节来源版本: chapter_id={chapter_id}"
            ));
        }
    }
    Ok(expected)
}

pub(crate) fn ensure_chapter_rag_sources_current(
    expected: &HashMap<i64, ChapterRagSourceVersion>,
    current: &HashMap<i64, ChapterRagSourceVersion>,
) -> Result<(), String> {
    for (chapter_id, expected_source) in expected {
        if current.get(chapter_id) != Some(expected_source) {
            return Err(format!(
                "章节来源已变化，拒绝写入旧 RAG 记录: chapter_id={chapter_id}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn filter_current_chapter_rag_results(
    results: Vec<SearchResult>,
    current_sources: &HashMap<i64, ChapterRagSourceVersion>,
) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(
            |result| match rag_source_version_from_metadata(&result.metadata) {
                Ok(Some((chapter_id, source))) => current_sources
                    .get(&chapter_id)
                    .is_some_and(|current| current == &source),
                Ok(None) => !requires_chapter_source_version(result),
                Err(_) => false,
            },
        )
        .collect()
}

pub(crate) fn filter_current_non_chapter_rag_results(
    results: Vec<SearchResult>,
    current_sources: &HashMap<String, String>,
) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(|result| {
            if result.id.starts_with("chapter:") {
                return true;
            }
            current_sources.get(&result.id).is_some_and(|current| {
                metadata_content_hash(&result.metadata) == Some(current.as_str())
            })
        })
        .collect()
}

fn requires_chapter_source_version(result: &SearchResult) -> bool {
    result.id.starts_with("chapter:")
        || result
            .metadata
            .get("data_type")
            .and_then(Value::as_str)
            .is_some_and(|data_type| {
                matches!(
                    RagDataType::parse(data_type),
                    Some(
                        RagDataType::Chapter
                            | RagDataType::ChapterSummary
                            | RagDataType::ChapterMetadata
                            | RagDataType::KeyEvent
                    )
                )
            })
}

fn required_metadata_str(metadata: &Value, key: &str) -> Result<String, String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("RAG 来源版本缺少有效 {key}"))
}

fn metadata_positive_i64(metadata: &Value, key: &str) -> Option<i64> {
    let value = metadata.get(key)?;
    let parsed = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))?;
    (parsed > 0).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::chapter::chapter;
    use chrono::NaiveDateTime;
    use serde_json::json;

    fn chapter(id: i64, chapter_number: i32, content: &str) -> chapter::Model {
        let timestamp = NaiveDateTime::parse_from_str("2026-08-08 10:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("valid timestamp");
        chapter::Model {
            id,
            project_id: 1,
            outline_id: None,
            chapter_number,
            title: Some(format!("第{chapter_number}章")),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: chapter::STATUS_COMPLETED.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    fn result(data_type: &str, metadata: Value) -> SearchResult {
        let mut metadata = metadata.as_object().cloned().expect("metadata object");
        metadata.insert("data_type".to_string(), json!(data_type));
        SearchResult {
            id: "record".to_string(),
            content: "content".to_string(),
            score: 1.0,
            vector_score: None,
            bm25_score: None,
            metadata: Value::Object(metadata),
        }
    }

    #[test]
    fn chapter_results_require_the_current_source_version() {
        let current = HashMap::from([(
            7,
            ChapterRagSourceVersion {
                content_sha256: "current".to_string(),
                updated_at: "2026-08-08 10:00:00".to_string(),
            },
        )]);
        let valid = result(
            RagDataType::Chapter.as_str(),
            json!({
                "source_chapter_id": 7,
                "source_chapter_content_sha256": "current",
                "source_chapter_updated_at": "2026-08-08 10:00:00",
            }),
        );
        let stale = result(
            RagDataType::ChapterSummary.as_str(),
            json!({
                "source_chapter_id": 7,
                "source_chapter_content_sha256": "old",
                "source_chapter_updated_at": "2026-08-08 09:00:00",
            }),
        );
        let legacy = result(RagDataType::KeyEvent.as_str(), json!({}));

        let filtered = filter_current_chapter_rag_results(vec![valid, stale, legacy], &current);

        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].metadata["source_chapter_content_sha256"],
            "current"
        );
    }

    #[test]
    fn non_chapter_records_without_source_versions_remain_searchable() {
        let result = result(RagDataType::Character.as_str(), json!({}));

        assert_eq!(
            filter_current_chapter_rag_results(vec![result], &HashMap::new()).len(),
            1
        );
    }

    #[test]
    fn non_chapter_records_require_the_current_canonical_hash() {
        let current = HashMap::from([
            ("blueprint:1:7".to_string(), "blueprint-current".to_string()),
            (
                "character:1:10:identity".to_string(),
                "character-current".to_string(),
            ),
        ]);
        let mut existing_blueprint = result(
            RagDataType::Blueprint.as_str(),
            json!({"content_hash": "blueprint-current"}),
        );
        existing_blueprint.id = "blueprint:1:7".to_string();
        let mut changed = result(
            RagDataType::Blueprint.as_str(),
            json!({"content_hash": "blueprint-old"}),
        );
        changed.id = "blueprint:1:7".to_string();
        let mut deleted_character = result(
            RagDataType::Character.as_str(),
            json!({"content_hash": "character-deleted"}),
        );
        deleted_character.id = "character:1:9:identity".to_string();
        let mut existing_character = result(
            RagDataType::Character.as_str(),
            json!({"content_hash": "character-current"}),
        );
        existing_character.id = "character:1:10:identity".to_string();
        let mut malformed = result(RagDataType::Blueprint.as_str(), json!({}));
        malformed.id = "blueprint:1:7".to_string();
        let mut chapter = result(RagDataType::Chapter.as_str(), json!({}));
        chapter.id = "chapter:1:4:p1".to_string();

        let filtered = filter_current_non_chapter_rag_results(
            vec![
                existing_blueprint,
                changed,
                deleted_character,
                existing_character,
                malformed,
                chapter,
            ],
            &current,
        );

        assert_eq!(
            filtered
                .iter()
                .map(|result| result.id.as_str())
                .collect::<Vec<_>>(),
            vec!["blueprint:1:7", "character:1:10:identity", "chapter:1:4:p1"]
        );
    }

    #[test]
    fn generation_source_fingerprint_tracks_prior_available_chapters_only() {
        let first = chapter(1, 1, "第一版正文");
        let future = chapter(3, 3, "未来章节");
        let current =
            chapter_generation_rag_source_fingerprint(&[future.clone(), first.clone()], 3);

        assert_eq!(
            current,
            chapter_generation_rag_source_fingerprint(&[first.clone()], 3)
        );

        let changed = chapter(1, 1, "第二版正文");
        assert_ne!(
            current,
            chapter_generation_rag_source_fingerprint(&[changed], 3)
        );
    }

    #[test]
    fn generation_prompt_source_fingerprint_tracks_non_rag_context() {
        let chapters = vec![chapter(1, 1, "第一章正文")];
        let current = chapter_generation_source_fingerprint(
            &chapters,
            1,
            2,
            None,
            None,
            "## 当前任务\n按简报续写",
            Some("角色仍在城门外"),
        );

        assert_ne!(
            current,
            chapter_generation_source_fingerprint(
                &chapters,
                1,
                2,
                None,
                None,
                "## 当前任务\n按简报续写",
                Some("角色已经进入城内"),
            )
        );
        assert_ne!(
            current,
            chapter_generation_source_fingerprint(
                &[chapter(1, 1, "第一章正文已修改")],
                1,
                2,
                None,
                None,
                "## 当前任务\n按简报续写",
                Some("角色仍在城门外"),
            )
        );
    }

    #[test]
    fn generation_prompt_source_fingerprint_tracks_the_full_nearest_later_chapter_only() {
        let shared_prefix = "后".repeat(600);
        let nearest = chapter(10, 10, &format!("{shared_prefix}甲"));
        let changed_nearest = chapter(10, 10, &format!("{shared_prefix}乙"));
        let farther = chapter(12, 12, "更远后续正文第一版");
        let changed_farther = chapter(12, 12, "更远后续正文第二版");
        let current = chapter_generation_source_fingerprint(
            &[nearest.clone(), farther],
            1,
            5,
            None,
            None,
            "## 当前任务\n补写第五章",
            Some("相同的后续正文截取片段"),
        );

        assert_ne!(
            current,
            chapter_generation_source_fingerprint(
                &[changed_nearest, changed_farther.clone()],
                1,
                5,
                None,
                None,
                "## 当前任务\n补写第五章",
                Some("相同的后续正文截取片段"),
            )
        );
        assert_eq!(
            current,
            chapter_generation_source_fingerprint(
                &[nearest, changed_farther],
                1,
                5,
                None,
                None,
                "## 当前任务\n补写第五章",
                Some("相同的后续正文截取片段"),
            )
        );
    }
}
