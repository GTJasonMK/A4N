use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;

use crate::actions::app_error_message;
use crate::state::AppState;

use super::super::finalization::finish_chapter_derived_cleanup;
use super::validation::validate_chapter_media_file_paths;

pub async fn delete_latest_chapter_outlines_cascade(
    state: &AppState,
    project_id: i64,
    count: u64,
) -> Result<(), String> {
    let outlines = state
        .orchestrator
        .chapter_outlines_for_project(project_id)
        .await
        .map_err(app_error_message)?;

    let to_delete = latest_chapter_outlines_to_delete(&outlines, count)?;
    let outline_ids = to_delete
        .iter()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    let chapter_numbers = to_delete
        .iter()
        .map(|outline| outline.chapter_number)
        .collect::<Vec<_>>();

    if let Some(chapter_number) = state
        .chapter
        .active_generation_chapter(project_id, &chapter_numbers)
    {
        return Err(generating_chapter_delete_error(chapter_number));
    }

    let chapters = state
        .orchestrator
        .chapters_for_project_numbers(project_id, &chapter_numbers)
        .await
        .map_err(app_error_message)?;
    if let Some(chapter) = chapters
        .iter()
        .find(|chapter| chapter.status == "generating")
    {
        return Err(generating_chapter_delete_error(chapter.chapter_number));
    }

    let cleanup_results = state
        .orchestrator
        .delete_chapter_content_records_and_outlines_record(
            project_id,
            chapters,
            &outline_ids,
            |cleanup_results| {
                for cleanup in cleanup_results {
                    validate_chapter_media_file_paths(
                        &state.config.data_dir,
                        &cleanup.artifacts.media_file_paths,
                    )
                    .map_err(|error| AppError::InvalidState {
                        current: format!("chapter_number={}：{error}", cleanup.chapter_number),
                        operation: "清理章节派生媒体文件".to_string(),
                    })?;
                }
                Ok(())
            },
        )
        .await
        .map_err(app_error_message)?;

    for cleanup in cleanup_results {
        finish_chapter_derived_cleanup(
            state,
            project_id,
            cleanup.chapter_id,
            cleanup.chapter_number,
            cleanup.artifacts,
        )
        .await?;
    }
    state
        .orchestrator
        .refresh_project_current_chapter(project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(in crate::workflow::chapter_cleanup) fn latest_chapter_outlines_to_delete(
    outlines: &[chapter_outline::Model],
    count: u64,
) -> Result<Vec<chapter_outline::Model>, String> {
    if count == 0 {
        return Err("删除章纲数量必须大于 0".to_string());
    }
    if count as usize > outlines.len() {
        return Err(format!(
            "删除章纲数量超过现有章纲数量：count={count}, total={}",
            outlines.len()
        ));
    }

    let mut seen = BTreeSet::new();
    for outline in outlines {
        if outline.chapter_number <= 0 {
            return Err(format!(
                "已有章节大纲章节号无效：chapter_number={}，请先清理章纲状态后再删除",
                outline.chapter_number
            ));
        }
        if !seen.insert(outline.chapter_number) {
            return Err(format!(
                "第 {} 章已有重复章节大纲，请先清理章纲状态后再删除",
                outline.chapter_number
            ));
        }
    }

    let mut sorted = outlines.to_vec();
    sorted.sort_by_key(|outline| (outline.chapter_number, outline.id));
    Ok(sorted.into_iter().rev().take(count as usize).collect())
}

fn generating_chapter_delete_error(chapter_number: i32) -> String {
    format!("第 {chapter_number} 章正在生成中，请等待当前生成完成后再删除章纲")
}
