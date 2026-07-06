use super::ImportProjectAnalysisSummary;
use super::blueprint::{apply_reverse_blueprint, upsert_import_blueprint};
use super::chapter_outline::{importable_chapters, upsert_outline_from_chapter};
use super::part_outline::upsert_import_part_outlines;
use super::progress::{
    cancel_import_progress, finish_import_progress, import_analysis_cancelled,
    import_progress_resume_stage, import_progress_totals_match, initial_import_progress,
    load_import_progress, load_import_progress_optional, progress_stage_completed,
    resume_import_progress, save_failed_import_progress, save_import_progress,
    update_import_progress_stage,
};
use crate::state::AppState;
use a4n_core::models::{
    chapter::chapter,
    project::{import_review_queue, novel_project},
};
use a4n_services::blueprint::needs_part_outlines_for_chapters;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;

const STAGE_ANALYSIS: &str = "generating_analysis_data";
const STAGE_OUTLINES: &str = "generating_outlines";
const STAGE_BLUEPRINT: &str = "extracting_blueprint";
const STAGE_PARTS: &str = "generating_part_outlines";

pub(super) async fn get_project_import_analysis_progress(
    state: &AppState,
    project_id: i64,
) -> Result<Option<Value>, String> {
    load_import_progress_optional(state.db.as_ref(), project_id).await
}

pub(super) async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;
    let chapters = importable_chapters(state.db.as_ref(), project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可分析的章节".to_string());
    }

    let needs_part_outlines = needs_part_outlines_for_chapters(chapters.len() as i32);
    let mut progress = if let Some(saved_progress) =
        load_import_progress_optional(state.db.as_ref(), project_id).await?
    {
        saved_progress
    } else {
        initial_import_progress(chapters.len(), needs_part_outlines)
    };
    if import_progress_totals_match(&progress, chapters.len(), needs_part_outlines) {
        if progress.get("status").and_then(Value::as_str) == Some("completed") {
            let analyzed_chapters =
                completed_import_stage(&progress, STAGE_ANALYSIS, chapters.len())?;
            let outlined_chapters =
                completed_import_stage(&progress, STAGE_OUTLINES, chapters.len())?;
            let generated_parts =
                completed_import_part_outlines(&progress, needs_part_outlines, chapters.len())?;
            return Ok(ImportProjectAnalysisSummary {
                progress,
                analyzed_chapters,
                outlined_chapters,
                generated_parts,
            });
        }
        if let Some(stage) = import_progress_resume_stage(&progress, needs_part_outlines) {
            resume_import_progress(&mut progress, &stage)?;
        } else {
            progress = initial_import_progress(chapters.len(), needs_part_outlines);
        }
    } else {
        progress = initial_import_progress(chapters.len(), needs_part_outlines);
    }
    save_import_progress(state.db.as_ref(), project_id, &progress).await?;

    let result = run_import_project_analysis_inner(
        state,
        user_id,
        project_id,
        &project,
        chapters,
        needs_part_outlines,
        &mut progress,
    )
    .await;

    match result {
        Ok(summary) => Ok(summary),
        Err(err) => {
            save_failed_import_progress(state.db.as_ref(), project_id, &err)
                .await
                .map_err(|progress_err| {
                    format!("{}; 记录导入失败进度失败: {}", err, progress_err)
                })?;
            Err(err)
        }
    }
}

fn completed_import_stage(progress: &Value, stage: &str, max: usize) -> Result<usize, String> {
    Ok(progress_stage_completed(progress, stage)?.min(max))
}

fn completed_import_part_outlines(
    progress: &Value,
    needs_part_outlines: bool,
    chapter_count: usize,
) -> Result<usize, String> {
    if needs_part_outlines {
        completed_import_stage(progress, STAGE_PARTS, chapter_count.div_ceil(25))
    } else {
        Ok(0)
    }
}

async fn run_import_project_analysis_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    project: &novel_project::Model,
    mut chapters: Vec<chapter::Model>,
    needs_part_outlines: bool,
    progress: &mut Value,
) -> Result<ImportProjectAnalysisSummary, String> {
    macro_rules! save_stage {
        ($stage:expr, $completed:expr, $total:expr, $message:expr) => {{
            update_import_progress_stage(progress, $stage, $completed, $total, $message)?;
            save_import_progress(state.db.as_ref(), project_id, progress).await?;
        }};
    }

    let total_chapters = chapters.len();
    let mut analyzed = completed_import_stage(progress, STAGE_ANALYSIS, total_chapters)?;

    if analyzed < total_chapters {
        save_stage!(
            STAGE_ANALYSIS,
            analyzed,
            total_chapters,
            format!("正在分析第{}章", chapters[analyzed].chapter_number)
        );

        for ch in chapters.iter().skip(analyzed) {
            if import_analysis_cancelled(state.db.as_ref(), project_id).await? {
                let outlined = completed_import_stage(progress, STAGE_OUTLINES, total_chapters)?;
                return cancelled_import_analysis_summary(
                    state,
                    project_id,
                    progress,
                    analyzed,
                    outlined,
                    needs_part_outlines,
                    chapters.len(),
                )
                .await;
            }

            save_stage!(
                STAGE_ANALYSIS,
                analyzed,
                total_chapters,
                format!("正在分析第{}章", ch.chapter_number)
            );
            crate::workflow::postprocess_chapter_after_content_change(state, ch.id)
                .await
                .map_err(|err| err.to_string())?;
            crate::workflow::generate_and_sync_chapter_analysis_with_progress(
                state, user_id, ch.id, None,
            )
            .await
            .map_err(|err| format!("第{}章结构化分析失败: {err}", ch.chapter_number))?;
            analyzed += 1;
            save_stage!(
                STAGE_ANALYSIS,
                analyzed,
                total_chapters,
                format!("已完成第{}章分析", ch.chapter_number)
            );
        }

        save_stage!(
            STAGE_ANALYSIS,
            total_chapters,
            total_chapters,
            "分析数据生成完成"
        );
    } else {
        analyzed = total_chapters;
    }

    chapters = importable_chapters(state.db.as_ref(), project_id).await?;
    let mut outlined = completed_import_stage(progress, STAGE_OUTLINES, chapters.len())?;

    if outlined < chapters.len() {
        save_stage!(
            STAGE_OUTLINES,
            outlined,
            chapters.len(),
            format!("正在更新第{}章大纲", chapters[outlined].chapter_number)
        );

        for ch in chapters.iter().skip(outlined) {
            if import_analysis_cancelled(state.db.as_ref(), project_id).await? {
                return cancelled_import_analysis_summary(
                    state,
                    project_id,
                    progress,
                    analyzed,
                    outlined,
                    needs_part_outlines,
                    chapters.len(),
                )
                .await;
            }

            save_stage!(
                STAGE_OUTLINES,
                outlined,
                chapters.len(),
                format!("正在更新第{}章大纲", ch.chapter_number)
            );
            upsert_outline_from_chapter(state.db.as_ref(), ch).await?;
            outlined += 1;
            save_stage!(
                STAGE_OUTLINES,
                outlined,
                chapters.len(),
                format!("已更新第{}章大纲", ch.chapter_number)
            );
        }

        save_stage!(
            STAGE_OUTLINES,
            chapters.len(),
            chapters.len(),
            "章节大纲已更新"
        );
    } else {
        outlined = chapters.len();
    }

    let blueprint_done = progress_stage_completed(progress, STAGE_BLUEPRINT)? >= 1;
    if !blueprint_done {
        save_stage!(STAGE_BLUEPRINT, 0, 1, "正在生成导入蓝图");
        upsert_import_blueprint(state.db.as_ref(), project, &chapters, needs_part_outlines).await?;
        apply_reverse_blueprint(state, project_id).await?;
        save_stage!(STAGE_BLUEPRINT, 1, 1, "导入蓝图已生成");
    }

    if import_analysis_cancelled(state.db.as_ref(), project_id).await? {
        return cancelled_import_analysis_summary(
            state,
            project_id,
            progress,
            analyzed,
            outlined,
            needs_part_outlines,
            chapters.len(),
        )
        .await;
    }

    let generated_parts = if needs_part_outlines {
        let expected_parts = chapters.len().div_ceil(25);
        let progress_parts = progress_stage_completed(progress, STAGE_PARTS)?;
        if progress_parts >= expected_parts {
            progress_parts
        } else {
            save_stage!(
                STAGE_PARTS,
                progress_parts,
                expected_parts,
                "正在生成分部大纲"
            );
            let generated =
                upsert_import_part_outlines(state.db.as_ref(), project_id, &chapters).await?;
            save_stage!(STAGE_PARTS, generated, expected_parts, "分部大纲已生成");
            generated
        }
    } else {
        0
    };

    finish_import_progress(progress)?;
    save_import_progress(state.db.as_ref(), project_id, progress).await?;
    crate::workflow::sync_project_status_from_artifacts(state, project_id)
        .await
        .map_err(|err| err.to_string())?;

    Ok(ImportProjectAnalysisSummary {
        progress: progress.clone(),
        analyzed_chapters: analyzed,
        outlined_chapters: outlined,
        generated_parts,
    })
}

async fn cancelled_import_analysis_summary(
    state: &AppState,
    project_id: i64,
    progress: &Value,
    analyzed_chapters: usize,
    outlined_chapters: usize,
    needs_part_outlines: bool,
    chapter_count: usize,
) -> Result<ImportProjectAnalysisSummary, String> {
    Ok(ImportProjectAnalysisSummary {
        progress: load_import_progress(state.db.as_ref(), project_id).await?,
        analyzed_chapters,
        outlined_chapters,
        generated_parts: completed_import_part_outlines(
            progress,
            needs_part_outlines,
            chapter_count,
        )?,
    })
}

pub(super) async fn cancel_import_analysis(
    state: &AppState,
    project_id: i64,
) -> Result<u64, String> {
    let result = import_review_queue::Entity::delete_many()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .filter(import_review_queue::Column::ReviewStatus.eq("pending"))
        .exec(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    cancel_import_progress(state.db.as_ref(), project_id).await?;

    Ok(result.rows_affected)
}
