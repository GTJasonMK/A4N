use super::super::*;

pub(in crate::actions::canvas_workflow) async fn run_canvas_generate_image_node(
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
    let mut image_input = build_canvas_image_input_for_run(
        Some(state),
        upstream_run_id,
        project_id,
        graph_id,
        node,
        document,
    )
    .await?;
    let step_key = format!("canvas_node:{}", node.id);
    let input_json = serde_json::to_value(&image_input).map_err(|err| err.to_string())?;
    let generation_spec = upsert_freeform_canvas_generation_spec(
        state,
        FreeformCanvasSpecInput {
            project_id,
            graph_id,
            node_id: node.id,
            canvas_text_asset_id: first_canvas_text_asset_id(node, document),
            target_kind: "canvas_image",
            target_entity_kind: "canvas_node",
            status: WORKFLOW_STATUS_RUNNING,
            source_material_json: json!({
                "prompt": image_input.prompt,
                "source_node_ids": image_input.source_node_ids}),
            params_json: json!({
                "workflow": "generate_image",
                "config_id": image_input.config_id,
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

    image_input.workflow_run_id = Some(run.workflow_run_id);

    match generate_canvas_image(state, user_id, image_input, None).await {
        Ok(image) => {
            if let Some(spec) = &generation_spec {
                mark_canvas_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = sync_completed_canvas_image_asset(
                state,
                project_id,
                image.id,
                generation_spec.as_ref().map(|spec| spec.id),
                "generated_canvas_image",
            )
            .await?;
            record_canvas_node_checkpoint(
                state,
                run.workflow_run_id,
                step_key,
                WORKFLOW_STATUS_COMPLETED,
                input_json,
                Some(json!({
                    "data_type": "image",
                    "generated_image_id": image.id,
                    "file_path": image.file_path,
                    "generation_spec_id": generation_spec.as_ref().map(|spec| spec.id),
                    "media_asset_id": media_asset_id})),
                None,
                true,
            )
            .await?;
            finish_canvas_node_success(state, run, project_id, graph_id, mode).await
        }
        Err(error) => {
            if canvas_workflow_run_is_cancelled(state, run.workflow_run_id).await? {
                return finish_canvas_node_cancelled(
                    state,
                    run,
                    project_id,
                    graph_id,
                    mode,
                    step_key,
                    input_json,
                    generation_spec.as_ref().map(|spec| spec.id),
                )
                .await;
            }
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
