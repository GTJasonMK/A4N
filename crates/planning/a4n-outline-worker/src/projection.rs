use std::collections::BTreeMap;

use a4n_api_contract::outline::{
    ChapterOutlineSummary, DeleteLatestOutlineRecordsInput, OutlineGenerationProgressSummary,
    PartOutlineSummary, UpdateChapterOutlineInput,
};
use a4n_db::project::OutlineGenerationProgressRows;
use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord, chapter_outline_status};

use super::WorkerFailure;

pub(super) fn chapter_outline_summary(
    outline: &ChapterOutlineRecord,
    source_kind: Option<&String>,
) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        source_kind: source_kind.cloned(),
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}

pub(super) fn part_outline_summary(part: &PartOutlineRecord) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

pub(super) fn generation_progress_summary(
    rows: &OutlineGenerationProgressRows,
) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
    // The DB read orders chapter outlines by number, so adjacent checks and this cursor stay sorted.
    let mut completed_chapters = Vec::with_capacity(rows.chapter_outlines.len());
    let mut completed_chapter_outlines = 0_u64;
    let mut previous_chapter_number = None;
    for outline in &rows.chapter_outlines {
        if !chapter_outline_status::is_valid_status(&outline.status) {
            return Err(WorkerFailure::conflict(format!(
                "章节大纲 {} 状态无效：{}",
                outline.id, outline.status
            )));
        }
        if previous_chapter_number == Some(outline.chapter_number) {
            return Err(WorkerFailure::conflict(format!(
                "章节号 {} 存在重复章节大纲",
                outline.chapter_number
            )));
        }
        previous_chapter_number = Some(outline.chapter_number);
        if outline.status == chapter_outline_status::STATUS_COMPLETED {
            completed_chapter_outlines += 1;
            completed_chapters.push(outline.chapter_number);
        }
    }

    let mut ranges = BTreeMap::<i32, i32>::new();
    for part in &rows.parts {
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(WorkerFailure::conflict(format!(
                "分部 {} 章节范围无效：{}..={}",
                part.id, part.start_chapter, part.end_chapter
            )));
        }
        let overlap = ranges
            .range(..part.start_chapter)
            .next_back()
            .and_then(|(_, end)| (*end >= part.start_chapter).then_some(part.start_chapter))
            .or_else(|| {
                ranges
                    .range(part.start_chapter..=part.end_chapter)
                    .next()
                    .map(|(&start, _)| start)
            });
        if let Some(chapter_number) = overlap {
            return Err(WorkerFailure::conflict(format!(
                "章节号 {} 被多个分部范围覆盖",
                chapter_number
            )));
        }
        ranges.insert(part.start_chapter, part.end_chapter);
    }

    let mut completed_parts = 0_u64;
    let mut completed_chapter_index = 0;
    for (start_chapter, end_chapter) in ranges {
        while completed_chapters
            .get(completed_chapter_index)
            .is_some_and(|chapter_number| *chapter_number < start_chapter)
        {
            completed_chapter_index += 1;
        }
        let first_completed_chapter_index = completed_chapter_index;
        while completed_chapters
            .get(completed_chapter_index)
            .is_some_and(|chapter_number| *chapter_number <= end_chapter)
        {
            completed_chapter_index += 1;
        }
        let chapter_count = i64::from(end_chapter) - i64::from(start_chapter) + 1;
        if (completed_chapter_index - first_completed_chapter_index) as i64 == chapter_count {
            completed_parts += 1;
        }
    }

    Ok(OutlineGenerationProgressSummary {
        total_parts: rows.parts.len() as u64,
        completed_parts,
        total_chapter_outlines: rows.chapter_outlines.len() as u64,
        completed_chapter_outlines,
    })
}

#[cfg(test)]
mod performance_tests {
    use std::{
        collections::{HashMap, HashSet},
        hint::black_box,
        time::Instant,
    };

    use a4n_api_contract::outline::OutlineGenerationProgressSummary;
    use a4n_db::project::{
        OutlineGenerationProgressChapter, OutlineGenerationProgressPart,
        OutlineGenerationProgressRows,
    };
    use a4n_domain::chapter::chapter_outline_status;

    use super::generation_progress_summary;
    use crate::WorkerFailure;

    const CHAPTER_COUNT: i32 = 1_000;
    const PART_COUNT: i32 = 20;
    const COMPLETED_CHAPTER_COUNT: i32 = 800;
    const SPARSE_RANGE_CHAPTER_SPAN: i32 = 1_000_000;
    const SAMPLES: usize = 15;
    const ITERATIONS_PER_SAMPLE: usize = 100;
    const SPARSE_RANGE_ITERATIONS_PER_SAMPLE: usize = 5;

    // Frozen pre-optimization implementation for the ignored A/B benchmark.
    fn baseline_generation_progress_summary(
        rows: &OutlineGenerationProgressRows,
    ) -> Result<OutlineGenerationProgressSummary, WorkerFailure> {
        let mut outlines_by_chapter = HashMap::<i32, &str>::new();
        for outline in &rows.chapter_outlines {
            if !chapter_outline_status::is_valid_status(&outline.status) {
                return Err(WorkerFailure::conflict(format!(
                    "章节大纲 {} 状态无效：{}",
                    outline.id, outline.status
                )));
            }
            if outlines_by_chapter
                .insert(outline.chapter_number, outline.status.as_str())
                .is_some()
            {
                return Err(WorkerFailure::conflict(format!(
                    "章节号 {} 存在重复章节大纲",
                    outline.chapter_number
                )));
            }
        }

        let mut claimed_chapters = HashSet::new();
        let mut completed_parts = 0_u64;
        for part in &rows.parts {
            if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
                return Err(WorkerFailure::conflict(format!(
                    "分部 {} 章节范围无效：{}..={}",
                    part.id, part.start_chapter, part.end_chapter
                )));
            }
            for chapter_number in part.start_chapter..=part.end_chapter {
                if !claimed_chapters.insert(chapter_number) {
                    return Err(WorkerFailure::conflict(format!(
                        "章节号 {} 被多个分部范围覆盖",
                        chapter_number
                    )));
                }
            }
            if (part.start_chapter..=part.end_chapter).all(|chapter_number| {
                outlines_by_chapter
                    .get(&chapter_number)
                    .is_some_and(|status| *status == chapter_outline_status::STATUS_COMPLETED)
            }) {
                completed_parts += 1;
            }
        }

        Ok(OutlineGenerationProgressSummary {
            total_parts: rows.parts.len() as u64,
            completed_parts,
            total_chapter_outlines: rows.chapter_outlines.len() as u64,
            completed_chapter_outlines: rows
                .chapter_outlines
                .iter()
                .filter(|outline| outline.status == chapter_outline_status::STATUS_COMPLETED)
                .count() as u64,
        })
    }

    fn progress_fixture() -> OutlineGenerationProgressRows {
        assert_eq!(CHAPTER_COUNT % PART_COUNT, 0);
        let chapters_per_part = CHAPTER_COUNT / PART_COUNT;
        let parts = (0..PART_COUNT)
            .map(|index| OutlineGenerationProgressPart {
                id: i64::from(index + 1),
                start_chapter: index * chapters_per_part + 1,
                end_chapter: (index + 1) * chapters_per_part,
            })
            .collect();
        let chapter_outlines = (1..=CHAPTER_COUNT)
            .map(|chapter_number| OutlineGenerationProgressChapter {
                id: i64::from(chapter_number),
                chapter_number,
                status: if chapter_number <= COMPLETED_CHAPTER_COUNT {
                    chapter_outline_status::STATUS_COMPLETED
                } else {
                    chapter_outline_status::STATUS_DRAFT
                }
                .to_string(),
            })
            .collect();
        OutlineGenerationProgressRows {
            parts,
            chapter_outlines,
        }
    }

    fn sparse_range_fixture() -> OutlineGenerationProgressRows {
        OutlineGenerationProgressRows {
            parts: vec![OutlineGenerationProgressPart {
                id: 1,
                start_chapter: 1,
                end_chapter: SPARSE_RANGE_CHAPTER_SPAN,
            }],
            chapter_outlines: Vec::new(),
        }
    }

    fn measure(
        run: fn(
            &OutlineGenerationProgressRows,
        ) -> Result<OutlineGenerationProgressSummary, WorkerFailure>,
        rows: &OutlineGenerationProgressRows,
        iterations: usize,
    ) -> f64 {
        let started = Instant::now();
        for _ in 0..iterations {
            black_box(run(rows).expect("benchmark rows must be valid"));
        }
        started.elapsed().as_secs_f64() * 1_000_000.0 / iterations as f64
    }

    fn percentile(samples: &[f64], percentile: f64) -> f64 {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
        sorted[index]
    }

    fn benchmark(rows: &OutlineGenerationProgressRows, iterations: usize) -> (Vec<f64>, Vec<f64>) {
        for _ in 0..2 {
            black_box(baseline_generation_progress_summary(rows).expect("baseline warmup"));
            black_box(generation_progress_summary(rows).expect("current warmup"));
        }

        let mut baseline_samples = Vec::with_capacity(SAMPLES);
        let mut current_samples = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            if sample % 2 == 0 {
                baseline_samples.push(measure(
                    baseline_generation_progress_summary,
                    rows,
                    iterations,
                ));
                current_samples.push(measure(generation_progress_summary, rows, iterations));
            } else {
                current_samples.push(measure(generation_progress_summary, rows, iterations));
                baseline_samples.push(measure(
                    baseline_generation_progress_summary,
                    rows,
                    iterations,
                ));
            }
        }
        (baseline_samples, current_samples)
    }

    #[test]
    #[ignore = "manual release CPU benchmark for outline progress projection"]
    fn benchmark_generation_progress_projection() {
        let rows = progress_fixture();
        let baseline = baseline_generation_progress_summary(&rows).expect("baseline result");
        assert_eq!(
            generation_progress_summary(&rows).expect("current result"),
            baseline
        );
        assert_eq!(baseline.completed_parts, 16);
        assert_eq!(baseline.completed_chapter_outlines, 800);

        let (baseline_samples, current_samples) = benchmark(&rows, ITERATIONS_PER_SAMPLE);

        let baseline_p50 = percentile(&baseline_samples, 0.50);
        let current_p50 = percentile(&current_samples, 0.50);
        println!(
            "PERF outline_progress_projection workload=typical chapters={CHAPTER_COUNT} parts={PART_COUNT} completed_chapters={COMPLETED_CHAPTER_COUNT} poll_interval_ms=3000 iterations_per_sample={ITERATIONS_PER_SAMPLE} samples={SAMPLES} baseline_p50_us={baseline_p50:.3} baseline_p95_us={:.3} current_p50_us={current_p50:.3} current_p95_us={:.3} reduction_pct={:.2}",
            percentile(&baseline_samples, 0.95),
            percentile(&current_samples, 0.95),
            (1.0 - current_p50 / baseline_p50) * 100.0
        );

        let sparse_rows = sparse_range_fixture();
        let sparse_baseline =
            baseline_generation_progress_summary(&sparse_rows).expect("sparse baseline result");
        assert_eq!(
            generation_progress_summary(&sparse_rows).expect("sparse current result"),
            sparse_baseline
        );
        assert_eq!(sparse_baseline.completed_parts, 0);
        let (baseline_samples, current_samples) =
            benchmark(&sparse_rows, SPARSE_RANGE_ITERATIONS_PER_SAMPLE);
        let baseline_p50 = percentile(&baseline_samples, 0.50);
        let current_p50 = percentile(&current_samples, 0.50);
        println!(
            "PERF outline_progress_projection workload=sparse_range chapter_span={SPARSE_RANGE_CHAPTER_SPAN} parts=1 chapter_outlines=0 poll_interval_ms=3000 iterations_per_sample={SPARSE_RANGE_ITERATIONS_PER_SAMPLE} samples={SAMPLES} baseline_p50_us={baseline_p50:.3} baseline_p95_us={:.3} current_p50_us={current_p50:.3} current_p95_us={:.3} reduction_pct={:.2}",
            percentile(&baseline_samples, 0.95),
            percentile(&current_samples, 0.95),
            (1.0 - current_p50 / baseline_p50) * 100.0
        );
    }
}

pub(super) fn delete_input(
    input: DeleteLatestOutlineRecordsInput,
) -> a4n_db::project::DeleteLatestOutlineRecordsInput {
    a4n_db::project::DeleteLatestOutlineRecordsInput {
        project_id: input.project_id,
        targets: input
            .targets
            .into_iter()
            .map(|target| a4n_db::project::OutlineDeleteTarget {
                id: target.id,
                expected_updated_at: target.expected_updated_at,
            })
            .collect(),
    }
}

pub(super) fn normalize_update_input(
    input: UpdateChapterOutlineInput,
) -> Result<a4n_db::project::ProjectChapterOutlineUpdateInput, WorkerFailure> {
    super::positive(input.project_id, "project_id")?;
    super::positive(input.outline_id, "outline_id")?;
    let expected_updated_at = input.expected_updated_at.trim().to_string();
    if expected_updated_at.is_empty() {
        return Err(WorkerFailure::invalid_request(
            "缺少章节大纲资料版本，请刷新后重试",
        ));
    }
    Ok(a4n_db::project::ProjectChapterOutlineUpdateInput {
        project_id: input.project_id,
        outline_id: input.outline_id,
        expected_updated_at,
        title: optional_text(input.title, "title")?,
        summary: optional_text(input.summary, "summary")?,
        beats: optional_array(input.beats, "beats")?,
        key_scenes: optional_array(input.key_scenes, "key_scenes")?,
    })
}

fn optional_text(value: Option<String>, field: &str) -> Result<Option<String>, WorkerFailure> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 不能为空"
                )))
            } else {
                Ok(value)
            }
        })
        .transpose()
}

fn optional_array(
    value: Option<serde_json::Value>,
    field: &str,
) -> Result<Option<serde_json::Value>, WorkerFailure> {
    value
        .map(|value| {
            if value.is_array() {
                Ok(value)
            } else {
                Err(WorkerFailure::invalid_request(format!(
                    "章节大纲字段 {field} 必须是数组"
                )))
            }
        })
        .transpose()
}
