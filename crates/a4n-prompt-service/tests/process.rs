use a4n_prompt::contracts::{PromptEmptyPayload, PromptRenderPayload};
use a4n_service_protocol::ServiceRequest;
use a4n_test_support::process::{
    HttpPromptClient, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PromptClient,
    SIDECAR_PROCESS_TEST_TOKEN, ServiceStatus, SidecarProcessTestDir,
    assert_process_concurrency_saturation, assert_protected_route_rejects_invalid_auth,
    assert_route_accepts_unknown_envelope_field, assert_route_rejects_malformed_json,
    assert_route_rejects_oversized_json, assert_route_rejects_unknown_payload_field,
    assert_route_rejects_version_mismatch, assert_sidecar_rejects_non_loopback_without_auth,
    disconnect_during_request_body, finish_sidecar, force_finish_sidecar, request_context,
    sidecar_base_url, sidecar_manager_with_dir, start_sidecar, trusted_remote_request_context,
    unused_local_addr,
};
use serde::de::DeserializeOwned;

fn typed_request<T: DeserializeOwned>(
    request: a4n_test_support::process::JsonRequest,
) -> ServiceRequest<T> {
    let (context, capability_version, payload) = request.into_context_and_payload();
    context
        .with_payload(
            capability_version,
            serde_json::from_value(payload).expect("test payload must match the typed contract"),
        )
        .expect("typed test request context should be valid")
}

fn project_request_context(
    request_id: &str,
    trace_id: &str,
    payload: serde_json::Value,
) -> a4n_test_support::process::JsonRequest {
    let mut context = a4n_service_protocol::ServiceRequestContext::deny_all(
        request_id,
        a4n_service_protocol::TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: None,
        },
    );
    context.user_id = Some(7);
    context.project_id = Some(42);
    context.root_project_id = Some(42);
    context
        .with_payload(PROMPT_RENDER_CAPABILITY_VERSION, payload)
        .expect("project prompt request context should be valid")
}

fn run_prompt_git_command(command: &str, git_dir: &std::path::Path, backup_dir: &std::path::Path) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_a4n-prompt-service"))
        .arg(command)
        .arg(backup_dir)
        .env("A4N_PROMPT_SERVICE_GIT_DIR", git_dir)
        .output()
        .expect("prompt maintenance command should start");
    assert!(
        output.status.success(),
        "prompt {command} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn prompt_service_process_serves_typed_list_and_stops() {
    let addr = unused_local_addr();
    let prompts_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    let temp_dir = SidecarProcessTestDir::new("prompt");
    let prompt_env = [
        (
            "A4N_PROMPT_SERVICE_PROMPTS_DIR".to_string(),
            prompts_dir.to_string_lossy().into_owned(),
        ),
        (
            "A4N_PROMPT_SERVICE_GIT_DIR".to_string(),
            temp_dir
                .path()
                .join("prompts.git")
                .to_string_lossy()
                .into_owned(),
        ),
    ];
    assert_sidecar_rejects_non_loopback_without_auth(
        "prompt-non-loopback",
        env!("CARGO_BIN_EXE_a4n-prompt-service"),
        "A4N_PROMPT_SERVICE_ADDR",
        prompt_env.clone(),
    )
    .await;
    let manager = sidecar_manager_with_dir(
        "prompt",
        env!("CARGO_BIN_EXE_a4n-prompt-service"),
        addr,
        "A4N_PROMPT_SERVICE_ADDR",
        &temp_dir,
        prompt_env,
    );
    let process = start_sidecar(&manager, PROMPT_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, PROMPT_SERVICE_NAME, "/prompt/list").await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/shutdown").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/prompt/list").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/prompt/render").await;
    assert_route_rejects_malformed_json(&base_url, "/prompt/list").await;
    assert_route_rejects_oversized_json(&base_url, "/prompt/list", 8 * 1024 * 1024).await;
    let client = HttpPromptClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let request = request_context(
        "prompt-process-list",
        "prompt-process-trace",
        PROMPT_RENDER_CAPABILITY_VERSION,
        serde_json::json!({}),
    );
    assert_route_rejects_version_mismatch(&sidecar_base_url(addr), "/prompt/list", &request).await;
    assert_route_accepts_unknown_envelope_field(&sidecar_base_url(addr), "/prompt/list", &request)
        .await;
    let response = client
        .list(typed_request::<PromptEmptyPayload>(request))
        .await
        .expect("typed prompt list should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    assert!(
        !response
            .payload
            .expect("prompt list payload")
            .prompts
            .is_empty()
    );

    let render_request = trusted_remote_request_context(
        "prompt-process-render",
        "prompt-process-render-trace",
        PROMPT_RENDER_CAPABILITY_VERSION,
        serde_json::json!({
            "template_name": "writing",
            "variables": {
                "chapter_word_count_range": "10-20 字",
                "chapter_word_count_min": 10,
                "chapter_word_count_max": 20
            }
        }),
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/prompt/render",
        &render_request,
    )
    .await;
    disconnect_during_request_body(addr, "/prompt/render").await;
    let rendered = client
        .render(typed_request::<PromptRenderPayload>(render_request))
        .await
        .expect("typed prompt render should recover after a client disconnect");
    assert_eq!(rendered.status, ServiceStatus::Ok);
    assert!(
        !rendered
            .payload
            .expect("render payload")
            .rendered
            .is_empty()
    );

    let validated = client
        .validate(typed_request(request_context(
            "prompt-process-validate",
            "prompt-process-validate-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({"template_name": "writing", "variables": []}),
        )))
        .await
        .expect("prompt validate should cross the independent process");
    assert_eq!(validated.status, ServiceStatus::Ok);
    let _ = validated.payload.expect("validate payload").missing;

    let missing = client
        .get(typed_request(request_context(
            "prompt-process-missing",
            "prompt-process-missing-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({"name": "missing-process-prompt"}),
        )))
        .await
        .expect("missing prompt should return a typed error response");
    assert_eq!(missing.status, ServiceStatus::Error);
    assert!(missing.error.is_some());

    let update = client
        .update(typed_request(request_context(
            "prompt-process-update",
            "prompt-process-update-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({
                "name": "writing",
                "content": "Prompt Git backup sentinel {{ chapter_title }}",
                "author": "process-test"
            }),
        )))
        .await
        .expect("prompt update should be persisted before backup");
    assert_eq!(update.status, ServiceStatus::Ok);
    let updated_version = update
        .payload
        .as_ref()
        .expect("updated prompt payload")
        .version
        .clone();

    let rendered_updated = client
        .render(typed_request(request_context(
            "prompt-process-render-updated",
            "prompt-process-render-updated-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({
                "template_name": "writing",
                "variables": {"chapter_title": "恢复哨兵"}
            }),
        )))
        .await
        .expect("updated prompt should render");
    assert_eq!(rendered_updated.status, ServiceStatus::Ok);
    assert_eq!(
        rendered_updated
            .payload
            .expect("updated render payload")
            .rendered,
        "Prompt Git backup sentinel 恢复哨兵"
    );

    let render_ab = client
        .render_ab(typed_request(request_context(
            "prompt-process-render-ab",
            "prompt-process-render-ab-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({
                "template_name": "writing",
                "variables": {
                    "chapter_title": "A/B 哨兵",
                    "chapter_word_count_range": "10-20 字",
                    "chapter_word_count_min": 10,
                    "chapter_word_count_max": 20
                }
            }),
        )))
        .await
        .expect("prompt render_ab should cross the independent process");
    assert_eq!(render_ab.status, ServiceStatus::Ok);
    let render_ab_payload = render_ab.payload.expect("render_ab payload");
    assert_eq!(
        render_ab_payload.current,
        "Prompt Git backup sentinel A/B 哨兵"
    );
    assert_ne!(render_ab_payload.current, render_ab_payload.original);

    let history = client
        .version_history(typed_request(request_context(
            "prompt-process-history",
            "prompt-process-history-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({"name": "writing"}),
        )))
        .await
        .expect("prompt version history should cross the independent process");
    assert_eq!(history.status, ServiceStatus::Ok);
    assert!(
        !history
            .payload
            .expect("version history payload")
            .versions
            .is_empty()
    );

    let diff = client
        .diff(typed_request(request_context(
            "prompt-process-version-diff",
            "prompt-process-version-diff-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({
                "name": "writing",
                "version_a": "1.0",
                "version_b": updated_version
            }),
        )))
        .await
        .expect("prompt version diff should cross the independent process");
    assert_eq!(diff.status, ServiceStatus::Ok);
    assert!(!diff.payload.expect("version diff payload").diff.is_empty());

    let override_set = client
        .set_project_override(typed_request(project_request_context(
            "prompt-process-override-set",
            "prompt-process-override-set-trace",
            serde_json::json!({
                "name": "writing",
                "content": "Override {{ chapter_title }}"
            }),
        )))
        .await
        .expect("prompt project override should cross the independent process");
    assert_eq!(override_set.status, ServiceStatus::Ok);
    override_set.payload.expect("override set acknowledgement");
    let overridden = client
        .render(typed_request(project_request_context(
            "prompt-process-override-render",
            "prompt-process-override-render-trace",
            serde_json::json!({
                "template_name": "writing",
                "variables": {"chapter_title": "覆盖哨兵"}
            }),
        )))
        .await
        .expect("project override should affect prompt rendering");
    assert_eq!(overridden.status, ServiceStatus::Ok);
    assert_eq!(
        overridden
            .payload
            .expect("override render payload")
            .rendered,
        "Override 覆盖哨兵"
    );
    let override_removed = client
        .remove_project_override(typed_request(project_request_context(
            "prompt-process-override-remove",
            "prompt-process-override-remove-trace",
            serde_json::json!({"name": "writing"}),
        )))
        .await
        .expect("prompt project override removal should cross the process");
    assert_eq!(override_removed.status, ServiceStatus::Ok);
    override_removed
        .payload
        .expect("override removal acknowledgement");

    finish_sidecar(process, &manager, PROMPT_SERVICE_NAME, addr).await;

    let git_dir = temp_dir.path().join("prompts.git");
    let backup_dir = temp_dir.path().join("prompt-backup");
    run_prompt_git_command("backup", &git_dir, &backup_dir);
    std::fs::remove_dir_all(&git_dir).expect("test should remove original repository");
    run_prompt_git_command("restore", &git_dir, &backup_dir);

    let restored_process = start_sidecar(&manager, PROMPT_SERVICE_NAME).await;
    let restored_client =
        HttpPromptClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    let restored = restored_client
        .get(typed_request(request_context(
            "prompt-process-restored-get",
            "prompt-process-restored-get-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({"name": "writing"}),
        )))
        .await
        .expect("restored prompt should be readable");
    assert_eq!(restored.status, ServiceStatus::Ok);
    assert_eq!(
        restored.payload.expect("restored prompt payload").content,
        "Prompt Git backup sentinel {{ chapter_title }}"
    );
    force_finish_sidecar(restored_process, &manager, PROMPT_SERVICE_NAME, addr).await;

    let force_kill_recovered = start_sidecar(&manager, PROMPT_SERVICE_NAME).await;
    let recovered_client =
        HttpPromptClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    let recovered_after_kill = recovered_client
        .get(typed_request(request_context(
            "prompt-process-force-kill-get",
            "prompt-process-force-kill-get-trace",
            PROMPT_RENDER_CAPABILITY_VERSION,
            serde_json::json!({"name": "writing"}),
        )))
        .await
        .expect("prompt Git should reopen after a forced process stop");
    assert_eq!(recovered_after_kill.status, ServiceStatus::Ok);
    assert_eq!(
        recovered_after_kill
            .payload
            .expect("force-kill prompt payload")
            .content,
        "Prompt Git backup sentinel {{ chapter_title }}"
    );
    finish_sidecar(force_kill_recovered, &manager, PROMPT_SERVICE_NAME, addr).await;

    let manifest_path = git_dir.join(".a4n-prompt-git-data.json");
    let manifest = std::fs::read(&manifest_path).expect("prompt data manifest should exist");
    let mut incompatible: serde_json::Value =
        serde_json::from_slice(&manifest).expect("prompt data manifest should be valid JSON");
    incompatible["format_version"] = serde_json::json!(0);
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&incompatible).unwrap(),
    )
    .unwrap();
    assert!(manager.start_sidecar(PROMPT_SERVICE_NAME).await.is_err());

    std::fs::write(&manifest_path, b"{").unwrap();
    assert!(manager.start_sidecar(PROMPT_SERVICE_NAME).await.is_err());

    std::fs::write(&manifest_path, manifest).unwrap();
    let recovered = start_sidecar(&manager, PROMPT_SERVICE_NAME).await;
    finish_sidecar(recovered, &manager, PROMPT_SERVICE_NAME, addr).await;
}
