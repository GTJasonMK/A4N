use super::super::*;

pub(in crate::actions::canvas_workflow) async fn run_canvas_assemble_node(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    run_handle: Option<CanvasRunHandle>,
    mode: &str,
) -> Result<CanvasRunSummary, String> {
    let upstream_run_id = run_handle.map(|run| run.workflow_run_id);
    let (video_ids, title, source_node_ids) =
        build_canvas_assemble_input_for_run(Some(state), upstream_run_id, node, document).await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = json!({
        "project_id": project_id,
        "graph_id": graph_id,
        "node_id": node.id,
        "video_ids": video_ids,
        "title": title,
        "source_node_ids": source_node_ids});
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_assembly",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "video_ids": video_ids,
                "title": title,
                "source_node_ids": source_node_ids}),
            params_json: json!({
                "workflow": "assemble",
                "node_config": node.config_json}),
        },
    )
    .await?;
    let run = start_canvas_node_run(
        state,
        project_id,
        graph_id,
        mode,
        &step_key,
        input_json.clone(),
        run_handle,
    )
    .await?;

    match run_canvas_assembly_task(state, project_id, run.workflow_run_id, || async {
        assemble_canvas_videos(state, user_id, project_id, video_ids.clone(), title).await
    })
    .await
    {
        Ok(video) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let preview_media_asset_id = sync_canvas_assembly_contract(
                state,
                project_id,
                graph_id,
                node.id,
                &video,
                &video_ids,
                generation_spec.as_ref().map(|spec| spec.id),
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "video",
                    "generated_video_id": video.id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": preview_media_asset_id})),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            }
            finish_canvas_run_failed(
                state,
                run.workflow_run_id,
                project_id,
                graph_id,
                mode,
                step_key,
                input_json,
                error,
            )
            .await
        }
    }
}
