use crate::state::AppState;
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_storage::remove_data_file_in_data_dir;

pub(in crate::workflow) async fn finish_chapter_derived_cleanup(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    artifacts: ChapterDerivedCleanupArtifacts,
) -> Result<(), String> {
    let trace_id = crate::sidecar_clients::chapter_rag_cleanup_operation_trace_id(project_id)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        finish_chapter_derived_cleanup_inner(
            state,
            project_id,
            chapter_id,
            chapter_number,
            artifacts,
        ),
    )
    .await
}

async fn finish_chapter_derived_cleanup_inner(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    artifacts: ChapterDerivedCleanupArtifacts,
) -> Result<(), String> {
    state
        .chapter_content_sync_application()
        .cleanup_derived_rag(project_id, chapter_id, chapter_number, &artifacts)
        .await
        .map_err(|error| error.to_string())?;
    if artifacts.scene_records_changed {
        crate::rag_ingest::sync_scene_rag_records(
            state,
            project_id,
            crate::rag_ingest::SceneRagSyncTraceSource::CurrentOperation,
        )
        .await
        .map_err(|err| {
            format!("更新场景档案 RAG 数据失败：chapter_number={chapter_number}：{err}")
        })?;
    }

    if artifacts.blueprint_facts_changed {
        crate::rag_ingest::sync_blueprint_material_rag_records(state, project_id).await?;
    } else if artifacts.character_facts_changed {
        crate::rag_ingest::sync_protagonist_rag_records(state, project_id).await?;
    }

    for path in artifacts.media_file_paths {
        remove_data_file_in_data_dir(state.data_dir(), &path, "章节媒体文件")?;
    }
    Ok(())
}
