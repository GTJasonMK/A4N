use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::{
    ChapterAnalysisEntry, ChapterStoryEventEntry, ChapterStoryEventForeshadowingCounts,
};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Set, Statement};
use serde_json::{Map, Value, json};
use std::time::Instant;

const CHAPTER_COUNT: i32 = 1_000;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

fn repeated(text: &str, count: usize) -> String {
    text.repeat(count)
}

fn analysis_data(chapter_number: i32) -> Value {
    let characters = ["林舟", "沈青", "陈墨"];
    let mut character_states = Map::new();
    for (index, character) in characters.iter().enumerate() {
        character_states.insert(
            (*character).into(),
            json!({
                "location": repeated(&format!("第{chapter_number}章所在地点记录。"), 4),
                "status": repeated("正在追查账本留下的线索。", 4),
                "changes": [
                    repeated("发现一项新的证据。", 3),
                    repeated("与同伴核对事件经过。", 3),
                ],
                "emotional_state": format!("保持警觉 {index}"),
            }),
        );
    }

    json!({
        "metadata": {
            "characters": characters,
            "locations": ["旧城", "河岸"],
            "items": ["账本", repeated("未用物件标签。", 8)],
            "tags": ["调查", repeated("未用分类标签。", 8)],
            "tone": "紧张",
            "timeline_marker": format!("第{chapter_number}天"),
        },
        "summaries": {
            "compressed": format!("第{chapter_number}章调查账本的来历"),
            "one_line": "账本现身",
            "keywords": ["账本", "调查"],
        },
        "character_states": character_states,
        "world_facts": [
            {
                "field": "history_timeline",
                "subject": "旧城历史",
                "statement": repeated("旧城曾发生一场影响组织势力变化的冲突。", 3),
                "assertion": "reported",
                "source_span_ids": [format!("chapter:{chapter_number}:span:1")],
                "evidence": [repeated("正文支持旧城历史变化。", 3)],
            },
            {
                "field": "core_rules",
                "subject": "账本记录规则",
                "statement": repeated("账本会记录交易双方以及日期。", 3),
                "assertion": "observed",
                "source_span_ids": [format!("chapter:{chapter_number}:span:2")],
                "evidence": [repeated("账本内容显示交易信息。", 3)],
            },
        ],
        "foreshadowing": {
            "planted": [{
                "description": repeated("异常来客留下未解线索。", 3),
                "original_text": repeated("原始正文伏笔片段。", 8),
                "category": "plot_twist",
                "priority": "high",
                "related_entities": characters,
            }],
            "resolved": [{
                "id": format!("foreshadowing-{chapter_number}"),
                "resolution": repeated("线索在本章得到解释。", 4),
                "source_span_ids": [format!("chapter:{chapter_number}:span:3")],
            }],
            "tensions": [repeated("尚未解决的矛盾。", 4)],
            "progressed": [{
                "id": format!("progressed-{chapter_number}"),
                "status": "partially_revealed",
                "resolution": repeated("伏笔推进说明。", 3),
                "source_span_ids": [format!("chapter:{chapter_number}:span:4")],
            }],
        },
        "key_events": (0..5)
            .map(|event_number| json!({
                "description": format!(
                    "线索{event_number}：第{chapter_number}章发现账本记录 {}",
                    repeated("灯火照亮河岸，林舟追寻来客留下的足迹。", 5),
                ),
                "type": "discovery",
                "importance": "medium",
                "involved_characters": [characters[event_number % characters.len()]],
            }))
            .collect::<Vec<_>>(),
    })
}

async fn create_fixture(db: &DatabaseConnection) -> i64 {
    let project = create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "故事事件投影性能".into(),
            description: String::new(),
            project_type: "novel_long".into(),
        },
    )
    .await
    .expect("benchmark project must be created");
    let now = Utc::now().naive_utc();
    let chapters = (1..=CHAPTER_COUNT)
        .map(|chapter_number| chapter::ActiveModel {
            project_id: Set(project.id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("调查线索 {chapter_number}"))),
            summary: Set(Some(format!("第 {chapter_number} 章推进摘要"))),
            content: Set("正文不属于故事事件列表响应".into()),
            word_count: Set(17),
            status: Set(chapter::STATUS_DRAFT.into()),
            analysis_data: Set((chapter_number % 10 != 0).then(|| analysis_data(chapter_number))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    for chunk in chapters.chunks(250) {
        chapter::Entity::insert_many(chunk.iter().cloned())
            .exec(db)
            .await
            .expect("benchmark chapters must be inserted");
    }
    db.execute_unprepared("ANALYZE chapter")
        .await
        .expect("chapter statistics must be refreshed");
    project.id
}

fn select_fields(value: Option<&Value>, fields: &[&str]) -> Value {
    match value {
        Some(Value::Object(source)) => Value::Object(
            fields
                .iter()
                .map(|field| {
                    (
                        (*field).into(),
                        source.get(*field).cloned().unwrap_or(Value::Null),
                    )
                })
                .collect(),
        ),
        Some(value) => value.clone(),
        None => Value::Null,
    }
}

fn project_analysis_data(analysis: Option<&Value>) -> Option<Value> {
    let root = analysis?.as_object()?;
    Some(json!({
        "metadata": select_fields(root.get("metadata"), &[
            "characters", "locations", "tone", "timeline_marker",
        ]),
        "summaries": select_fields(root.get("summaries"), &[
            "compressed", "summary", "short", "one_line", "keywords",
        ]),
        "key_events": root.get("key_events").cloned().unwrap_or(Value::Null),
    }))
}

fn array_count(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

fn projected_entry(entry: &ChapterAnalysisEntry) -> ChapterStoryEventEntry {
    let root = entry.analysis_data.as_ref().and_then(Value::as_object);
    let foreshadowing = root.and_then(|value| value.get("foreshadowing"));
    ChapterStoryEventEntry {
        id: entry.id,
        chapter_number: entry.chapter_number,
        title: entry.title.clone(),
        summary: entry.summary.clone(),
        analysis_data: project_analysis_data(entry.analysis_data.as_ref()),
        foreshadowing_counts: ChapterStoryEventForeshadowingCounts {
            planted: array_count(foreshadowing.and_then(|value| value.get("planted"))),
            resolved: array_count(foreshadowing.and_then(|value| value.get("resolved"))),
            tensions: array_count(foreshadowing.and_then(|value| value.get("tensions"))),
        },
    }
}

async fn projected_entries(
    db: &DatabaseConnection,
    project_id: i64,
) -> Vec<ChapterStoryEventEntry> {
    const QUERY: &str = r#"
        SELECT id, chapter_number, title, summary,
            CASE
                WHEN analysis_data IS NULL THEN NULL
                WHEN json_typeof(analysis_data) <> 'object' THEN analysis_data
                ELSE json_build_object(
                    'metadata', CASE
                        WHEN json_typeof(analysis_data->'metadata') = 'object' THEN json_build_object(
                            'characters', analysis_data #> '{metadata,characters}',
                            'locations', analysis_data #> '{metadata,locations}',
                            'tone', analysis_data #> '{metadata,tone}',
                            'timeline_marker', analysis_data #> '{metadata,timeline_marker}'
                        )
                        ELSE analysis_data->'metadata'
                    END,
                    'summaries', CASE
                        WHEN json_typeof(analysis_data->'summaries') = 'object' THEN json_build_object(
                            'compressed', analysis_data #> '{summaries,compressed}',
                            'summary', analysis_data #> '{summaries,summary}',
                            'short', analysis_data #> '{summaries,short}',
                            'one_line', analysis_data #> '{summaries,one_line}',
                            'keywords', analysis_data #> '{summaries,keywords}'
                        )
                        ELSE analysis_data->'summaries'
                    END,
                    'key_events', analysis_data->'key_events'
                )
            END AS analysis_data,
            CASE WHEN json_typeof(analysis_data #> '{foreshadowing,planted}') = 'array'
                THEN json_array_length(analysis_data #> '{foreshadowing,planted}') ELSE 0 END AS planted_count,
            CASE WHEN json_typeof(analysis_data #> '{foreshadowing,resolved}') = 'array'
                THEN json_array_length(analysis_data #> '{foreshadowing,resolved}') ELSE 0 END AS resolved_count,
            CASE WHEN json_typeof(analysis_data #> '{foreshadowing,tensions}') = 'array'
                THEN json_array_length(analysis_data #> '{foreshadowing,tensions}') ELSE 0 END AS tension_count
        FROM chapter
        WHERE project_id = $1 AND status <> 'deleted'
        ORDER BY chapter_number ASC
    "#;
    let statement =
        Statement::from_sql_and_values(DatabaseBackend::Postgres, QUERY, [project_id.into()]);
    db.query_all(statement)
        .await
        .expect("narrow story event rows must load")
        .into_iter()
        .map(|row| {
            let planted_count = usize::try_from(
                row.try_get::<i32>("", "planted_count")
                    .expect("planted count must be selected"),
            )
            .expect("planted count must be non-negative");
            let resolved_count = usize::try_from(
                row.try_get::<i32>("", "resolved_count")
                    .expect("resolved count must be selected"),
            )
            .expect("resolved count must be non-negative");
            let tension_count = usize::try_from(
                row.try_get::<i32>("", "tension_count")
                    .expect("tension count must be selected"),
            )
            .expect("tension count must be non-negative");
            ChapterStoryEventEntry {
                id: row.try_get("", "id").expect("id must be selected"),
                chapter_number: row
                    .try_get("", "chapter_number")
                    .expect("chapter number must be selected"),
                title: row.try_get("", "title").expect("title must be selected"),
                summary: row
                    .try_get("", "summary")
                    .expect("summary must be selected"),
                analysis_data: row
                    .try_get("", "analysis_data")
                    .expect("analysis projection must be selected"),
                foreshadowing_counts: ChapterStoryEventForeshadowingCounts {
                    planted: planted_count,
                    resolved: resolved_count,
                    tensions: tension_count,
                },
            }
        })
        .collect()
}

async fn in_place_entries(db: &DatabaseConnection, project_id: i64) -> Vec<ChapterStoryEventEntry> {
    a4n_db::chapter_content::chapter_story_events_for_project(db, project_id)
        .await
        .expect("in-place story event entries must load")
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

async fn timed_full(db: &DatabaseConnection, project_id: i64) -> (f64, usize) {
    let started = Instant::now();
    let rows = a4n_db::chapter_content::chapter_analyses_for_project(db, project_id)
        .await
        .expect("full chapter analyses must load");
    let bytes = serde_json::to_vec(&rows)
        .expect("full response must serialize")
        .len();
    (started.elapsed().as_secs_f64() * 1_000.0, bytes)
}

async fn timed_projected(
    db: &DatabaseConnection,
    project_id: i64,
) -> (f64, usize, Vec<ChapterStoryEventEntry>) {
    let started = Instant::now();
    let rows = projected_entries(db, project_id).await;
    let bytes = serde_json::to_vec(&rows)
        .expect("projected response must serialize")
        .len();
    (started.elapsed().as_secs_f64() * 1_000.0, bytes, rows)
}

async fn timed_in_place(
    db: &DatabaseConnection,
    project_id: i64,
) -> (f64, usize, Vec<ChapterStoryEventEntry>) {
    let started = Instant::now();
    let rows = in_place_entries(db, project_id).await;
    let bytes = serde_json::to_vec(&rows)
        .expect("in-place projected response must serialize")
        .len();
    (started.elapsed().as_secs_f64() * 1_000.0, bytes, rows)
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_chapter_story_event_analysis_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_fixture(&db).await;
    let full = a4n_db::chapter_content::chapter_analyses_for_project(&db, project_id)
        .await
        .expect("full chapter analyses must load");
    let expected = full.iter().map(projected_entry).collect::<Vec<_>>();
    let expected_in_place = full
        .clone()
        .into_iter()
        .map(ChapterStoryEventEntry::from)
        .collect::<Vec<_>>();
    let actual = projected_entries(&db, project_id).await;
    assert_eq!(actual, expected);
    assert_eq!(in_place_entries(&db, project_id).await, expected_in_place);
    assert_eq!(actual.len(), CHAPTER_COUNT as usize);
    assert_eq!(
        actual
            .iter()
            .filter(|row| row.analysis_data.is_some())
            .count(),
        900
    );

    for _ in 0..WARMUPS {
        let _ = timed_full(&db, project_id).await;
        let (_, _, projected) = timed_projected(&db, project_id).await;
        assert_eq!(projected, expected);
        let (_, _, projected) = timed_in_place(&db, project_id).await;
        assert_eq!(projected, expected_in_place);
    }
    let mut full_samples = Vec::with_capacity(SAMPLES);
    let mut projected_samples = Vec::with_capacity(SAMPLES);
    let mut in_place_samples = Vec::with_capacity(SAMPLES);
    let mut full_bytes = 0;
    let mut projected_bytes = 0;
    let mut in_place_bytes = 0;
    for sample in 0..SAMPLES {
        if sample % 2 == 0 {
            let (elapsed, bytes) = timed_full(&db, project_id).await;
            full_samples.push(elapsed);
            full_bytes = bytes;
            let (elapsed, bytes, rows) = timed_projected(&db, project_id).await;
            projected_samples.push(elapsed);
            projected_bytes = bytes;
            assert_eq!(rows, expected);
            let (elapsed, bytes, rows) = timed_in_place(&db, project_id).await;
            in_place_samples.push(elapsed);
            in_place_bytes = bytes;
            assert_eq!(rows, expected_in_place);
        } else {
            let (elapsed, bytes, rows) = timed_in_place(&db, project_id).await;
            in_place_samples.push(elapsed);
            in_place_bytes = bytes;
            assert_eq!(rows, expected_in_place);
            let (elapsed, bytes, rows) = timed_projected(&db, project_id).await;
            projected_samples.push(elapsed);
            projected_bytes = bytes;
            assert_eq!(rows, expected);
            let (elapsed, bytes) = timed_full(&db, project_id).await;
            full_samples.push(elapsed);
            full_bytes = bytes;
        }
    }
    println!(
        "{{\"chapters\":{CHAPTER_COUNT},\"analyzed\":900,\"warmups\":{WARMUPS},\"samples\":{SAMPLES},\"full_bytes\":{full_bytes},\"sql_projected_bytes\":{projected_bytes},\"in_place_bytes\":{in_place_bytes},\"full_p50_ms\":{:.3},\"full_p95_ms\":{:.3},\"sql_projected_p50_ms\":{:.3},\"sql_projected_p95_ms\":{:.3},\"in_place_p50_ms\":{:.3},\"in_place_p95_ms\":{:.3}}}",
        percentile(&full_samples, 0.50),
        percentile(&full_samples, 0.95),
        percentile(&projected_samples, 0.50),
        percentile(&projected_samples, 0.95),
        percentile(&in_place_samples, 0.50),
        percentile(&in_place_samples, 0.95),
    );
}
