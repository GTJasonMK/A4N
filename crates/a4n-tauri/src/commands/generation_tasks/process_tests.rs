use super::*;
use a4n_api_contract::{
    generation_tasks::{
        CancelGenerationTaskInput, GenerationStartRequestMeta, GenerationSubscriptionHandle,
        GenerationTaskHandle, GenerationTaskSummary,
    },
    image_generation::{ImageGenerationOperation, StartImageGenerationInput},
};
use a4n_core::{
    config::app_config::AppConfig,
    files::paths::AppPaths,
    models::{ai::generated_image, user::role::ROLE_USER},
};
use a4n_runtime::state::RuntimeProfile;
use a4n_service_protocol::{GenerationEvent, GenerationTaskStatus};
use a4n_test_support::process::FakeJsonProvider;
use base64::Engine as _;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::time::Duration;
use tauri::{
    WebviewWindow, WebviewWindowBuilder,
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets},
    webview::InvokeRequest,
};

const FIRST_CHANNEL_ID: u32 = 42;
const REMOUNT_CHANNEL_ID: u32 = 43;
const CANCEL_CHANNEL_ID: u32 = 44;
const PNG_1X1_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tauri_channel_remounts_active_image_generation_after_window_cleanup() {
    let provider = FakeJsonProvider::start_delayed(
        json!({"data": [{"b64_json": PNG_1X1_BASE64}]}),
        Duration::from_secs(1),
    )
    .await;
    let expected_bytes = base64::engine::general_purpose::STANDARD
        .decode(PNG_1X1_BASE64)
        .unwrap();
    let expected_sha256 = format!("{:x}", Sha256::digest(&expected_bytes));
    let (db, database_url) = a4n_migration::test_support::connect_test_database_with_url().await;
    let owner = a4n_db::auth::create_user(
        &db,
        "tauri_generation_owner",
        "tauri-generation-owner@example.invalid",
        "tauri-generation-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: "Tauri image generation".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let image_config = a4n_db::config::create_image_config_record(
        &db,
        a4n_db::config::ImageConfigRecordInput {
            name: "tauri-image-provider".to_string(),
            backend: "openai-compatible-image".to_string(),
            base_url: format!("{}/v1", provider.base_url()),
            api_key_encrypted: Some("tauri-test-key".to_string()),
            model: "tauri-test-image-model".to_string(),
            max_concurrent: 1,
            supported_sizes_json: json!([{"width": 1024, "height": 1024}]),
        },
    )
    .await
    .unwrap();
    let runtime_config_snapshot = json!({
        "schema_version": 1,
        "image": {
            "id": image_config.id,
            "name": image_config.name,
            "backend": image_config.backend,
            "base_url": image_config.base_url,
            "api_key_encrypted": image_config.api_key_encrypted,
            "model": image_config.model,
            "max_concurrent": image_config.max_concurrent,
            "supported_sizes": image_config.supported_sizes_json,
            "is_default": image_config.is_default,
        },
    });

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let temp = tempfile::Builder::new()
        .prefix(".a4n-tauri-generation-")
        .tempdir_in(workspace_root)
        .unwrap();
    let mut config = AppConfig::default();
    config.database_url = database_url;
    config.data_dir = temp.path().to_path_buf();
    config.cache_dir = config.data_dir.join("cache");
    config.temp_dir = config.data_dir.join("tmp");
    config.log_dir = config.data_dir.join("logs");
    config.security.jwt_secret = Some("tauri-generation-test-secret".to_string());
    AppPaths::from_config(&config).ensure_dirs().unwrap();
    let data_dir = config.data_dir.clone();
    let state = AppState::new_with_profile(config, RuntimeProfile::Minimal)
        .await
        .unwrap();

    let registry = GenerationSubscriptionRegistry::default();
    let cleanup_registry = registry.clone();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let app = mock_builder()
        .manage(state.clone())
        .manage(registry)
        .channel_interceptor(move |_webview, callback, index, body| {
            if !matches!(
                callback.0,
                FIRST_CHANNEL_ID | REMOUNT_CHANNEL_ID | CANCEL_CHANNEL_ID
            ) {
                return false;
            }
            let event = body
                .clone()
                .deserialize::<GenerationTaskEvent>()
                .expect("Tauri Channel event must match the canonical generation contract");
            event_tx.send((callback.0, index, event)).unwrap();
            if callback.0 == FIRST_CHANNEL_ID && index == 0 {
                // 模拟生产 `WindowEvent::Destroyed`：只允许走 handle_window_destroyed。
                super::handle_window_destroyed(&cleanup_registry, "main");
            }
            true
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::dispatch::invoke_host_command,
            super::subscribe_generation_task,
            super::get_generation_task_summary,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let main_window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let input = StartImageGenerationInput {
        meta: GenerationStartRequestMeta {
            start_request_id: "00000000-0000-4000-8000-000000000272".to_string(),
            retry_of_task_id: None,
        },
        project_id: project.id,
        operation: ImageGenerationOperation::Scene {
            prompt: "雨夜中的仓库入口，电影感构图".to_string(),
            config_id: Some(image_config.id),
            chapter_id: None,
            width: 1024,
            height: 1024,
        },
    };
    let handle: GenerationTaskHandle = invoke_host(
        &main_window,
        owner.id,
        "start_image_generation",
        json!({
            "input": input,
            "runtime_config_snapshot": runtime_config_snapshot.clone(),
        }),
    )
    .unwrap();
    assert_eq!(handle.status, GenerationTaskStatus::Reserved);

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let task = generation_actions::authorize_generation_task(
                &state,
                owner.id,
                &handle.task_id,
                handle.attempt,
            )
            .await
            .unwrap();
            if task.checkpoint.covers_through_sequence >= 2
                && task
                    .public_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.status.is_terminal())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("image generation should expose a replayable running snapshot");

    let _: GenerationSubscriptionHandle = invoke(
        &main_window,
        "subscribe_generation_task",
        json!({
            "userId": owner.id,
            "taskId": handle.task_id,
            "attempt": handle.attempt,
            "afterSequence": null,
            "onEvent": format!("__CHANNEL__:{FIRST_CHANNEL_ID}"),
        }),
    )
    .unwrap();
    let (channel_id, index, first_event) =
        tokio::time::timeout(Duration::from_secs(2), event_rx.recv())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(channel_id, FIRST_CHANNEL_ID);
    assert_eq!(index, 0);
    assert_eq!(first_event.sequence, 1);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), event_rx.recv())
            .await
            .is_err(),
        "window cleanup 后初始 replay 仍继续发送事件"
    );

    let remount_window = WebviewWindowBuilder::new(&app, "remount", Default::default())
        .build()
        .unwrap();
    let _: GenerationSubscriptionHandle = invoke(
        &remount_window,
        "subscribe_generation_task",
        json!({
            "userId": owner.id,
            "taskId": handle.task_id,
            "attempt": handle.attempt,
            "afterSequence": first_event.sequence,
            "onEvent": format!("__CHANNEL__:{REMOUNT_CHANNEL_ID}"),
        }),
    )
    .unwrap();
    let mut events = vec![first_event];
    let mut remount_index = 0;
    loop {
        let (channel_id, index, event) =
            tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
                .await
                .expect("remounted Tauri Channel should reach terminal")
                .expect("Tauri Channel interceptor should remain active");
        assert_eq!(channel_id, REMOUNT_CHANNEL_ID);
        assert_eq!(index, remount_index);
        remount_index += 1;
        let terminal = event.event.is_terminal();
        events.push(event);
        if terminal {
            break;
        }
    }

    for (index, event) in events.iter().enumerate() {
        event.validate().unwrap();
        assert_eq!(event.task_id, handle.task_id);
        assert_eq!(event.attempt, handle.attempt);
        assert_eq!(event.sequence, u64::try_from(index + 1).unwrap());
    }
    let artifacts = events
        .iter()
        .filter_map(|event| match &event.event {
            GenerationEvent::Artifact(artifact) => Some(artifact),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0].artifact;
    assert_eq!(artifact.artifact_kind, "generated_image");
    assert_eq!(artifact.content_type, "image/png");
    assert_eq!(artifact.sha256, expected_sha256);
    assert_eq!(artifact.revision, expected_sha256);
    assert_eq!(artifact.byte_size, expected_bytes.len() as u64);
    assert!(!artifact.temporary);
    let artifact_json = serde_json::to_value(artifact).unwrap();
    assert!(artifact_json.get("file_path").is_none());
    assert!(artifact_json.get("url").is_none());

    let completed = match &events.last().unwrap().event {
        GenerationEvent::Completed(completed) => completed,
        other => panic!("最后一个 Tauri Channel 事件不是 completed: {other:?}"),
    };
    let result_ref = completed.result_ref.as_ref().unwrap();
    assert_eq!(result_ref.kind, "generated_image");
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event.is_terminal())
            .count(),
        1
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), event_rx.recv())
            .await
            .is_err(),
        "completed 后 Tauri Channel 仍继续发送事件"
    );

    let summary: GenerationTaskSummary = invoke(
        &remount_window,
        "get_generation_task_summary",
        json!({
            "userId": owner.id,
            "taskId": handle.task_id,
            "attempt": handle.attempt,
        }),
    )
    .unwrap();
    assert_eq!(summary.handle.status, GenerationTaskStatus::Completed);
    assert_eq!(summary.result_ref.as_ref(), Some(result_ref));
    assert_eq!(summary.terminal_event.as_ref(), events.last());

    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(
        artifact.artifact_id,
        format!("generated-image:{}", images[0].id)
    );
    assert_eq!(
        std::fs::read(data_dir.join(&images[0].file_path)).unwrap(),
        expected_bytes
    );

    let cancelled: GenerationTaskHandle = invoke_host(
        &remount_window,
        owner.id,
        "start_image_generation",
        json!({
            "input": StartImageGenerationInput {
                meta: GenerationStartRequestMeta {
                    start_request_id: "00000000-0000-4000-8000-000000000278".to_string(),
                    retry_of_task_id: None,
                },
                project_id: project.id,
                operation: ImageGenerationOperation::Scene {
                    prompt: "取消中的雨夜仓库".to_string(),
                    config_id: Some(image_config.id),
                    chapter_id: None,
                    width: 1024,
                    height: 1024,
                },
            },
            "runtime_config_snapshot": runtime_config_snapshot,
        }),
    )
    .unwrap();
    let _: GenerationSubscriptionHandle = invoke(
        &remount_window,
        "subscribe_generation_task",
        json!({
            "userId": owner.id,
            "taskId": cancelled.task_id,
            "attempt": cancelled.attempt,
            "afterSequence": null,
            "onEvent": format!("__CHANNEL__:{CANCEL_CHANNEL_ID}"),
        }),
    )
    .unwrap();
    let cancel_handle: GenerationTaskHandle = invoke_host(
        &remount_window,
        owner.id,
        "cancel_generation_task",
        json!({
            "input": CancelGenerationTaskInput {
                task_id: cancelled.task_id.clone(),
                attempt: cancelled.attempt,
            }
        }),
    )
    .unwrap();
    assert_eq!(cancel_handle.task_id, cancelled.task_id);

    let mut cancel_events = Vec::new();
    loop {
        let (channel_id, index, event) =
            tokio::time::timeout(Duration::from_secs(5), event_rx.recv())
                .await
                .expect("cancelled Tauri Channel should reach terminal")
                .expect("Tauri Channel interceptor should remain active");
        assert_eq!(channel_id, CANCEL_CHANNEL_ID);
        assert_eq!(index, cancel_events.len());
        let terminal = event.event.is_terminal();
        cancel_events.push(event);
        if terminal {
            break;
        }
    }
    for (index, event) in cancel_events.iter().enumerate() {
        event.validate().unwrap();
        assert_eq!(event.task_id, cancelled.task_id);
        assert_eq!(event.attempt, cancelled.attempt);
        assert_eq!(event.sequence, u64::try_from(index + 1).unwrap());
    }
    assert!(matches!(
        cancel_events[cancel_events.len() - 2].event,
        GenerationEvent::CancelRequested(_)
    ));
    assert!(matches!(
        cancel_events.last().unwrap().event,
        GenerationEvent::Cancelled(_)
    ));
    assert_eq!(
        cancel_events
            .iter()
            .filter(|event| event.event.is_terminal())
            .count(),
        1
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), event_rx.recv())
            .await
            .is_err(),
        "cancelled 后 Tauri Channel 仍继续发送事件"
    );
    let cancelled_summary: GenerationTaskSummary = invoke(
        &remount_window,
        "get_generation_task_summary",
        json!({
            "userId": owner.id,
            "taskId": cancelled.task_id,
            "attempt": cancelled.attempt,
        }),
    )
    .unwrap();
    assert_eq!(
        cancelled_summary.handle.status,
        GenerationTaskStatus::Cancelled
    );
    assert_eq!(
        cancelled_summary.terminal_event.as_ref(),
        cancel_events.last()
    );

    provider.stop().await;
}

fn invoke<T: DeserializeOwned>(
    window: &WebviewWindow<MockRuntime>,
    command: &str,
    body: Value,
) -> Result<T, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: command.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|body| {
        body.deserialize()
            .expect("Tauri IPC response must deserialize")
    })
}

fn invoke_host<T: DeserializeOwned>(
    window: &WebviewWindow<MockRuntime>,
    user_id: i64,
    command: &str,
    args: Value,
) -> Result<T, Value> {
    invoke(
        window,
        "invoke_host_command",
        json!({ "command": command, "args": args, "userId": user_id }),
    )
}
