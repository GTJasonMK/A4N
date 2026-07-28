use std::{
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
use std::{sync::Mutex, time::UNIX_EPOCH};

#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-llm-provider",
    feature = "embedded-media-providers"
))]
use std::convert::Infallible;

use a4n_api_contract::temp_media::TempMediaDeleteRequest;
use a4n_core::models::{
    ai::generated_image,
    chapter::{chapter, chapter_outline, part_outline},
    generation::generation_task_checkpoint,
    project::novel_blueprint,
    task::cleanup_outbox,
    user::role::{ROLE_ADMIN, ROLE_USER},
};
use a4n_db::generation::{
    PersistGenerationCheckpointInput, ReserveGenerationTaskInput, persist_generation_checkpoint,
    reserve_generation_task,
};
use a4n_queue::priority::{TaskKind, TaskPriority};
use a4n_service_protocol::GenerationDeliveryMode;
use axum::body::Body;
#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-llm-provider",
    feature = "embedded-media-providers"
))]
use axum::routing::any;
use axum::{
    Json, Router,
    extract::Path as AxumPath,
    http::{HeaderMap, StatusCode, header},
};
use base64::Engine as _;
use chrono::{Duration as ChronoDuration, Utc};
use futures::{StreamExt as _, stream};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde_json::json;
use sha2::Digest as _;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const PROXY_FAULT_NONE: usize = 0;
const PROXY_FAULT_CONSISTENCY: usize = 1;
const PROXY_FAULT_EXPIRED_WITH_REF: usize = 2;
const PROXY_FAULT_EXPIRED_WITHOUT_REF: usize = 3;
const PROXY_FAULT_CORRUPT_WITH_REF: usize = 4;
const PROXY_FAULT_CORRUPT_WITHOUT_REF: usize = 5;
#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-media-providers"
))]
const PNG_1X1_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
struct FakeOpenAiSseProvider {
    addr: SocketAddr,
    first_frame_unix_ms: Arc<Mutex<Vec<u64>>>,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
impl FakeOpenAiSseProvider {
    async fn start_delayed(
        frames: Vec<String>,
        frame_delay: Duration,
        routed: Vec<(String, Vec<String>, Duration)>,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake OpenAI SSE provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("fake OpenAI SSE provider should expose its local address");
        let frames = Arc::new(frames);
        let routed = routed
            .into_iter()
            .map(|(needle, frames, delay)| (needle, Arc::new(frames), delay))
            .collect::<Vec<_>>();
        let first_frame_unix_ms = Arc::new(Mutex::new(Vec::new()));
        let recorded_first_frames = Arc::clone(&first_frame_unix_ms);
        let router = Router::new().fallback(any(move |body: String| {
            let route_body = if routed.is_empty() {
                String::new()
            } else {
                let request: serde_json::Value =
                    serde_json::from_str(&body).expect("fake OpenAI request must be valid JSON");
                let current_user_content = request["messages"]
                    .as_array()
                    .and_then(|messages| {
                        messages
                            .iter()
                            .rev()
                            .find(|message| message["role"] == "user")
                    })
                    .and_then(|message| message["content"].as_str())
                    .expect("fake OpenAI request must contain a user message");
                current_user_content
                    .split_once("输入：\n")
                    .and_then(|(_, input)| {
                        serde_json::Deserializer::from_str(input)
                            .into_iter::<serde_json::Value>()
                            .next()?
                            .ok()
                    })
                    .and_then(|input| {
                        input
                            .pointer("/conversation/latest_user_input")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_else(|| current_user_content.to_string())
            };
            let (frames, frame_delay) = routed
                .iter()
                .filter_map(|(needle, frames, delay)| {
                    route_body.rfind(needle).map(|index| (index, frames, delay))
                })
                .max_by_key(|(index, _, _)| *index)
                .map(|(_, frames, delay)| (Arc::clone(frames), *delay))
                .unwrap_or_else(|| (Arc::clone(&frames), frame_delay));
            let recorded_first_frames = Arc::clone(&recorded_first_frames);
            async move {
                let body = stream::iter(frames.as_ref().clone().into_iter().enumerate()).then(
                    move |(index, frame)| {
                        let recorded_first_frames = Arc::clone(&recorded_first_frames);
                        async move {
                            tokio::time::sleep(frame_delay).await;
                            if index == 0 {
                                recorded_first_frames.lock().unwrap().push(
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as u64,
                                );
                            }
                            Ok::<String, Infallible>(frame)
                        }
                    },
                );
                (
                    [
                        (header::CONTENT_TYPE, "text/event-stream"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    Body::from_stream(body),
                )
            }
        }));
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        Self {
            addr,
            first_frame_unix_ms,
            shutdown,
            task,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}/v1", self.addr)
    }

    fn first_frame_unix_ms(&self) -> Vec<u64> {
        self.first_frame_unix_ms.lock().unwrap().clone()
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        tokio::time::timeout(Duration::from_secs(5), self.task)
            .await
            .expect("fake OpenAI SSE provider should stop before timeout")
            .expect("fake OpenAI SSE provider task should join")
            .expect("fake OpenAI SSE provider should stop cleanly");
    }
}

#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-media-providers"
))]
struct FakeOpenAiImageProvider {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), std::io::Error>>,
}

#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-media-providers"
))]
impl FakeOpenAiImageProvider {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake OpenAI image provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("fake OpenAI image provider should expose its local address");
        let router = Router::new().fallback(any(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                json!({"data": [{"b64_json": PNG_1X1_BASE64}]}).to_string(),
            )
        }));
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        Self {
            addr,
            shutdown,
            task,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}/v1", self.addr)
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        tokio::time::timeout(Duration::from_secs(5), self.task)
            .await
            .expect("fake OpenAI image provider should stop before timeout")
            .expect("fake OpenAI image provider task should join")
            .expect("fake OpenAI image provider should stop cleanly");
    }
}

#[tokio::test]
async fn server_process_recovers_cleanup_after_restart() {
    let (db, database_url) = isolated_test_database().await;

    let user = a4n_db::auth::create_user(
        &db,
        "cleanup_process_admin",
        "cleanup-process-admin@example.invalid",
        "cleanup-process-password",
        ROLE_ADMIN,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: user.id,
            title: "cleanup recovery".to_string(),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let project_id = project.id;
    let managed_orphan_paths = [
        "images/orphan.png",
        "videos/orphan.mp4",
        "portraits/orphan.png",
        "audio/orphan.mp3",
        "media/orphan.bin",
        "assets/orphan.bin",
    ]
    .map(|path| format!("projects/{project_id}/{path}"));
    let non_media_orphan_path = format!("projects/{}/chapters/orphan.md", project_id + 1);
    let restart_path = format!("projects/{project_id}/custom/restart.bin");
    let failed_path = format!("projects/{project_id}/custom/not-a-file.bin");
    let expired_cache_output =
        format!("projects/{project_id}/videos/chapter_1/assembled_expired.mp4");
    let expired_cache_manifest = format!("{expired_cache_output}.cache.json");
    let expired_upload = data_dir.join("tmp/video_uploads/video_upload_expired.mp4");
    for path in managed_orphan_paths
        .iter()
        .chain([&non_media_orphan_path, &restart_path])
    {
        let absolute = data_dir.join(path);
        std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        std::fs::write(absolute, b"orphan").unwrap();
    }
    std::fs::create_dir_all(data_dir.join(&failed_path)).unwrap();
    for path in [&expired_cache_output, &expired_cache_manifest] {
        let absolute = data_dir.join(path);
        std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        std::fs::write(absolute, b"expired cache").unwrap();
    }
    std::fs::create_dir_all(expired_upload.parent().unwrap()).unwrap();
    std::fs::write(&expired_upload, b"expired upload").unwrap();
    let expired_at = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
    for path in [
        data_dir.join(&expired_cache_output),
        data_dir.join(&expired_cache_manifest),
        expired_upload.clone(),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(expired_at))
            .unwrap();
    }

    let restart = a4n_db::cleanup_outbox::insert_file_artifact_cleanup_intent(
        &db,
        format!("test-process-restart-{project_id}"),
        a4n_db::cleanup_outbox::FileArtifactCleanupPayload::new(
            project_id,
            vec![restart_path.clone()],
        ),
    )
    .await
    .unwrap();
    let failed = a4n_db::cleanup_outbox::insert_file_artifact_cleanup_intent(
        &db,
        format!("test-process-delete-failure-{project_id}"),
        a4n_db::cleanup_outbox::FileArtifactCleanupPayload::new(
            project_id,
            vec![failed_path.clone()],
        ),
    )
    .await
    .unwrap();

    let first_addr = unused_local_addr();
    let mut first = spawn_server(&database_url, &data_dir, &frontend_dir, first_addr);
    wait_for_listener(&mut first, first_addr).await;
    stop_server(first);
    let restarted = cleanup_outbox::Entity::find_by_id(restart.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        restarted.status,
        a4n_db::cleanup_outbox::CLEANUP_STATUS_COMPLETED
    );
    assert_eq!(restarted.attempt_count, 1);
    let pending = cleanup_outbox::Entity::find_by_id(failed.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        pending.status,
        a4n_db::cleanup_outbox::CLEANUP_STATUS_PENDING
    );
    assert_eq!(pending.attempt_count, 1);
    assert!(pending.last_error.as_deref().unwrap().contains("不是文件"));
    for path in &managed_orphan_paths {
        assert!(!data_dir.join(path).exists(), "未恢复受管媒体: {path}");
    }
    assert!(!data_dir.join(&non_media_orphan_path).exists());
    assert!(!data_dir.join(&restart_path).exists());
    assert!(!data_dir.join(&expired_cache_output).exists());
    assert!(!data_dir.join(&expired_cache_manifest).exists());
    assert!(!expired_upload.exists());

    std::fs::remove_dir(data_dir.join(&failed_path)).unwrap();
    std::fs::write(data_dir.join(&failed_path), b"retry").unwrap();
    let mut retry: cleanup_outbox::ActiveModel = pending.into();
    retry.next_attempt_at = Set(Utc::now().naive_utc() - ChronoDuration::seconds(1));
    retry.update(&db).await.unwrap();

    let second_addr = unused_local_addr();
    let mut second = spawn_server(&database_url, &data_dir, &frontend_dir, second_addr);
    wait_for_listener(&mut second, second_addr).await;
    stop_server(second);
    let completed = cleanup_outbox::Entity::find_by_id(failed.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        completed.status,
        a4n_db::cleanup_outbox::CLEANUP_STATUS_COMPLETED
    );
    assert_eq!(completed.attempt_count, 2);
    assert!(completed.last_error.is_none());
    assert!(!data_dir.join(&failed_path).exists());
}

#[tokio::test]
async fn server_process_recovers_uploaded_image_after_record_and_cleanup_intent_failures() {
    let (db, database_url) = isolated_test_database().await;
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let first_addr = unused_local_addr();
    let mut first = spawn_server(&database_url, &data_dir, &frontend_dir, first_addr);
    wait_for_listener(&mut first, first_addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let auth = client
        .post(format!("http://{first_addr}/api/auth/login"))
        .json(&serde_json::json!({
            "username": "cleanup_process_admin",
            "password": "cleanup-process-password",
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let user_id = auth["user"]["id"].as_i64().unwrap();
    let access_token = auth["tokens"]["access_token"].as_str().unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id,
            title: "cleanup upload fault injection".to_string(),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();

    install_upload_failure_triggers(&db).await;
    let response = client
        .post(format!("http://{first_addr}/api/commands/upload_image"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "userId": user_id,
            "projectId": project.id,
            "input": {
                "file_name": "fault.png",
                "mime_type": "image/png",
                "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
                "image_type": "imported",
                "prompt": "fault injection",
                "model": null,
                "chapter_id": null,
                "panel_id": null,
                "metadata_json": null
            }
        }))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let error = response.json::<serde_json::Value>().await.unwrap()["error"]
        .as_str()
        .unwrap()
        .to_string();
    stop_server(first);

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert!(error.contains("导入图片记录创建失败"), "{error}");
    assert!(error.contains("后续清理也失败"), "{error}");
    let images_dir = data_dir
        .join("projects")
        .join(project.id.to_string())
        .join("images");
    let mut orphan_files = std::fs::read_dir(&images_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("imported_image_"))
        })
        .collect::<Vec<_>>();
    assert_eq!(orphan_files.len(), 1);
    let orphan_path = orphan_files.pop().unwrap();
    let orphan_relative_path = format!(
        "projects/{}/images/{}",
        project.id,
        orphan_path.file_name().unwrap().to_string_lossy()
    );
    assert!(orphan_path.is_file());
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::ProjectId.eq(project.id))
            .filter(
                cleanup_outbox::Column::CleanupKind
                    .eq(a4n_db::cleanup_outbox::FILE_ARTIFACT_CLEANUP_KIND),
            )
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );

    remove_upload_failure_triggers(&db).await;
    let second_addr = unused_local_addr();
    let mut second = spawn_server(&database_url, &data_dir, &frontend_dir, second_addr);
    wait_for_listener(&mut second, second_addr).await;
    stop_server(second);

    assert!(!orphan_path.exists());
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let cleanup = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(project.id))
        .filter(
            cleanup_outbox::Column::CleanupKind
                .eq(a4n_db::cleanup_outbox::FILE_ARTIFACT_CLEANUP_KIND),
        )
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        cleanup.status,
        a4n_db::cleanup_outbox::CLEANUP_STATUS_COMPLETED
    );
    assert_eq!(cleanup.attempt_count, 1);
    assert!(cleanup.completed_at.is_some());
    assert!(
        cleanup
            .dedupe_key
            .starts_with("file-artifact:startup-orphan-media:")
    );
    assert_eq!(cleanup.payload_json["paths"][0], orphan_relative_path);
}

#[tokio::test]
async fn server_process_recovers_uploaded_image_killed_before_database_commit() {
    const INSERT_LOCK_KEY: i64 = 4_205_150_915;

    let (db, database_url) = isolated_test_database().await;
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let first_addr = unused_local_addr();
    let mut first = spawn_server(&database_url, &data_dir, &frontend_dir, first_addr);
    wait_for_listener(&mut first, first_addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let access_token = client
        .post(format!("http://{first_addr}/api/auth/login"))
        .json(&serde_json::json!({
            "username": "cleanup_process_admin",
            "password": "cleanup-process-password",
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let user = a4n_db::auth::list_users(&db)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.username == "cleanup_process_admin")
        .expect("bootstrap admin user must exist after server start");
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: user.id,
            title: "cleanup upload kill window".to_string(),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();

    install_upload_blocking_trigger(&db, INSERT_LOCK_KEY).await;
    let blocker = db.begin().await.unwrap();
    blocker
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT pg_advisory_xact_lock({INSERT_LOCK_KEY})"),
        ))
        .await
        .unwrap();

    let user_id = user.id;
    let project_id = project.id;
    let request = tokio::spawn({
        let client = client.clone();
        let access_token = access_token.clone();
        async move {
            client
                .post(format!("http://{first_addr}/api/commands/upload_image"))
                .bearer_auth(access_token)
                .json(&serde_json::json!({
                    "userId": user_id,
                    "projectId": project_id,
                    "input": {
                        "file_name": "kill-window.png",
                        "mime_type": "image/png",
                        "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
                        "image_type": "imported",
                        "prompt": "kill window",
                        "model": null,
                        "chapter_id": null,
                        "panel_id": null,
                        "metadata_json": null
                    }
                }))
                .send()
                .await
        }
    });
    let images_dir = data_dir
        .join("projects")
        .join(project_id.to_string())
        .join("images");
    let orphan_path = wait_for_file_with_prefix(&mut first, &images_dir, "imported_image_").await;

    stop_server(first);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), request)
            .await
            .expect("upload request should end after server is killed")
            .expect("upload request task should join")
            .is_err(),
        "killed upload request must not report success"
    );
    blocker.rollback().await.unwrap();
    remove_upload_blocking_trigger(&db).await;

    assert!(orphan_path.is_file());
    assert!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::ProjectId.eq(project_id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );

    let second_addr = unused_local_addr();
    let mut second = spawn_server(&database_url, &data_dir, &frontend_dir, second_addr);
    wait_for_listener(&mut second, second_addr).await;
    stop_server(second);

    assert!(!orphan_path.exists());
    let cleanup = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(project_id))
        .filter(
            cleanup_outbox::Column::CleanupKind
                .eq(a4n_db::cleanup_outbox::FILE_ARTIFACT_CLEANUP_KIND),
        )
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        cleanup.status,
        a4n_db::cleanup_outbox::CLEANUP_STATUS_COMPLETED
    );
    assert_eq!(cleanup.attempt_count, 1);
}

#[tokio::test]
async fn server_process_returns_canonical_temp_media_artifact_contract() {
    let (_db, database_url) = isolated_test_database().await;
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        )
        .unwrap();
    let expected_sha256 = format!("{:x}", sha2::Sha256::digest(&bytes));
    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let response = client
        .post(format!("http://{addr}/api/temp-media"))
        .bearer_auth("temp-media-process-token")
        .json(&json!({
            "owner_user_id": 7,
            "project_id": 11,
            "source_image_id": 13,
            "expires_in_seconds": 86400,
            "file_name": "reference.png",
            "mime_type": "image/png",
            "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<a4n_api_contract::temp_media::TempMediaUploadResponse>()
        .await
        .unwrap();

    assert_eq!(response.owner_user_id, 7);
    assert_eq!(response.project_id, 11);
    assert_eq!(response.source_image_id, 13);
    assert_eq!(response.content_type, "image/png");
    assert_eq!(response.byte_size, bytes.len() as u64);
    assert_eq!(response.sha256, expected_sha256);
    assert!(response.url.starts_with("https://media.example.test/"));
    assert!(response.url.contains(&response.artifact_id));
    let expires_at = chrono::DateTime::parse_from_rfc3339(&response.expires_at).unwrap();
    assert!(expires_at > chrono::Utc::now());
    assert!(expires_at <= chrono::Utc::now() + ChronoDuration::hours(24));

    let file_name = format!("{}.png", response.artifact_id);
    let path = data_dir.join("temp-media/video-inputs").join(&file_name);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let served = client
        .get(format!("http://{addr}/api/temp-media/{file_name}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(served.as_ref(), bytes.as_slice());

    let unauthorized_delete = client
        .delete(format!(
            "http://{addr}/api/temp-media/{}",
            response.artifact_id
        ))
        .json(&json!({"sha256": response.sha256}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        unauthorized_delete.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    let mismatched_delete = client
        .delete(format!(
            "http://{addr}/api/temp-media/{}",
            response.artifact_id
        ))
        .bearer_auth("temp-media-process-token")
        .json(&json!({"sha256": "b".repeat(64)}))
        .send()
        .await
        .unwrap();
    assert_eq!(mismatched_delete.status(), reqwest::StatusCode::BAD_REQUEST);
    assert!(path.is_file());

    let deleted = client
        .delete(format!(
            "http://{addr}/api/temp-media/{}",
            response.artifact_id
        ))
        .bearer_auth("temp-media-process-token")
        .json(&a4n_api_contract::temp_media::TempMediaDeleteRequest {
            sha256: response.sha256.clone(),
        })
        .send()
        .await
        .unwrap();
    assert_eq!(deleted.status(), reqwest::StatusCode::NO_CONTENT);
    assert!(!path.exists());

    let repeated_delete = client
        .delete(format!(
            "http://{addr}/api/temp-media/{}",
            response.artifact_id
        ))
        .bearer_auth("temp-media-process-token")
        .json(&json!({"sha256": response.sha256}))
        .send()
        .await
        .unwrap();
    assert_eq!(repeated_delete.status(), reqwest::StatusCode::NOT_FOUND);

    stop_server(server);
}

#[tokio::test]
async fn server_process_exclusively_owns_data_directory_until_killed() {
    let (_db, database_url) = isolated_test_database().await;
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let first_addr = unused_local_addr();
    let mut first = spawn_server(&database_url, &data_dir, &frontend_dir, first_addr);
    wait_for_listener(&mut first, first_addr).await;

    let second = spawn_server(&database_url, &data_dir, &frontend_dir, unused_local_addr());
    let error = wait_for_failure(second).await;
    assert!(error.contains("另一个 A4N Host 占用"), "{error}");
    assert!(tokio::net::TcpStream::connect(first_addr).await.is_ok());

    stop_server(first);
    let recovered_addr = unused_local_addr();
    let mut recovered = spawn_server(&database_url, &data_dir, &frontend_dir, recovered_addr);
    wait_for_listener(&mut recovered, recovered_addr).await;
    stop_server(recovered);
}

#[tokio::test]
async fn server_destructive_drain_releases_database_locks_before_waiting() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("destructive_lock_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_freeform_video_project_record(
        &db,
        owner.id,
        a4n_db::project::CreateFreeformVideoProjectParams {
            title: format!("destructive lock {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(40))
        .build()
        .unwrap();
    let access_token = login_access_token(&client, addr, &username).await;
    let reservation = reserve_generation_task(
        &db,
        generation_reservation_input(
            owner.id,
            project.id,
            Uuid::new_v4().to_string(),
            "destructive-lock-wait",
        ),
    )
    .await
    .unwrap();
    let task_id = reservation.task.checkpoint.task_id;
    // A finalizing task cannot be interrupted by drain, keeping the real request in its wait loop.
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE generation_task_checkpoint SET status = 'finalizing' WHERE task_id = $1",
        [task_id.clone().into()],
    ))
    .await
    .unwrap();

    let request_client = client.clone();
    let request_token = access_token.clone();
    let delete_request = tokio::spawn(async move {
        request_client
            .post(format!("http://{addr}/api/commands/delete_project"))
            .bearer_auth(request_token)
            .json(&json!({"userId": owner.id, "projectId": project.id}))
            .send()
            .await
            .unwrap()
    });

    let gate_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let gate = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT lifecycle_gate FROM novel_project WHERE id = $1",
                [project.id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "lifecycle_gate")
            .unwrap();
        if gate == "deleting" {
            break;
        }
        assert!(
            Instant::now() < gate_deadline,
            "delete request did not commit its lifecycle gate"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let conflicting_locks = db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            r#"
SELECT relation.relname, held_lock.mode, held_lock.pid
FROM pg_locks AS held_lock
JOIN pg_class AS relation ON relation.oid = held_lock.relation
JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
WHERE namespace.nspname = current_schema()
  AND relation.relname IN (
    'user',
    'novel_project',
    'generation_task_checkpoint',
    'project_cleanup_outbox'
  )
  AND held_lock.pid <> pg_backend_pid()
  AND held_lock.granted
  AND held_lock.mode <> 'AccessShareLock'
ORDER BY relation.relname, held_lock.mode, held_lock.pid
"#,
        ))
        .await
        .unwrap();
    assert!(
        conflicting_locks.is_empty(),
        "destructive drain sleep retained database locks: {conflicting_locks:?}"
    );

    let lock_probe = db.begin().await.unwrap();
    lock_probe
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "SET LOCAL lock_timeout = '250ms'",
        ))
        .await
        .unwrap();
    for (sql, value) in [
        (
            r#"SELECT id FROM "user" WHERE id = $1 FOR UPDATE NOWAIT"#,
            owner.id.into(),
        ),
        (
            "SELECT id FROM novel_project WHERE id = $1 FOR UPDATE NOWAIT",
            project.id.into(),
        ),
        (
            "SELECT task_id FROM generation_task_checkpoint WHERE task_id = $1 FOR UPDATE NOWAIT",
            task_id.into(),
        ),
    ] {
        lock_probe
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [value],
            ))
            .await
            .unwrap()
            .expect("destructive drain target row should remain lockable");
    }
    lock_probe
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "LOCK TABLE project_cleanup_outbox IN SHARE ROW EXCLUSIVE MODE NOWAIT",
        ))
        .await
        .unwrap();
    lock_probe.commit().await.unwrap();

    let response = delete_request.await.unwrap();
    let status = response.status();
    let error = response.json::<serde_json::Value>().await.unwrap()["error"]
        .as_str()
        .unwrap()
        .to_string();
    stop_server(server);
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert!(
        error.contains("generation_destructive_drain_timeout"),
        "{error}"
    );
}

#[tokio::test]
async fn server_process_recovers_snapshot_only_generation_subscription() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let owner = a4n_db::auth::create_user(
        &db,
        &format!("generation_process_owner_{suffix}"),
        &format!("generation_process_owner_{suffix}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    a4n_db::auth::create_user(
        &db,
        &format!("generation_process_outsider_{suffix}"),
        &format!("generation_process_outsider_{suffix}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_freeform_video_project_record(
        &db,
        owner.id,
        a4n_db::project::CreateFreeformVideoProjectParams {
            title: format!("generation process {suffix}"),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    let (mut provider_delete_rx, provider_shutdown, provider_task) = {
        let (provider_delete_tx, provider_delete_rx) = mpsc::unbounded_channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider_addr = listener.local_addr().unwrap();
        let router = Router::new().route(
            "/openai/v1/videos/{video_id}",
            axum::routing::get(|| async { Json(json!({"status": "completed"})) }).delete(
                move |AxumPath(video_id): AxumPath<String>, headers: HeaderMap| {
                    let provider_delete_tx = provider_delete_tx.clone();
                    async move {
                        provider_delete_tx
                            .send((
                                video_id,
                                headers
                                    .get(header::AUTHORIZATION)
                                    .and_then(|value| value.to_str().ok())
                                    .map(str::to_string),
                            ))
                            .unwrap();
                        StatusCode::NO_CONTENT
                    }
                },
            ),
        );
        let (provider_shutdown, shutdown_rx) = oneshot::channel();
        let provider_task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });
        let config = a4n_db::config::create_video_config_record(
            &db,
            a4n_db::config::VideoConfigCreateRecordInput {
                name: format!("cleanup-video-{suffix}"),
                backend: "cliproxy-openai-video".to_string(),
                base_url: format!("http://{provider_addr}/openai/v1"),
                api_key_encrypted: Some("provider-cleanup-key".to_string()),
                model: "cleanup-video-model".to_string(),
                max_concurrent: 1,
                default_width: 1280,
                default_height: 720,
                default_duration_secs: 5,
                min_duration_secs: 1,
                max_duration_secs: 15,
                duration_mode: "range".to_string(),
                supported_durations_json: None,
                default_fps: 24,
                provider_options_json: None,
            },
        )
        .await
        .unwrap();
        a4n_db::video::create_generated_video_record(
            &db,
            a4n_db::video::CreateGeneratedVideoRecordDraftInput {
                project_id: project.id,
                chapter_id: None,
                video_type: "chapter_video".to_string(),
                workflow_mode: Some("reference_media".to_string()),
                prompt: "provider cleanup".to_string(),
                negative_prompt: None,
                model: config.model.clone(),
                width: 1280,
                height: 720,
                duration_secs: 5.0,
                fps: 24,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: Some("provider-job-owner-lost".to_string()),
                metadata_json: Some(json!({"config_id": config.id})),
            },
        )
        .await
        .unwrap();
        (provider_delete_rx, provider_shutdown, provider_task)
    };

    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    let checkpoint_provider_kind = "cliproxy-openai-video";
    #[cfg(not(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    )))]
    let checkpoint_provider_kind = "openai-compatible-video";

    let startup_recovery = reserve_generation_task(
        &db,
        generation_reservation_input(
            owner.id,
            project.id,
            Uuid::new_v4().to_string(),
            "server-process-startup-recovery",
        ),
    )
    .await
    .unwrap();
    let startup_recovered = startup_recovery.task.checkpoint.task_id.clone();
    persist_generation_checkpoint(
        &db,
        PersistGenerationCheckpointInput {
            task_id: startup_recovered.clone(),
            attempt: 1,
            expected_checkpoint_revision: startup_recovery.task.checkpoint.checkpoint_revision,
            public_snapshot: startup_recovery.task.public_snapshot.unwrap(),
            step_key: Some("video_provider_job".to_string()),
            call_key: None,
            provider_kind: Some(checkpoint_provider_kind.to_string()),
            provider_model: Some("video-model".to_string()),
            provider_job_id: Some("provider-job-owner-lost".to_string()),
            provider_idempotency_key_sha256: Some("b".repeat(64)),
            resume_metadata_version: Some(1),
            execution_checkpoint_json: Some(json!({
                "version": 1,
                "video_id": 42,
                "provider_job_id": "provider-job-owner-lost",
                "request": {
                    "version": 1,
                    "video_id": 42,
                    "request_fingerprint_sha256": "b".repeat(64),
                },
                "staging_artifacts": [{
                    "artifact_id": "0123456789abcdef0123456789abcdef",
                    "artifact_kind": "remote_temp_media",
                    "content_type": "image/png",
                    "sha256": "c".repeat(64),
                    "byte_size": 3,
                    "revision": "c".repeat(64),
                    "temporary": true,
                    "expires_at_ms": Utc::now().timestamp_millis() + 3_600_000,
                }],
            })),
        },
    )
    .await
    .unwrap();

    let (delete_tx, mut delete_rx) = mpsc::unbounded_channel();
    let delete_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let delete_addr = delete_listener.local_addr().unwrap();
    let delete_router = Router::new().route(
        "/api/temp-media/{artifact_id}",
        axum::routing::delete(
            move |AxumPath(artifact_id): AxumPath<String>,
                  headers: HeaderMap,
                  Json(payload): Json<TempMediaDeleteRequest>| {
                let delete_tx = delete_tx.clone();
                async move {
                    delete_tx
                        .send((
                            artifact_id,
                            payload.sha256,
                            headers
                                .get(header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok())
                                .map(str::to_string),
                        ))
                        .unwrap();
                    StatusCode::NO_CONTENT
                }
            },
        ),
    );
    let (delete_shutdown, delete_shutdown_rx) = oneshot::channel();
    let delete_task = tokio::spawn(async move {
        axum::serve(delete_listener, delete_router)
            .with_graceful_shutdown(async move {
                let _ = delete_shutdown_rx.await;
            })
            .await
    });

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();

    let addr = unused_local_addr();
    let remote_temp_upload_url = format!("http://{delete_addr}/api/temp-media");
    let mut command = server_command(&database_url, &data_dir, &frontend_dir, addr);
    let mut server = command
        .env("A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL", &remote_temp_upload_url)
        .env(
            "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN",
            "temp-media-process-token",
        )
        .spawn()
        .expect("start a4n-server generation recovery process test");
    wait_for_listener(&mut server, addr).await;
    let (deleted_artifact_id, deleted_sha256, authorization) =
        tokio::time::timeout(PROCESS_TIMEOUT, delete_rx.recv())
            .await
            .expect("staging artifact DELETE should reach the remote Host")
            .expect("remote staging DELETE observer should remain active");
    assert_eq!(deleted_artifact_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(deleted_sha256, "c".repeat(64));
    assert_eq!(
        authorization.as_deref(),
        Some("Bearer temp-media-process-token")
    );
    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    {
        let (provider_job_id, authorization) =
            tokio::time::timeout(PROCESS_TIMEOUT, provider_delete_rx.recv())
                .await
                .expect("provider DELETE should reach the loopback OpenAI Videos adapter")
                .expect("provider DELETE observer should remain active");
        assert_eq!(provider_job_id, "provider-job-owner-lost");
        assert_eq!(
            authorization.as_deref(),
            Some("Bearer provider-cleanup-key")
        );
    }
    // Startup recovery intentionally interrupts tasks that were already active.
    // Create this task after the process is ready so the test exercises the
    // live Host's snapshot-only subscription path rather than lifecycle drain.
    let active = reserve_generation_task(
        &db,
        generation_reservation_input(
            owner.id,
            project.id,
            Uuid::new_v4().to_string(),
            "server-process-active",
        ),
    )
    .await
    .unwrap()
    .task
    .checkpoint
    .task_id;
    let mut child_input = generation_reservation_input(
        owner.id,
        project.id,
        Uuid::new_v4().to_string(),
        "server-process-active-child",
    );
    child_input.business_key_json = json!({
        "version": 1,
        "project_id": project.id,
        "operation": "server-process-active-child",
    });
    child_input.input_fingerprint_json = json!({"prompt": "server-process-active-child"});
    child_input.root_task_id = Some(active.clone());
    child_input.parent_task_id = Some(active.clone());
    child_input.item_key = Some("child-1".to_string());
    let child = reserve_generation_task(&db, child_input)
        .await
        .unwrap()
        .task
        .checkpoint
        .task_id;
    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let owner_token =
        login_access_token(&client, addr, &format!("generation_process_owner_{suffix}")).await;
    let outsider_token = login_access_token(
        &client,
        addr,
        &format!("generation_process_outsider_{suffix}"),
    )
    .await;

    let snapshot_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/snapshot?attempt=1"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), reqwest::StatusCode::OK);
    let snapshot = snapshot_response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(snapshot["task_id"], active);
    assert_eq!(snapshot["covers_through_sequence"], 1);
    assert_eq!(snapshot["status"], "reserved");

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/events?attempt=1"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let error = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error["code"], "generation_snapshot_required");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/events?attempt=1"
        ))
        .header("Last-Event-ID", "1:1")
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);

    let cancel_response = client
        .post(format!(
            "http://{addr}/api/generation-tasks/{active}/cancel"
        ))
        .bearer_auth(&owner_token)
        .json(&json!({"task_id": active, "attempt": 1}))
        .send()
        .await
        .unwrap();
    let cancel_status = cancel_response.status();
    let cancel_body = cancel_response.text().await.unwrap();
    assert_eq!(
        cancel_status,
        reqwest::StatusCode::ACCEPTED,
        "{cancel_body}"
    );
    let cancel_handle = serde_json::from_str::<serde_json::Value>(&cancel_body).unwrap();
    assert_eq!(cancel_handle["task_id"], active);
    assert_eq!(cancel_handle["status"], "cancelled");

    let sse_payload = tokio::time::timeout(PROCESS_TIMEOUT, async {
        let mut body = String::new();
        loop {
            let chunk = events_response
                .chunk()
                .await
                .unwrap()
                .expect("generation SSE closed before cancelled terminal");
            body.push_str(&String::from_utf8_lossy(&chunk));
            if body.contains("\"kind\":\"cancelled\"") {
                return body;
            }
        }
    })
    .await
    .unwrap();
    let sse_events = sse_payload
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str::<serde_json::Value>(data).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        sse_events.len(),
        2,
        "cancel request must emit one ack and one terminal"
    );
    assert_eq!(sse_events[0]["sequence"], 2);
    assert_eq!(sse_events[0]["event"]["kind"], "cancel_requested");
    assert_eq!(sse_events[1]["sequence"], 3);
    assert_eq!(sse_events[1]["event"]["kind"], "cancelled");
    drop(events_response);

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/snapshot?attempt=1"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let cancelled_snapshot = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(cancelled_snapshot["status"], "cancelled");
    assert_eq!(cancelled_snapshot["covers_through_sequence"], 3);
    assert_eq!(cancelled_snapshot["terminal_event"]["sequence"], 3);
    assert_eq!(
        cancelled_snapshot["terminal_event"]["event"]["kind"],
        "cancelled"
    );

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{child}/snapshot?attempt=1"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let child_snapshot = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(child_snapshot["status"], "interrupted");
    assert_eq!(child_snapshot["covers_through_sequence"], 258);
    assert_eq!(child_snapshot["terminal_event"]["sequence"], 258);
    assert_eq!(
        child_snapshot["terminal_event"]["event"]["kind"],
        "interrupted"
    );

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/events?attempt=1"
        ))
        .header("Last-Event-ID", "1:3")
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let error = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error["code"], "generation_snapshot_required");

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{startup_recovered}/snapshot?attempt=1"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let terminal_snapshot = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(terminal_snapshot["status"], "interrupted");
    let terminal_covers = terminal_snapshot["covers_through_sequence"]
        .as_u64()
        .unwrap();
    assert_eq!(terminal_covers, 258);
    let cleanup_rows = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::TaskId.eq(startup_recovered.clone()))
        .all(&db)
        .await
        .unwrap();
    let expected_cleanup_rows = if cfg!(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    )) {
        4
    } else {
        3
    };
    assert_eq!(cleanup_rows.len(), expected_cleanup_rows);
    assert_eq!(
        cleanup_rows
            .iter()
            .filter(|row| row.cleanup_kind == "generation_provider_job_cleanup")
            .count(),
        expected_cleanup_rows - 1
    );
    assert_eq!(
        cleanup_rows
            .iter()
            .filter(|row| row.cleanup_kind == "generation_staging_artifact_delete")
            .count(),
        1
    );
    let staging_cleanup = cleanup_rows
        .iter()
        .find(|row| row.cleanup_kind == "generation_staging_artifact_delete")
        .unwrap();
    assert_eq!(staging_cleanup.status, "completed");
    assert_eq!(staging_cleanup.outcome.as_deref(), Some("deleted"));
    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    {
        let provider_delete = cleanup_rows
            .iter()
            .find(|row| row.payload_json["operation"] == "delete")
            .expect("completed reconcile must schedule one provider delete");
        assert_eq!(provider_delete.status, "completed");
        assert_eq!(provider_delete.outcome.as_deref(), Some("deleted"));
    }
    let cleanup_payloads = cleanup_rows
        .iter()
        .map(|row| row.payload_json.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(cleanup_payloads.contains("provider-job-owner-lost"));
    assert!(cleanup_payloads.contains("0123456789abcdef0123456789abcdef"));
    assert!(!cleanup_payloads.contains("file_path"));
    assert!(!cleanup_payloads.contains("/home/"));

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{startup_recovered}/events?attempt=1"
        ))
        .header("Last-Event-ID", format!("1:{terminal_covers}"))
        .bearer_auth(&owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let error = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error["code"], "generation_snapshot_required");

    let response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{active}/snapshot?attempt=1"
        ))
        .bearer_auth(&outsider_token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    let error = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(error["code"], "generation_task_forbidden");

    stop_server(server);
    let restarted_addr = unused_local_addr();
    let mut command = server_command(&database_url, &data_dir, &frontend_dir, restarted_addr);
    let mut restarted = command
        .env("A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL", &remote_temp_upload_url)
        .env(
            "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN",
            "temp-media-process-token",
        )
        .spawn()
        .expect("restart a4n-server generation recovery process test");
    wait_for_listener(&mut restarted, restarted_addr).await;
    stop_server(restarted);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), delete_rx.recv())
            .await
            .is_err(),
        "已完成的 staging delete 在 Server 重启后被重复执行"
    );
    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    assert!(
        tokio::time::timeout(Duration::from_millis(100), provider_delete_rx.recv())
            .await
            .is_err(),
        "已完成的 provider delete 在 Server 重启后被重复执行"
    );
    assert_eq!(
        cleanup_outbox::Entity::find()
            .filter(cleanup_outbox::Column::TaskId.eq(startup_recovered))
            .all(&db)
            .await
            .unwrap()
            .len(),
        expected_cleanup_rows,
        "重复恢复不得重复插入 provider/staging cleanup intent"
    );
    let _ = delete_shutdown.send(());
    tokio::time::timeout(Duration::from_secs(5), delete_task)
        .await
        .expect("remote staging DELETE observer should stop before timeout")
        .expect("remote staging DELETE observer task should join")
        .expect("remote staging DELETE observer should stop cleanly");
    #[cfg(any(
        feature = "embedded-capabilities",
        feature = "embedded-media-providers"
    ))]
    {
        let _ = provider_shutdown.send(());
        tokio::time::timeout(Duration::from_secs(5), provider_task)
            .await
            .expect("provider DELETE observer should stop before timeout")
            .expect("provider DELETE observer task should join")
            .expect("provider DELETE observer should stop cleanly");
    }
}

#[tokio::test]
async fn server_process_streams_chapter_generation_failure_terminal() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("chapter_generation_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("chapter generation {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    novel_blueprint::ActiveModel {
        project_id: Set(project.id),
        title: Set("测试蓝图".to_string()),
        genre: Set("测试".to_string()),
        style: Set(None),
        tone: Set(None),
        target_audience: Set(None),
        one_sentence_summary: Set(Some("用于进程链测试".to_string())),
        full_synopsis: Set(Some("用于进程链测试".to_string())),
        world_setting: Set(None),
        current_version: Set("1.0.0".to_string()),
        needs_part_outlines: Set(false),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(Some(1)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        summary: Set(Some("用于进程链测试".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(Some(100)),
        status: Set(chapter_outline::STATUS_DRAFT.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("chapter-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let start_request_id = Uuid::new_v4().to_string();
    let start_input = json!({
        "meta": {
            "start_request_id": start_request_id,
            "retry_of_task_id": null
        },
        "project_id": project.id,
        "chapter_number": 1,
        "writing_notes": null,
        "min_word_count": 100,
        "max_word_count": 200,
        "client_rag_context": null
    });
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/chapter"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}/events?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    let events = read_generation_sse_until_terminal(&mut events_response).await;
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let failed = terminal_events[0];
    assert_eq!(failed, events.last().unwrap());
    assert_eq!(failed["event"]["kind"], "failed");
    assert_eq!(
        failed["event"]["data"]["error"]["code"], "provider_job_failed",
        "unexpected chapter terminal: {failed}"
    );
    assert_eq!(
        failed["event"]["data"]["error"]["stage_code"],
        "provider_connect"
    );
    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "terminal 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "failed");
    assert_eq!(summary["result_ref"], serde_json::Value::Null);
    assert_eq!(&summary["terminal_event"], failed);

    let retry_response = client
        .post(format!("http://{addr}/api/generation-tasks/chapter"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(retry_response.status(), reqwest::StatusCode::OK);
    let retry_handle = retry_response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(retry_handle["task_id"], task_id);
    assert_eq!(retry_handle["attempt"], 1);
    assert_eq!(retry_handle["status"], "failed");

    let tasks = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, "failed");
    assert!(tasks[0].result_kind.is_none());
    assert!(
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project.id))
            .filter(chapter::Column::ChapterNumber.eq(1))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    stop_server(server);
}

#[tokio::test]
async fn server_process_streams_outline_generation_failure_terminal() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("outline_generation_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("outline generation {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    novel_blueprint::ActiveModel {
        project_id: Set(project.id),
        title: Set("测试蓝图".to_string()),
        genre: Set("测试".to_string()),
        style: Set(None),
        tone: Set(None),
        target_audience: Set(None),
        one_sentence_summary: Set(Some("用于大纲进程链测试".to_string())),
        full_synopsis: Set(Some("用于大纲进程链测试".to_string())),
        world_setting: Set(None),
        current_version: Set("1.0.0".to_string()),
        needs_part_outlines: Set(true),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(Some(1)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("outline-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let start_input = json!({
        "meta": {
            "start_request_id": Uuid::new_v4().to_string(),
            "retry_of_task_id": null
        },
        "project_id": project.id,
        "operation": {
            "kind": "generate_parts",
            "part_count": 1,
            "chapters_per_part": 1,
            "variables": null
        }
    });
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/outline"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}/events?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    let events = read_generation_sse_until_terminal(&mut events_response).await;
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let failed = terminal_events[0];
    assert_eq!(failed, events.last().unwrap());
    assert_eq!(failed["event"]["kind"], "failed");
    assert_eq!(
        failed["event"]["data"]["error"]["code"], "provider_job_failed",
        "unexpected outline terminal: {failed}"
    );
    assert_eq!(
        failed["event"]["data"]["error"]["stage_code"],
        "outline_generate_parts"
    );
    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "terminal 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "failed");
    assert_eq!(summary["result_ref"], serde_json::Value::Null);
    assert_eq!(&summary["terminal_event"], failed);

    let retry_response = client
        .post(format!("http://{addr}/api/generation-tasks/outline"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(retry_response.status(), reqwest::StatusCode::OK);
    let retry_handle = retry_response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(retry_handle["task_id"], task_id);
    assert_eq!(retry_handle["attempt"], 1);
    assert_eq!(retry_handle["status"], "failed");

    let tasks = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, "failed");
    assert!(tasks[0].result_kind.is_none());
    assert!(
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    stop_server(server);
}

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
#[tokio::test]
async fn server_process_reconnects_inspiration_sse_ten_times_without_losing_one_mib() {
    const REPLY_BYTES: usize = 1024 * 1024;
    const PROVIDER_FRAME_BYTES: usize = 1024;
    const DISCONNECTS: usize = 10;
    const EVENTS_PER_CONNECTION: usize = 1;

    let reply = "x".repeat(REPLY_BYTES);
    let expected_raw_json = format!(r#"{{"reply":"{reply}","is_complete":false}}"#);
    let delta_frame = |content: &str| {
        format!(
            "data: {}\n\n",
            json!({
                "choices": [{
                    "index": 0,
                    "delta": {"content": content},
                    "finish_reason": null
                }]
            })
        )
    };
    let mut frames = Vec::with_capacity(REPLY_BYTES / PROVIDER_FRAME_BYTES + 5);
    frames.push(delta_frame(r#"{"reply":""#));
    frames.extend(
        reply
            .as_bytes()
            .chunks(PROVIDER_FRAME_BYTES)
            .map(|chunk| delta_frame(std::str::from_utf8(chunk).unwrap())),
    );
    frames.push(delta_frame(r#"","is_complete":false}"#));
    frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 3,
                "completion_tokens": 4,
                "total_tokens": 7
            }
        })
    ));
    frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        })
    ));
    frames.push("data: [DONE]\n\n".to_string());
    let provider =
        FakeOpenAiSseProvider::start_delayed(frames, Duration::from_millis(2), Vec::new()).await;
    let expected_reply_sha256 = format!("{:x}", sha2::Sha256::digest(reply.as_bytes()));
    let expected_raw_sha256 = format!("{:x}", sha2::Sha256::digest(expected_raw_json.as_bytes()));
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("inspiration_generation_success_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("inspiration generation success {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("inspiration-process-success-{suffix}"),
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 5,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/inspiration"))
        .bearer_auth(&token)
        .json(&json!({
            "meta": {
                "start_request_id": Uuid::new_v4().to_string(),
                "retry_of_task_id": null
            },
            "project_id": project.id,
            "message": "请帮我梳理主角动机",
            "variables": {}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let events_url = format!("http://{addr}/api/generation-tasks/{task_id}/events?attempt=1");
    let mut events = Vec::new();
    let mut cursor = 0;
    for disconnect_index in 0..DISCONNECTS {
        let mut request = client.get(&events_url).bearer_auth(&token);
        if cursor > 0 {
            request = request.header("Last-Event-ID", format!("1:{cursor}"));
        }
        let mut events_response = request.send().await.unwrap();
        assert_eq!(
            events_response.status(),
            reqwest::StatusCode::OK,
            "第 {} 次 SSE 订阅未成功",
            disconnect_index + 1
        );
        let batch = read_generation_sse_events(&mut events_response, EVENTS_PER_CONNECTION).await;
        assert_eq!(
            batch.len(),
            EVENTS_PER_CONNECTION,
            "第 {} 次主动断线前事件不足",
            disconnect_index + 1
        );
        assert!(
            batch
                .iter()
                .all(|event| !generation_event_is_terminal(event)),
            "第 {} 次主动断线前任务已终态",
            disconnect_index + 1
        );
        for event in batch {
            let expected_sequence = u64::try_from(events.len() + 1).unwrap();
            assert_eq!(event["sequence"], expected_sequence);
            cursor = expected_sequence;
            events.push(event);
        }
        drop(events_response);
    }

    let mut events_response = client
        .get(&events_url)
        .bearer_auth(&token)
        .header("Last-Event-ID", format!("1:{cursor}"))
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    for event in read_generation_sse_until_terminal(&mut events_response).await {
        let expected_sequence = u64::try_from(events.len() + 1).unwrap();
        assert_eq!(event["sequence"], expected_sequence);
        events.push(event);
    }
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let kinds = events
        .iter()
        .filter_map(|event| event["event"]["kind"].as_str())
        .collect::<Vec<_>>();
    assert!(
        kinds.contains(&"text_delta"),
        "成功流没有正文增量: {kinds:?}"
    );
    assert!(kinds.contains(&"usage"), "成功流没有 usage: {kinds:?}");
    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let completed = terminal_events[0];
    assert_eq!(completed, events.last().unwrap());
    assert_eq!(completed["event"]["kind"], "completed");
    assert_eq!(completed["event"]["data"]["finish_reason"], "stop");
    assert_eq!(completed["event"]["data"]["final_usage"]["total_tokens"], 7);
    assert_eq!(
        completed["event"]["data"]["result_ref"]["kind"],
        "inspiration_message"
    );
    assert_eq!(
        completed["event"]["data"]["result_ref"]["revision"],
        expected_raw_sha256
    );
    assert_eq!(
        completed["event"]["data"]["final_text_hashes"][0]["sha256"],
        expected_raw_sha256
    );

    let streamed_raw_json = events
        .iter()
        .filter(|event| {
            event["event"]["kind"] == "text_delta"
                && event["event"]["data"]["stream_id"] == "raw_json"
        })
        .filter_map(|event| event["event"]["data"]["delta"].as_str())
        .collect::<String>();
    assert_eq!(streamed_raw_json, expected_raw_json);
    let streamed_reply =
        serde_json::from_str::<serde_json::Value>(&streamed_raw_json).unwrap()["reply"]
            .as_str()
            .unwrap()
            .to_string();
    assert_eq!(streamed_reply.len(), REPLY_BYTES);
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(streamed_reply.as_bytes())),
        expected_reply_sha256
    );

    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "completed 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "completed");
    assert_eq!(
        summary["result_ref"],
        completed["event"]["data"]["result_ref"]
    );
    assert_eq!(&summary["terminal_event"], completed);

    let messages = a4n_db::inspiration::inspiration_messages(&db, project.id, 10)
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .any(|message| message.role == "assistant" && message.content == expected_raw_json)
    );
    assert!(messages.iter().any(|message| {
        message.role == "user" && message.content == "请帮我梳理主角动机"
    }));
    eprintln!(
        "generation SSE reconnects={DISCONNECTS} reply_bytes={REPLY_BYTES} sha256={expected_reply_sha256} events={}",
        events.len()
    );

    stop_server(server);
    provider.stop().await;
}

/// 真实 Chromium 使用生产 generation client 经测试代理断线，再按 cursor 自动续订。
#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
#[tokio::test]
async fn real_chromium_recovers_generation_after_proxy_aborts_sse() {
    const REPLY_BYTES: usize = 64 * 1024;
    const PROVIDER_FRAME_BYTES: usize = 512;
    const CAPACITY_UPSTREAM_CHUNKS: usize = 10_000;
    const CAPACITY_TOTAL_BYTES: usize = 45_000;
    const CAPACITY_EVENT_BUDGET: u64 = 1_000;
    const CAPACITY_REACT_COMMIT_BUDGET: u64 = 300;
    const WEB_UI_MEDIAN_BUDGET_MS: u64 = 250;
    const TAURI_UI_MEDIAN_BUDGET_MS: u64 = 175;
    const CANCEL_P95_BUDGET_MS: u64 = 500;

    let reply = "y".repeat(REPLY_BYTES);
    let expected_raw_json = format!(r#"{{"reply":"{reply}","is_complete":false}}"#);
    let delta_frame = |content: &str| {
        format!(
            "data: {}\n\n",
            json!({
                "choices": [{
                    "index": 0,
                    "delta": {"content": content},
                    "finish_reason": null
                }]
            })
        )
    };
    let mut frames = Vec::with_capacity(REPLY_BYTES / PROVIDER_FRAME_BYTES + 5);
    frames.push(delta_frame(r#"{"reply":""#));
    frames.extend(
        reply
            .as_bytes()
            .chunks(PROVIDER_FRAME_BYTES)
            .map(|chunk| delta_frame(std::str::from_utf8(chunk).unwrap())),
    );
    frames.push(delta_frame(r#"","is_complete":false}"#));
    frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 2,
                "completion_tokens": 3,
                "total_tokens": 5
            }
        })
    ));
    frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        })
    ));
    frames.push("data: [DONE]\n\n".to_string());
    let empty_capacity_raw_json = r#"{"reply":"","is_complete":false}"#;
    let capacity_raw_json = format!(
        r#"{{"reply":"{}","is_complete":false}}"#,
        "z".repeat(CAPACITY_TOTAL_BYTES - empty_capacity_raw_json.len())
    );
    assert_eq!(capacity_raw_json.len(), CAPACITY_TOTAL_BYTES);
    let mut capacity_frames = Vec::with_capacity(CAPACITY_UPSTREAM_CHUNKS + 3);
    let mut offset = 0;
    for index in 0..CAPACITY_UPSTREAM_CHUNKS {
        let next = offset + index % 8 + 1;
        capacity_frames.push(delta_frame(&capacity_raw_json[offset..next]));
        offset = next;
    }
    assert_eq!(offset, capacity_raw_json.len());
    capacity_frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 11,
                "completion_tokens": 12,
                "total_tokens": 23
            }
        })
    ));
    capacity_frames.push(format!(
        "data: {}\n\n",
        json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        })
    ));
    capacity_frames.push("data: [DONE]\n\n".to_string());
    let product_page_reply = "真实产品页正式回复";
    let product_page_raw_json = json!({
        "ai_message": product_page_reply,
        "next_question": null,
        "progress_summary": null,
        "ui_control": {
            "type": "text_input",
            "placeholder": "继续补充你的想法",
        },
        "conversation_state": {},
        "is_complete": false,
    })
    .to_string();
    let product_page_frames = vec![
        delta_frame(&product_page_raw_json),
        format!(
            "data: {}\n\n",
            json!({
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            })
        ),
        "data: [DONE]\n\n".to_string(),
    ];
    let tauri_remount_reply = "真实 Wry 首字重挂载完成";
    let tauri_remount_raw_json = format!(
        r#"{{"ai_message":"{tauri_remount_reply}","next_question":null,"progress_summary":null,"ui_control":{{"type":"text_input","placeholder":"继续补充你的想法"}},"conversation_state":{{}},"is_complete":false}}"#
    );
    let tauri_remount_split = tauri_remount_raw_json
        .find("重挂载完成")
        .expect("Tauri remount fixture must contain a visible first chunk");
    let tauri_destroy_reply = "真实 Wry 销毁清理首字后续内容";
    let tauri_destroy_raw_json = format!(
        r#"{{"ai_message":"{tauri_destroy_reply}","next_question":null,"progress_summary":null,"ui_control":{{"type":"text_input","placeholder":"继续补充你的想法"}},"conversation_state":{{}},"is_complete":false}}"#
    );
    let tauri_destroy_split = tauri_destroy_raw_json
        .find("后续内容")
        .expect("Tauri destroy fixture must contain a visible first chunk");
    let terminal_frames = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }]
        })
    );
    let tauri_remount_frames = vec![
        delta_frame(&tauri_remount_raw_json[..tauri_remount_split]),
        delta_frame(&tauri_remount_raw_json[tauri_remount_split..]),
        terminal_frames.clone(),
    ];
    let tauri_destroy_frames = vec![
        delta_frame(&tauri_destroy_raw_json[..tauri_destroy_split]),
        delta_frame(&tauri_destroy_raw_json[tauri_destroy_split..]),
        terminal_frames,
    ];
    let blueprint_stage_frames = |value: serde_json::Value| {
        vec![
            delta_frame(&value.to_string()),
            format!(
                "data: {}\n\n",
                json!({
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }]
                })
            ),
            "data: [DONE]\n\n".to_string(),
        ]
    };
    let blueprint_core_frames = blueprint_stage_frames(json!({
        "title": "蓝图产品页验证",
        "target_audience": "成年读者",
        "genre": "悬疑",
        "style": "克制写实",
        "tone": "紧张",
        "one_sentence_summary": "档案员追查被篡改的城市记忆。",
        "full_synopsis": "档案员在调查失踪案时发现城市记忆被系统性篡改，并决定公开真相。",
    }));
    let blueprint_world_frames = blueprint_stage_frames(json!({
        "world_setting": {
            "premise": "城市以记忆档案维持秩序。",
            "core_rules": "记忆可封存但修改会留下审计痕迹。",
            "technology_or_magic": {
                "source": "档案技术",
                "rules": ["封存需要本人授权"],
                "limits": ["无法复制完整情绪"],
                "costs": ["读取会损耗短期记忆"]
            },
            "key_locations": [{
                "name": "中央档案馆",
                "description": "保存全城记忆的设施",
                "story_function": "调查核心现场"
            }],
            "factions": [{
                "name": "审校局",
                "description": "管理档案访问",
                "agenda": "隐瞒旧案",
                "representative_characters": ["顾言"]
            }],
            "history_timeline": [{
                "period": "十年前",
                "event": "全城档案迁移",
                "impact": "旧案记录出现缺口"
            }],
            "culture_and_society": {
                "values": "秩序优先",
                "class_structure": "按档案权限分层",
                "daily_life": "居民凭记忆凭证办理事务"
            },
            "conflict_sources": [{
                "name": "真相与秩序",
                "description": "公开旧案会动摇制度",
                "stakes": "全城信任可能崩塌"
            }],
            "mysteries_and_foreshadowing": [{
                "mystery": "谁修改了旧案",
                "early_clues": ["缺失的审计编号"],
                "payoff_direction": "审校局内部人员参与"
            }],
            "genre_promises": ["档案悬疑"]
        }
    }));
    let blueprint_characters_frames = blueprint_stage_frames(json!({
        "characters": [
            {
                "name": "林澈",
                "character_type": "protagonist",
                "gender": "男",
                "age": "27",
                "appearance": "黑发灰眼",
                "personality": "谨慎执拗",
                "background": "城市档案员",
                "motivation": "查明失踪案",
                "arc_summary": "从守序者转为真相公开者",
                "relationship_to_protagonist": "本人",
                "arc_stages": {
                    "start": "相信制度",
                    "core_wound": "未能救下同伴",
                    "desire": "证明旧案无误",
                    "need": "承认制度缺陷",
                    "secret": "曾签署封存令",
                    "voice_style": "克制短句",
                    "turning_point": "发现自己的签名",
                    "end": "公开真相"
                }
            },
            {
                "name": "许灯",
                "character_type": "supporting",
                "gender": "女",
                "age": "25",
                "appearance": "短发义眼",
                "personality": "直率敏锐",
                "background": "记忆修复师",
                "motivation": "偿还旧债",
                "arc_summary": "从旁观者成为盟友",
                "relationship_to_protagonist": "盟友",
                "arc_stages": {
                    "start": "只为报酬行动",
                    "core_wound": "救援失败",
                    "desire": "离开城市",
                    "need": "重新承担责任",
                    "secret": "修复过旧案证词",
                    "voice_style": "快速直接",
                    "turning_point": "留下关键证据",
                    "end": "共同公开档案"
                }
            },
            {
                "name": "顾言",
                "character_type": "antagonist",
                "gender": "男",
                "age": "45",
                "appearance": "银框眼镜",
                "personality": "温和强势",
                "background": "审校局主管",
                "motivation": "维持城市秩序",
                "arc_summary": "从保护者暴露为操控者",
                "relationship_to_protagonist": "对手",
                "arc_stages": {
                    "start": "提供有限协助",
                    "core_wound": "经历城市失序",
                    "desire": "控制全部档案",
                    "need": "面对谎言代价",
                    "secret": "主导旧案修改",
                    "voice_style": "礼貌缓慢",
                    "turning_point": "审计记录曝光",
                    "end": "失去权力"
                }
            }
        ]
    }));
    let blueprint_relationships_frames = blueprint_stage_frames(json!({
        "relationships": [{
            "relationship_type": "rival",
            "character_a": "林澈",
            "character_b": "顾言",
            "description": "真相公开与秩序维持的对立",
            "strength": 8,
            "evolution": {
                "start": "互相试探",
                "tension": "调查不断逼近旧案",
                "turning_points": ["审计记录曝光"],
                "hidden_truth": "顾言主导旧案修改",
                "story_function": "推动制度冲突",
                "end": "公开决裂"
            }
        }]
    }));
    let chapter_content = "雨停后，林澈走进中央档案馆。值夜灯沿长廊逐盏亮起，像有人在黑暗里为他标记路线。他调出失踪案卷，发现每一份证词都缺少同一个审计编号。许灯把修复仪接上终端，残留数据随即显出顾言的签名。门外传来脚步声，林澈关闭屏幕，却把证据写入了离线存储。他终于明白，这起失踪案并非记录错误，而是一场持续十年的系统性篡改。";
    let chapter_frames = vec![
        delta_frame(chapter_content),
        format!(
            "data: {}\n\n",
            json!({
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            })
        ),
        "data: [DONE]\n\n".to_string(),
    ];
    let optimize_reply = "正式优化结论：本章线索递进清楚，结尾已经形成有效悬念。";
    let optimize_frames = blueprint_stage_frames(json!({
        "thought": "核对章节叙事效果并直接回复",
        "public_update": null,
        "action": "final",
        "arguments": {"reply": optimize_reply},
    }));
    let provider = FakeOpenAiSseProvider::start_delayed(
        frames,
        Duration::from_millis(20),
        vec![
            (
                "请生成故事核心定位".to_string(),
                blueprint_core_frames,
                Duration::from_millis(5),
            ),
            (
                "请基于故事核心生成完整 world_setting".to_string(),
                blueprint_world_frames,
                Duration::from_millis(5),
            ),
            (
                "请基于故事核心和世界观生成主要角色".to_string(),
                blueprint_characters_frames,
                Duration::from_millis(5),
            ),
            (
                "请基于故事核心、世界观和角色档案生成角色关系网络".to_string(),
                blueprint_relationships_frames,
                Duration::from_millis(5),
            ),
            (
                "产品页优化核对".to_string(),
                optimize_frames,
                Duration::from_millis(5),
            ),
            (
                "writing_chapter_outline".to_string(),
                chapter_frames,
                Duration::from_millis(5),
            ),
            (
                "product-page-".to_string(),
                product_page_frames,
                Duration::from_millis(20),
            ),
            (
                "tauri-wry-remount".to_string(),
                tauri_remount_frames,
                Duration::from_millis(1_500),
            ),
            (
                "tauri-wry-destroyed".to_string(),
                tauri_destroy_frames,
                Duration::from_secs(5),
            ),
            (
                "capacity-10000-chunks".to_string(),
                capacity_frames,
                Duration::ZERO,
            ),
        ],
    )
    .await;
    #[cfg(feature = "embedded-media-providers")]
    let image_provider = FakeOpenAiImageProvider::start().await;
    let expected_raw_sha256 = format!("{:x}", sha2::Sha256::digest(expected_raw_json.as_bytes()));
    let expected_capacity_sha256 =
        format!("{:x}", sha2::Sha256::digest(capacity_raw_json.as_bytes()));
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("inspiration_proxy_abort_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("inspiration proxy abort {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("inspiration-proxy-abort-{suffix}"),
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 5,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();
    #[cfg(feature = "embedded-media-providers")]
    let image_config = {
        let config = a4n_db::config::create_image_config_record(
            &db,
            a4n_db::config::ImageConfigCreateRecordInput {
                name: format!("browser-process-image-{suffix}"),
                backend: "openai-compatible-image".to_string(),
                base_url: image_provider.base_url(),
                api_key_encrypted: Some("browser-process-key".to_string()),
                model: "browser-process-image-model".to_string(),
                max_concurrent: 1,
                default_width: 1024,
                default_height: 1024,
                provider_options_json: None,
            },
        )
        .await
        .unwrap();
        a4n_db::config::set_default_image_config_record(&db, config)
            .await
            .unwrap()
    };

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;
    let (proxy_addr, mut proxy_requests, proxy_fault, proxy_fault_hits, proxy_shutdown, proxy_task) =
        spawn_abort_once_sse_proxy(addr).await;
    let vite_addr = SocketAddr::from(([127, 0, 0, 1], 5173));
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let vite_port = vite_addr.port().to_string();
    let mut vite = Command::new(workspace_root.join("frontend/node_modules/.bin/vite"))
        .args(["--host", "127.0.0.1", "--port", &vite_port, "--strictPort"])
        .current_dir(workspace_root.join("frontend"))
        .env("A4N_DISABLE_VITE_HMR", "1")
        .env("VITE_A4N_API_BASE_URL", format!("http://{proxy_addr}/api"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_listener(&mut vite, vite_addr).await;

    let mut browser_url = format!(
        "http://{vite_addr}/process-tests/generation.html#backend=http://{addr}&username={username}&password=generation-process-password&project={}",
        project.id
    );
    #[cfg(feature = "embedded-media-providers")]
    browser_url.push_str(&format!("&image_config={}", image_config.id));
    let webdriver_addr = unused_local_addr();
    let mut webdriver = Command::new(chromedriver_binary())
        .arg(format!("--port={}", webdriver_addr.port()))
        .arg("--allowed-ips=")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_listener(&mut webdriver, webdriver_addr).await;
    let webdriver_url = format!("http://{webdriver_addr}");
    let browser = reqwest::Client::new();
    let session: serde_json::Value = browser
        .post(format!("{webdriver_url}/session"))
        .json(&json!({
            "capabilities": {"alwaysMatch": {
                "browserName": "chrome",
                "goog:chromeOptions": {"args": [
                    "--headless=new",
                    "--disable-gpu",
                    "--disable-dev-shm-usage",
                    "--no-sandbox",
                    "--window-size=1440,1000",
                    format!("--user-data-dir={}", temp.path().join("chromium-profile").display()),
                ]},
            }},
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let session_id = session["value"]["sessionId"].as_str().unwrap();
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": browser_url}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let browser_dom = tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let source: serde_json::Value = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("data-a4n-result=\"") || dom.contains("data-a4n-error=\"") {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("browser harness did not reach a result before timeout");
    let product_message = format!("product-page-generation-{suffix}");
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/inspiration/{}", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(&project.title) && dom.contains("chat-composer") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("inspiration product page did not become interactive before timeout");
    webdriver_send_inspiration_message(&browser, &webdriver_url, session_id, &product_message)
        .await;
    let product_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(&product_message)
                && dom.contains(product_page_reply)
                && !dom.contains("发送中")
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("inspiration product page generation did not complete before timeout");
    assert!(product_dom.contains("灵感对话"));
    const CONSISTENCY_WARNING: &str = "生成结果一致性校验失败，以下内容来自正式消息，只供核对";
    assert!(!product_dom.contains(CONSISTENCY_WARNING));
    let (product_task, product_assistant_id) = assert_completed_inspiration_exchange(
        &db,
        project.id,
        &product_message,
        &product_page_raw_json,
    )
    .await;
    eprintln!(
        "real inspiration product page task={} message={} assistant={}",
        product_task.task_id, product_message, product_assistant_id
    );

    let consistency_message = format!("product-page-consistency-{suffix}");
    proxy_fault_hits.store(0, std::sync::atomic::Ordering::SeqCst);
    proxy_fault.store(PROXY_FAULT_CONSISTENCY, std::sync::atomic::Ordering::SeqCst);
    webdriver_send_inspiration_message(&browser, &webdriver_url, session_id, &consistency_message)
        .await;
    let consistency_dom = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(&consistency_message)
                && dom.contains(product_page_reply)
                && dom.contains(CONSISTENCY_WARNING)
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    let consistency_dom = match consistency_dom {
        Ok(dom) => dom,
        Err(_) => {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            let context = dom
                .find(&consistency_message)
                .map(|index| dom[index..].chars().take(2_000).collect::<String>())
                .unwrap_or_default();
            panic!(
                "inspiration consistency page timed out: mutations={} user={} reply={} warning={} transport_error={} finalization_error={} sending={} context={context}",
                proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst),
                dom.contains(&consistency_message),
                dom.contains(product_page_reply),
                dom.contains(CONSISTENCY_WARNING),
                dom.contains("生成结果一致性校验失败，请从正式结果重新加载"),
                dom.contains("灵感对话结果校验失败"),
                dom.contains("发送中"),
            );
        }
    };
    assert!(proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    proxy_fault.store(PROXY_FAULT_NONE, std::sync::atomic::Ordering::SeqCst);
    assert!(consistency_dom.contains("灵感对话"));
    let (consistency_task, consistency_assistant_id) = assert_completed_inspiration_exchange(
        &db,
        project.id,
        &consistency_message,
        &product_page_raw_json,
    )
    .await;
    assert_ne!(consistency_task.task_id, product_task.task_id);
    eprintln!(
        "real inspiration consistency page task={} message={} assistant={}",
        consistency_task.task_id, consistency_message, consistency_assistant_id
    );
    for (fault, label, expected_text) in [
        (
            PROXY_FAULT_EXPIRED_WITH_REF,
            "expired-with-ref",
            "生成任务恢复快照已过期，以下内容来自正式消息，只供核对",
        ),
        (
            PROXY_FAULT_EXPIRED_WITHOUT_REF,
            "expired-without-ref",
            "生成任务恢复快照已过期，且没有正式结果引用，无法恢复",
        ),
        (
            PROXY_FAULT_CORRUPT_WITH_REF,
            "corrupt-with-ref",
            "生成任务恢复快照已损坏，以下内容来自正式消息，只供核对",
        ),
        (
            PROXY_FAULT_CORRUPT_WITHOUT_REF,
            "corrupt-without-ref",
            "生成任务恢复快照已损坏，且没有正式结果引用，无法恢复",
        ),
    ] {
        let message = format!("product-page-{label}-{suffix}");
        proxy_fault_hits.store(0, std::sync::atomic::Ordering::SeqCst);
        proxy_fault.store(fault, std::sync::atomic::Ordering::SeqCst);
        webdriver_send_inspiration_message(&browser, &webdriver_url, session_id, &message).await;
        let recovery_dom = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let source = browser
                    .get(format!("{webdriver_url}/session/{session_id}/source"))
                    .send()
                    .await
                    .unwrap()
                    .json::<serde_json::Value>()
                    .await
                    .unwrap();
                let dom = source["value"].as_str().unwrap();
                if dom.contains(&message) && dom.contains(expected_text) && !dom.contains("发送中")
                {
                    break dom.to_string();
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("inspiration {label} page did not settle before timeout"));
        assert!(recovery_dom.contains("灵感对话"));
        assert!(proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst) >= 3);
        proxy_fault.store(PROXY_FAULT_NONE, std::sync::atomic::Ordering::SeqCst);
        let (task, assistant_id) = assert_completed_inspiration_exchange(
            &db,
            project.id,
            &message,
            &product_page_raw_json,
        )
        .await;
        eprintln!(
            "real inspiration recovery page mode={label} task={} assistant={assistant_id}",
            task.task_id
        );
    }
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/inspiration/{}", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(&project.title) && dom.contains("生成蓝图") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("inspiration page did not reload before blueprint product page test");
    webdriver_click_button(&browser, &webdriver_url, session_id, "生成蓝图").await;
    let blueprint_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("故事蓝图")
                && dom.contains("档案员在调查失踪案时发现城市记忆被系统性篡改")
                && dom.contains("确认进入项目")
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("blueprint product page generation did not complete before timeout");
    webdriver_click_dialog_button(&browser, &webdriver_url, session_id, "查看蓝图").await;
    assert!(!blueprint_dom.contains("正式蓝图只读核对"));
    let (blueprint_task, blueprint) = assert_completed_blueprint(
        &db,
        project.id,
        "档案员在调查失踪案时发现城市记忆被系统性篡改，并决定公开真相。",
    )
    .await;
    eprintln!(
        "real blueprint product page task={} blueprint={} version={}",
        blueprint_task.task_id, blueprint.id, blueprint.current_version
    );

    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/novel/{}", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/inspiration/{}", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let blueprint_remount = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("灵感对话") && dom.contains("生成蓝图") && dom.contains("<textarea")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    if blueprint_remount.is_err() {
        let source = browser
            .get(format!("{webdriver_url}/session/{session_id}/source"))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let url = browser
            .get(format!("{webdriver_url}/session/{session_id}/url"))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let dom = source["value"].as_str().unwrap();
        panic!(
            "inspiration page did not remount before blueprint recovery test: url={} page={} textarea={} generate={} generating={} confirm={} login={} dom={}",
            url["value"],
            dom.contains("灵感对话"),
            dom.contains("<textarea"),
            dom.contains("生成蓝图"),
            dom.contains("生成中"),
            dom.contains("确认进入项目"),
            dom.contains("登录"),
            dom.chars().take(2_000).collect::<String>(),
        );
    }
    proxy_fault_hits.store(0, std::sync::atomic::Ordering::SeqCst);
    proxy_fault.store(
        PROXY_FAULT_EXPIRED_WITH_REF,
        std::sync::atomic::Ordering::SeqCst,
    );
    webdriver_click_button(&browser, &webdriver_url, session_id, "生成蓝图").await;
    let blueprint_recovery_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("生成任务恢复快照已过期，当前展示正式蓝图，只供核对")
                && dom.contains("档案员在调查失踪案时发现城市记忆被系统性篡改")
                && dom.contains("正式蓝图只读核对")
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("blueprint expired recovery did not settle before timeout");
    webdriver_click_dialog_button(&browser, &webdriver_url, session_id, "关闭").await;
    assert!(!blueprint_recovery_dom.contains("确认进入项目"));
    assert!(proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    proxy_fault.store(PROXY_FAULT_NONE, std::sync::atomic::Ordering::SeqCst);
    let (blueprint_recovery_task, recovered_blueprint) = assert_completed_blueprint(
        &db,
        project.id,
        "档案员在调查失踪案时发现城市记忆被系统性篡改，并决定公开真相。",
    )
    .await;
    assert_ne!(
        recovered_blueprint.current_version,
        blueprint.current_version
    );
    eprintln!(
        "real blueprint recovery page task={} blueprint={} version={}",
        blueprint_recovery_task.task_id,
        recovered_blueprint.id,
        recovered_blueprint.current_version
    );

    let now = Utc::now().naive_utc();
    chapter_outline::ActiveModel {
        project_id: Set(project.id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("章节产品页验证".to_string())),
        summary: Set(Some("档案员发现失踪案卷的审计编号被系统性抹除".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(Some(100)),
        status: Set(chapter_outline::STATUS_DRAFT.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/write/{}?chapter=1", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("蓝图产品页验证") && dom.contains("章节产品页验证") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("writing desk did not load the seeded chapter outline before timeout");
    webdriver_click_button(&browser, &webdriver_url, session_id, "写作助手").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            if source["value"].as_str().unwrap().contains("开始生成") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("writing assistant did not expose chapter generation before timeout");
    webdriver_click_button(&browser, &webdriver_url, session_id, "开始生成").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            if source["value"].as_str().unwrap().contains("生成章节正文") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("chapter generation confirmation did not open before timeout");
    webdriver_click_dialog_button(&browser, &webdriver_url, session_id, "生成").await;
    let chapter_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(chapter_content) && dom.contains("章节正文生成完成") {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("chapter product page generation did not complete before timeout");
    assert!(!chapter_dom.contains("正式章节一致性校验异常"));
    let (chapter_task, persisted_chapter) =
        assert_completed_chapter(&db, project.id, chapter_content).await;
    eprintln!(
        "real chapter product page task={} chapter={} revision={}",
        chapter_task.task_id,
        persisted_chapter.id,
        chapter_task.result_revision.as_deref().unwrap()
    );

    let optimize_session = a4n_db::optimize::create_optimization_session(
        &db,
        a4n_db::optimize::OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: persisted_chapter.id,
            name: Some("产品页优化验证".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: Some("普通交互会话".to_string()),
        },
    )
    .await
    .unwrap();
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/novel/{}", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    browser
        .post(format!("{webdriver_url}/session/{session_id}/url"))
        .json(&json!({"url": format!("http://{vite_addr}/write/{}?chapter=1", project.id)}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("蓝图产品页验证") && dom.contains(chapter_content) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("writing desk did not remount with the formal chapter before optimize test");
    webdriver_click_button(&browser, &webdriver_url, session_id, "写作助手").await;
    webdriver_click_button(&browser, &webdriver_url, session_id, "优化").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains("输入问题或优化要求") && dom.contains("产品页优化验证")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("optimize product page did not load the formal session before timeout");
    let optimize_message = "产品页优化核对：请判断本章线索是否清楚";
    webdriver_submit_textarea(
        &browser,
        &webdriver_url,
        session_id,
        "textarea[placeholder='输入问题或优化要求']",
        "button[data-optimize-submit='send']",
        optimize_message,
    )
    .await;
    let optimize_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(optimize_message) && dom.contains(optimize_reply) {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("optimize product page generation did not complete before timeout");
    assert!(!optimize_dom.contains("已只读加载正式 Turn 供核对"));
    let (optimize_task, optimize_turn) = assert_completed_optimize_turn(
        &db,
        project.id,
        optimize_session.id,
        optimize_message,
        optimize_reply,
    )
    .await;
    eprintln!(
        "real optimize product page task={} session={} turn={}",
        optimize_task.task_id, optimize_session.id, optimize_turn.id
    );

    let optimize_recovery_message = "产品页优化核对：请再次确认结尾悬念";
    proxy_fault_hits.store(0, std::sync::atomic::Ordering::SeqCst);
    proxy_fault.store(
        PROXY_FAULT_EXPIRED_WITH_REF,
        std::sync::atomic::Ordering::SeqCst,
    );
    webdriver_submit_textarea(
        &browser,
        &webdriver_url,
        session_id,
        "textarea[placeholder='输入问题或优化要求']",
        "button[data-optimize-submit='send']",
        optimize_recovery_message,
    )
    .await;
    let optimize_recovery_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(optimize_recovery_message)
                && dom.contains(optimize_reply)
                && dom.contains("生成任务恢复快照已过期，已只读加载正式 Turn 供核对")
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("optimize expired recovery did not settle before timeout");
    assert!(optimize_recovery_dom.contains("结果校验失败"));
    assert!(proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    proxy_fault.store(PROXY_FAULT_NONE, std::sync::atomic::Ordering::SeqCst);
    let (optimize_recovery_task, optimize_recovery_turn) = assert_completed_optimize_turn(
        &db,
        project.id,
        optimize_session.id,
        optimize_recovery_message,
        optimize_reply,
    )
    .await;
    assert_ne!(optimize_recovery_task.task_id, optimize_task.task_id);
    assert_ne!(optimize_recovery_turn.id, optimize_turn.id);
    eprintln!(
        "real optimize recovery page task={} session={} turn={}",
        optimize_recovery_task.task_id, optimize_session.id, optimize_recovery_turn.id
    );

    webdriver_click_button(&browser, &webdriver_url, session_id, "生成").await;
    proxy_fault_hits.store(0, std::sync::atomic::Ordering::SeqCst);
    proxy_fault.store(
        PROXY_FAULT_EXPIRED_WITH_REF,
        std::sync::atomic::Ordering::SeqCst,
    );
    webdriver_click_button(&browser, &webdriver_url, session_id, "开始生成").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            if source["value"]
                .as_str()
                .unwrap()
                .contains("重新生成章节正文")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("chapter regeneration confirmation did not open before timeout");
    webdriver_click_dialog_button(&browser, &webdriver_url, session_id, "重新生成").await;
    let chapter_recovery_dom = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            if dom.contains(chapter_content)
                && dom.contains("生成任务恢复快照已过期，已只读加载正式章节供核对")
            {
                break dom.to_string();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("chapter expired recovery did not settle before timeout");
    assert!(chapter_recovery_dom.contains("正式章节一致性校验异常"));
    assert!(proxy_fault_hits.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    proxy_fault.store(PROXY_FAULT_NONE, std::sync::atomic::Ordering::SeqCst);
    let (chapter_recovery_task, recovered_chapter) =
        assert_completed_chapter(&db, project.id, chapter_content).await;
    assert_ne!(chapter_recovery_task.task_id, chapter_task.task_id);
    assert_eq!(recovered_chapter.id, persisted_chapter.id);
    eprintln!(
        "real chapter recovery page task={} chapter={}",
        chapter_recovery_task.task_id, recovered_chapter.id
    );
    let _ = browser
        .delete(format!("{webdriver_url}/session/{session_id}"))
        .send()
        .await;
    stop_server(webdriver);
    let browser_error = browser_dom
        .find("data-a4n-error=\"")
        .map(|_| browser_process_value(&browser_dom, "error"));
    let browser_result = browser_process_value(&browser_dom, "result");
    let first_cursor = tokio::time::timeout(PROCESS_TIMEOUT, proxy_requests.recv())
        .await
        .unwrap()
        .unwrap();
    let resumed_cursor = tokio::time::timeout(PROCESS_TIMEOUT, proxy_requests.recv())
        .await
        .unwrap()
        .unwrap();

    let _ = proxy_shutdown.send(());
    proxy_task.await.unwrap().unwrap();
    stop_server(server);
    let provider_frame_count_before_tauri = provider.first_frame_unix_ms().len();
    let tauri_binary = std::env::var("A4N_WDIO_APP_BINARY")
        .expect("real Wry process requires A4N_WDIO_APP_BINARY");
    let tauri_data_dir = tempfile::tempdir_in(&workspace_root).unwrap();
    let tauri_log_dir = tauri_data_dir.path().join("logs");
    let wdio_output = tokio::time::timeout(
        Duration::from_secs(300),
        tokio::process::Command::new("npm")
            .args(["run", "test:tauri-generation"])
            .current_dir(workspace_root.join("frontend"))
            .env("A4N_WDIO_APP_BINARY", tauri_binary)
            .env("A4N_DATABASE_URL", &database_url)
            .env("A4N_PORTABLE_HOME", temp.path())
            .env("A4N_DATA_DIR", tauri_data_dir.path())
            .env("A4N_CACHE_DIR", tauri_data_dir.path().join("cache"))
            .env("A4N_TEMP_DIR", tauri_data_dir.path().join("tmp"))
            .env("A4N_LOG_DIR", &tauri_log_dir)
            .env("A4N_JWT_SECRET", "generation-process-jwt-secret")
            .env("A4N_RUNTIME_PROFILE", "minimal")
            .env("A4N_WDIO_USERNAME", &username)
            .env("A4N_WDIO_PASSWORD", "generation-process-password")
            .env("A4N_WDIO_PROJECT_ID", project.id.to_string())
            .env("A4N_WDIO_SUFFIX", &suffix)
            .env_remove("A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON")
            .output(),
    )
    .await
    .expect("real Wry process exceeded 300 seconds")
    .expect("real Wry WebdriverIO process should start");
    stop_server(vite);
    let wdio_stdout = String::from_utf8_lossy(&wdio_output.stdout);
    let wdio_stderr = String::from_utf8_lossy(&wdio_output.stderr);
    let tauri_log = std::fs::read_dir(&tauri_log_dir)
        .expect("real Wry log directory must exist")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| std::fs::read_to_string(entry.path()).unwrap_or_default())
        .collect::<String>();
    let tauri_errors = tauri_log
        .lines()
        .filter(|line| line.contains(" WARN ") || line.contains(" ERROR "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        wdio_output.status.success(),
        "real Wry WebdriverIO failed\nstdout:\n{wdio_stdout}\nstderr:\n{wdio_stderr}\nTauri WARN/ERROR:\n{tauri_errors}"
    );
    let result_payload = wdio_stdout
        .lines()
        .find_map(|line| {
            line.split_once("A4N_TAURI_WRY_RESULT=")
                .map(|(_, value)| value)
        })
        .expect("real Wry process did not report A4N_TAURI_WRY_RESULT");
    let result_end = result_payload
        .rfind('}')
        .expect("real Wry result must contain a JSON object");
    let tauri_result: serde_json::Value = serde_json::from_str(&result_payload[..=result_end])
        .expect("real Wry result must be valid JSON");
    assert_eq!(tauri_result["remountOccurrences"], 1);
    assert_eq!(tauri_result["remountCompleted"], true);
    assert_eq!(tauri_result["destroyStillSending"], true);
    let provider_after_tauri = provider.first_frame_unix_ms();
    let tauri_provider_frames = &provider_after_tauri[provider_frame_count_before_tauri..];
    assert!(
        tauri_provider_frames.len() >= 2,
        "real Wry scenarios must each consume a provider first frame: {tauri_provider_frames:?}"
    );
    let mut tauri_ui_latency_ms = ["remountFirstTextAt", "destroyFirstTextAt"]
        .into_iter()
        .zip(tauri_provider_frames)
        .map(|(field, provider)| {
            tauri_result[field]
                .as_u64()
                .expect("real Wry first-text timestamp must be an integer")
                .checked_sub(*provider)
                .expect("real Wry first text cannot precede the provider frame")
        })
        .collect::<Vec<_>>();
    tauri_ui_latency_ms.sort_unstable();
    let middle = tauri_ui_latency_ms.len() / 2;
    let tauri_ui_median_ms = if tauri_ui_latency_ms.len() % 2 == 0 {
        tauri_ui_latency_ms[middle - 1]
            + (tauri_ui_latency_ms[middle] - tauri_ui_latency_ms[middle - 1]) / 2
    } else {
        tauri_ui_latency_ms[middle]
    };
    assert!(
        tauri_ui_median_ms <= TAURI_UI_MEDIAN_BUDGET_MS,
        "provider-to-Tauri-UI median {tauri_ui_median_ms}ms exceeds {TAURI_UI_MEDIAN_BUDGET_MS}ms; samples={tauri_ui_latency_ms:?}"
    );
    let (tauri_task, tauri_assistant_id) = assert_completed_inspiration_exchange(
        &db,
        project.id,
        &format!("tauri-wry-remount-{suffix}"),
        &tauri_remount_raw_json,
    )
    .await;
    let destroyed_log = tauri_log
        .lines()
        .find(|line| line.contains("generation Tauri window destroyed"))
        .expect("production WindowEvent::Destroyed log was not recorded");
    assert!(destroyed_log.contains("window_label=\"main\""));
    assert!(destroyed_log.contains("subscriptions_cancelled=1"));
    eprintln!(
        "real Tauri/Wry task={} assistant={} first-frame={tauri_ui_latency_ms:?} median={tauri_ui_median_ms}ms destroyed_subscriptions=1",
        tauri_task.task_id, tauri_assistant_id
    );
    let provider_first_frames = provider.first_frame_unix_ms();
    provider.stop().await;
    #[cfg(feature = "embedded-media-providers")]
    image_provider.stop().await;

    assert!(
        browser_error.is_none(),
        "browser harness failed: {browser_error:?}"
    );
    assert_eq!(first_cursor, None);
    assert!(
        resumed_cursor
            .as_deref()
            .is_some_and(|cursor| cursor.starts_with("1:")),
        "browser did not resume with Last-Event-ID: {resumed_cursor:?}"
    );
    assert_eq!(browser_result["status"], "completed");
    assert_eq!(browser_result["terminal"], true);
    assert_eq!(browser_result["client_state"], "detached");
    assert_eq!(browser_result["recovery_state"], "idle");
    assert_eq!(browser_result["text_bytes"], expected_raw_json.len());
    assert_eq!(browser_result["usage_total"], 5);
    assert_eq!(
        browser_result["checkpoint_covers"],
        browser_result["sequence"]
    );
    assert_eq!(browser_result["checkpoint_status"], "completed");
    assert_eq!(browser_result["summary_status"], "completed");
    assert_eq!(
        browser_result["summary_result_revision"],
        expected_raw_sha256
    );
    assert_eq!(
        browser_result["summary_terminal_sequence"],
        browser_result["sequence"]
    );
    let capacity = &browser_result["capacity"];
    assert_eq!(capacity["status"], "completed");
    assert_eq!(capacity["terminal"], true);
    assert_eq!(capacity["text_bytes"], CAPACITY_TOTAL_BYTES);
    assert_eq!(capacity["dom_text_bytes"], CAPACITY_TOTAL_BYTES);
    assert_eq!(capacity["text_sha256"], expected_capacity_sha256);
    assert!(
        capacity["published_events"]
            .as_u64()
            .is_some_and(|events| events <= CAPACITY_EVENT_BUDGET),
        "10,000 upstream chunks exceeded the event budget: {capacity}"
    );
    assert!(
        capacity["react_commits"]
            .as_u64()
            .is_some_and(|commits| commits <= CAPACITY_REACT_COMMIT_BUDGET),
        "10,000 upstream chunks exceeded the React commit budget: {capacity}"
    );
    eprintln!(
        "generation capacity upstream_chunks={CAPACITY_UPSTREAM_CHUNKS} published_events={} react_commits={} bytes={CAPACITY_TOTAL_BYTES} sha256={expected_capacity_sha256}",
        capacity["published_events"], capacity["react_commits"]
    );
    let ui_first_frames = browser_result["ui_first_frame_unix_ms"]
        .as_array()
        .expect("browser must report UI first-frame samples");
    assert_eq!(ui_first_frames.len(), 5);
    assert!(provider_first_frames.len() >= ui_first_frames.len());
    let mut ui_latency_ms = provider_first_frames
        .iter()
        .zip(ui_first_frames)
        .map(|(provider, ui)| {
            ui.as_u64()
                .expect("UI first-frame timestamp must be an integer")
                .checked_sub(*provider)
                .expect("UI first frame cannot precede the provider frame")
        })
        .collect::<Vec<_>>();
    ui_latency_ms.sort_unstable();
    let median_ui_latency_ms = ui_latency_ms[ui_latency_ms.len() / 2];
    assert!(
        median_ui_latency_ms <= WEB_UI_MEDIAN_BUDGET_MS,
        "provider-to-Web-UI median {median_ui_latency_ms}ms exceeds {WEB_UI_MEDIAN_BUDGET_MS}ms; samples={ui_latency_ms:?}"
    );
    eprintln!(
        "provider-to-Web-UI first-frame latency samples={ui_latency_ms:?} median={median_ui_latency_ms}ms"
    );
    #[cfg(feature = "embedded-media-providers")]
    {
        let expected_image_sha256 = format!(
            "{:x}",
            sha2::Sha256::digest(
                base64::engine::general_purpose::STANDARD
                    .decode(PNG_1X1_BASE64)
                    .unwrap()
            )
        );
        let combination = &browser_result["combination"];
        assert_eq!(combination["image_status"], "completed");
        assert_eq!(combination["image_terminal"], true);
        assert_eq!(combination["image_client_state"], "detached");
        assert_eq!(combination["image_artifact_count"], 1);
        assert_eq!(combination["image_artifact_kind"], "generated_image");
        assert_eq!(
            combination["image_artifact_revision"],
            expected_image_sha256
        );
        assert!(
            combination["image_result_revision"]
                .as_str()
                .is_some_and(|revision| revision.len() == 64),
            "image result revision is invalid: {}",
            combination["image_result_revision"]
        );
        assert_eq!(
            combination["image_terminal_result_revision"],
            combination["image_result_revision"]
        );
        assert_eq!(
            combination["image_checkpoint_covers"],
            combination["image_sequence"]
        );
        assert_eq!(
            combination["image_summary_terminal_sequence"],
            combination["image_sequence"]
        );
        assert_eq!(combination["cancel_status"], "cancelled");
        assert_eq!(combination["cancel_terminal"], true);
        assert_eq!(combination["cancel_client_state"], "detached");
        assert_eq!(combination["cancel_summary_status"], "cancelled");
        assert!(
            combination["cancel_sequence"]
                .as_u64()
                .is_some_and(|sequence| sequence >= 3)
        );
        assert_eq!(
            combination["cancel_checkpoint_covers"],
            combination["cancel_sequence"]
        );
        assert_eq!(
            combination["cancel_summary_terminal_sequence"],
            combination["cancel_sequence"]
        );
        let mut cancel_latency_ms = combination["cancel_latency_ms"]
            .as_array()
            .expect("browser must report cancellation latency samples")
            .iter()
            .map(|sample| {
                sample
                    .as_u64()
                    .expect("cancellation latency must be an integer")
            })
            .collect::<Vec<_>>();
        assert_eq!(cancel_latency_ms.len(), 20);
        cancel_latency_ms.sort_unstable();
        let cancel_p95_ms = cancel_latency_ms[18];
        assert!(
            cancel_p95_ms <= CANCEL_P95_BUDGET_MS,
            "cancel p95 {cancel_p95_ms}ms exceeds {CANCEL_P95_BUDGET_MS}ms; samples={cancel_latency_ms:?}"
        );
        eprintln!("Web cancel-to-terminal samples={cancel_latency_ms:?} p95={cancel_p95_ms}ms");
    }
}

#[cfg(any(
    feature = "embedded-capabilities",
    feature = "embedded-media-providers"
))]
#[tokio::test]
async fn server_process_streams_image_generation_success_terminal() {
    let provider = FakeOpenAiImageProvider::start().await;
    let expected_bytes = base64::engine::general_purpose::STANDARD
        .decode(PNG_1X1_BASE64)
        .unwrap();
    let expected_sha256 = format!("{:x}", sha2::Sha256::digest(&expected_bytes));
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("image_generation_success_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("image generation success {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let image_config = a4n_db::config::create_image_config_record(
        &db,
        a4n_db::config::ImageConfigCreateRecordInput {
            name: format!("image-process-success-{suffix}"),
            backend: "openai-compatible-image".to_string(),
            base_url: provider.base_url(),
            api_key_encrypted: Some("process-test-key".to_string()),
            model: "process-test-image-model".to_string(),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
            provider_options_json: None,
        },
    )
    .await
    .unwrap();
    let image_config = a4n_db::config::set_default_image_config_record(&db, image_config)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/image"))
        .bearer_auth(&token)
        .json(&json!({
            "meta": {
                "start_request_id": Uuid::new_v4().to_string(),
                "retry_of_task_id": null
            },
            "project_id": project.id,
            "operation": {
                "kind": "scene",
                "prompt": "雨夜中的仓库入口，电影感构图",
                "config_id": image_config.id,
                "chapter_id": null
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}/events?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    let events = read_generation_sse_until_terminal(&mut events_response).await;
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let artifacts = events
        .iter()
        .filter(|event| event["event"]["kind"] == "artifact")
        .collect::<Vec<_>>();
    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0]["event"]["data"]["artifact"];
    assert_eq!(artifact["artifact_kind"], "generated_image");
    assert_eq!(artifact["content_type"], "image/png");
    assert_eq!(artifact["sha256"], expected_sha256);
    assert_eq!(artifact["revision"], expected_sha256);
    assert_eq!(artifact["byte_size"], expected_bytes.len());
    assert!(artifact.get("file_path").is_none());
    assert!(artifact.get("url").is_none());

    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let completed = terminal_events[0];
    assert_eq!(completed, events.last().unwrap());
    assert_eq!(completed["event"]["kind"], "completed");
    assert_eq!(
        completed["event"]["data"]["result_ref"]["kind"],
        "generated_image"
    );
    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "completed 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "completed");
    assert_eq!(
        summary["result_ref"],
        completed["event"]["data"]["result_ref"]
    );
    assert_eq!(&summary["terminal_event"], completed);

    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(
        artifact["artifact_id"],
        format!("generated-image:{}", images[0].id)
    );
    assert_eq!(
        std::fs::read(data_dir.join(&images[0].file_path)).unwrap(),
        expected_bytes
    );

    stop_server(server);
    provider.stop().await;
}

#[tokio::test]
async fn server_process_streams_inspiration_generation_failure_terminal() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("inspiration_generation_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("inspiration generation {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("inspiration-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let start_input = json!({
        "meta": {
            "start_request_id": Uuid::new_v4().to_string(),
            "retry_of_task_id": null
        },
        "project_id": project.id,
        "message": "请帮我梳理主角动机",
        "variables": {}
    });
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/inspiration"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}/events?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    let events = read_generation_sse_until_terminal(&mut events_response).await;
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let failed = terminal_events[0];
    assert_eq!(failed, events.last().unwrap());
    assert_eq!(failed["event"]["kind"], "failed");
    assert_eq!(
        failed["event"]["data"]["error"]["code"],
        "provider_job_failed"
    );
    assert_eq!(
        failed["event"]["data"]["error"]["stage_code"],
        "provider_connect"
    );
    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "terminal 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "failed");
    assert_eq!(summary["result_ref"], serde_json::Value::Null);
    assert_eq!(&summary["terminal_event"], failed);
    assert!(
        a4n_db::inspiration::inspiration_messages(&db, project.id, 80)
            .await
            .unwrap()
            .is_empty(),
        "provider 失败不得写入正式灵感消息"
    );
    stop_server(server);
}

#[tokio::test]
async fn server_process_streams_optimize_chat_generation_failure_terminal() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let username = format!("optimize_chat_generation_owner_{suffix}");
    let owner = a4n_db::auth::create_user(
        &db,
        &username,
        &format!("{username}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("optimize chat generation {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("这是一段用于内容优化进程测试的正文。".to_string()),
        word_count: Set(18),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let session = a4n_db::optimize::create_optimization_session(
        &db,
        a4n_db::optimize::OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("进程失败测试".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: None,
            agent_instruction: None,
            fixes_applied: 0,
            summary: Some("普通交互会话".to_string()),
        },
    )
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigCreateRecordInput {
            name: format!("optimize-chat-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            request_options_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_llm_config_record(&db, llm)
        .await
        .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let frontend_dir = temp.path().join("frontend");
    std::fs::create_dir_all(&frontend_dir).unwrap();
    std::fs::write(frontend_dir.join("index.html"), "test").unwrap();
    let addr = unused_local_addr();
    let mut server = spawn_server(&database_url, &data_dir, &frontend_dir, addr);
    wait_for_listener(&mut server, addr).await;

    let client = reqwest::Client::builder()
        .timeout(PROCESS_TIMEOUT)
        .build()
        .unwrap();
    let token = login_access_token(&client, addr, &username).await;
    let start_input = json!({
        "meta": {
            "start_request_id": Uuid::new_v4().to_string(),
            "retry_of_task_id": null
        },
        "request": {
            "session_id": session.id,
            "message": "请帮我优化这一段内容",
            "permissions": null
        }
    });
    let response = client
        .post(format!("http://{addr}/api/generation-tasks/optimize-chat"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    let handle = response.json::<serde_json::Value>().await.unwrap();
    let task_id = handle["task_id"].as_str().unwrap().to_string();
    assert_eq!(handle["attempt"], 1);
    assert_eq!(handle["status"], "reserved");

    let mut events_response = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}/events?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(events_response.status(), reqwest::StatusCode::OK);
    let events = read_generation_sse_until_terminal(&mut events_response).await;
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["task_id"], task_id);
        assert_eq!(event["attempt"], 1);
        assert_eq!(event["sequence"], u64::try_from(index + 1).unwrap());
    }
    let terminal_events = events
        .iter()
        .filter(|event| generation_event_is_terminal(event))
        .collect::<Vec<_>>();
    assert_eq!(terminal_events.len(), 1);
    let failed = terminal_events[0];
    assert_eq!(failed, events.last().unwrap());
    assert_eq!(failed["event"]["kind"], "failed");
    assert_eq!(
        failed["event"]["data"]["error"]["code"],
        "provider_job_failed"
    );
    assert_eq!(
        failed["event"]["data"]["error"]["stage_code"],
        "provider_connect"
    );
    assert!(
        tokio::time::timeout(PROCESS_TIMEOUT, events_response.chunk())
            .await
            .unwrap()
            .unwrap()
            .is_none(),
        "terminal 后 SSE 仍有事件"
    );

    let summary = client
        .get(format!(
            "http://{addr}/api/generation-tasks/{task_id}?attempt=1"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(summary["handle"]["status"], "failed");
    assert_eq!(summary["result_ref"], serde_json::Value::Null);
    assert_eq!(&summary["terminal_event"], failed);

    let retry_response = client
        .post(format!("http://{addr}/api/generation-tasks/optimize-chat"))
        .bearer_auth(&token)
        .json(&start_input)
        .send()
        .await
        .unwrap();
    assert_eq!(retry_response.status(), reqwest::StatusCode::OK);
    let retry_handle = retry_response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(retry_handle["task_id"], task_id);
    assert_eq!(retry_handle["attempt"], 1);
    assert_eq!(retry_handle["status"], "failed");

    let turns = a4n_db::optimize::optimization_turns(&db, session.id)
        .await
        .unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].status, "failed");
    assert!(turns[0].final_reply.is_none());
    assert!(turns[0].error.is_some());
    let formal_messages = a4n_db::optimize::optimization_session_messages(&db, session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
        .collect::<Vec<_>>();
    assert!(
        formal_messages.is_empty(),
        "provider 连接失败不得写入正式 user/assistant 消息"
    );
    stop_server(server);
}

async fn read_generation_sse_until_terminal(
    response: &mut reqwest::Response,
) -> Vec<serde_json::Value> {
    let events = read_generation_sse(response, None).await;
    assert!(
        events.last().is_some_and(generation_event_is_terminal),
        "generation SSE 未到达终态"
    );
    events
}

async fn read_generation_sse_events(
    response: &mut reqwest::Response,
    max_events: usize,
) -> Vec<serde_json::Value> {
    assert!(max_events > 0);
    read_generation_sse(response, Some(max_events)).await
}

async fn spawn_abort_once_sse_proxy(
    backend: SocketAddr,
) -> (
    SocketAddr,
    mpsc::UnboundedReceiver<Option<String>>,
    Arc<std::sync::atomic::AtomicUsize>,
    Arc<std::sync::atomic::AtomicUsize>,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), std::io::Error>>,
) {
    use axum::{body::Bytes, extract::OriginalUri, response::Response};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tower_http::cors::CorsLayer;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let aborted = Arc::new(AtomicBool::new(false));
    let fault = Arc::new(AtomicUsize::new(PROXY_FAULT_NONE));
    let fault_hits = Arc::new(AtomicUsize::new(0));
    let event_fault = Arc::clone(&fault);
    let event_fault_hits = Arc::clone(&fault_hits);
    let fallback_fault = Arc::clone(&fault);
    let fallback_fault_hits = Arc::clone(&fault_hits);
    let client = reqwest::Client::new();
    let event_client = client.clone();
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let router = Router::new()
        .route(
            "/api/generation-tasks/{task_id}/events",
            axum::routing::get(move |OriginalUri(uri): OriginalUri, headers: HeaderMap| {
                let aborted = Arc::clone(&aborted);
                let fault = Arc::clone(&event_fault);
                let fault_hits = Arc::clone(&event_fault_hits);
                let client = event_client.clone();
                let request_tx = request_tx.clone();
                async move {
                    let mut request = client.get(format!("http://{backend}{uri}"));
                    for name in [header::AUTHORIZATION, header::ACCEPT] {
                        if let Some(value) = headers.get(&name) {
                            request = request.header(&name, value);
                        }
                    }
                    if let Some(value) = headers.get("last-event-id") {
                        request = request.header("last-event-id", value);
                    }
                    let response = request.send().await.unwrap();
                    let status = response.status();
                    let content_type = response.headers().get(header::CONTENT_TYPE).cloned();
                    let last_event_id = headers
                        .get("last-event-id")
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string);
                    request_tx.send(last_event_id).unwrap();
                    let fault = fault.load(Ordering::SeqCst);
                    if proxy_recovery_fault(fault).is_some() {
                        response.bytes().await.unwrap();
                        fault_hits.fetch_add(1, Ordering::SeqCst);
                        return Response::builder()
                            .status(StatusCode::CONFLICT)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                json!({
                                    "code": "generation_snapshot_required",
                                    "message": "proxy requires snapshot recovery",
                                    "retryable": true,
                                })
                                .to_string(),
                            ))
                            .unwrap();
                    }
                    let body = if fault == PROXY_FAULT_CONSISTENCY {
                        let body = response.text().await.unwrap();
                        let body = corrupt_completed_hash_in_sse(&body);
                        fault_hits.fetch_add(1, Ordering::SeqCst);
                        Body::from(body)
                    } else if !aborted.swap(true, Ordering::SeqCst) {
                        let mut upstream = response.bytes_stream();
                        let mut prefix = Vec::new();
                        while !prefix.windows(2).any(|window| window == b"\n\n") {
                            prefix.extend_from_slice(&upstream.next().await.unwrap().unwrap());
                        }
                        Body::from_stream(
                            stream::once(async move {
                                Ok::<Bytes, std::io::Error>(Bytes::from(prefix))
                            })
                            .chain(stream::once(async {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::ConnectionReset,
                                    "browser process proxy aborted SSE",
                                ))
                            })),
                        )
                    } else {
                        Body::from_stream(response.bytes_stream())
                    };
                    let mut builder = Response::builder().status(status);
                    if let Some(content_type) = content_type {
                        builder = builder.header(header::CONTENT_TYPE, content_type);
                    }
                    builder.body(body).unwrap()
                }
            }),
        )
        .fallback(any(
            move |method: axum::http::Method,
                  OriginalUri(uri): OriginalUri,
                  headers: HeaderMap,
                  body: Bytes| {
                let client = client.clone();
                let fault = Arc::clone(&fallback_fault);
                let fault_hits = Arc::clone(&fallback_fault_hits);
                async move {
                    let fault = fault.load(Ordering::SeqCst);
                    let recovery_fault = proxy_recovery_fault(fault);
                    let corrupt_snapshot =
                        fault == PROXY_FAULT_CONSISTENCY && uri.path().ends_with("/snapshot");
                    let mut request = client.request(method, format!("http://{backend}{uri}"));
                    for name in [header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE] {
                        if let Some(value) = headers.get(&name) {
                            request = request.header(&name, value);
                        }
                    }
                    let response = request.body(body).send().await.unwrap();
                    let mut status = response.status();
                    let content_type = response.headers().get(header::CONTENT_TYPE).cloned();
                    let mut body = response.bytes().await.unwrap().to_vec();
                    if recovery_fault.is_some() && uri.path().ends_with("/snapshot") {
                        status = StatusCode::CONFLICT;
                        body = serde_json::to_vec(&json!({
                            "code": "generation_snapshot_required",
                            "message": "proxy snapshot unavailable",
                            "retryable": true,
                        }))
                        .unwrap();
                        fault_hits.fetch_add(1, Ordering::SeqCst);
                    } else if let Some((snapshot_state, keep_result_ref)) = recovery_fault
                        && uri.query().is_some_and(|query| query.contains("attempt="))
                    {
                        let mut summary = serde_json::from_slice::<serde_json::Value>(&body)
                            .expect("generation summary proxy response must be JSON");
                        summary["snapshot_state"] = json!(snapshot_state);
                        if !keep_result_ref {
                            summary["result_ref"] = serde_json::Value::Null;
                        }
                        body = serde_json::to_vec(&summary).unwrap();
                        fault_hits.fetch_add(1, Ordering::SeqCst);
                    } else if corrupt_snapshot {
                        let mut snapshot = serde_json::from_slice::<serde_json::Value>(&body)
                            .expect("generation snapshot proxy response must be JSON");
                        assert!(corrupt_completed_hash(&mut snapshot));
                        fault_hits.fetch_add(1, Ordering::SeqCst);
                        body = serde_json::to_vec(&snapshot).unwrap();
                    }
                    let mut builder = Response::builder().status(status);
                    if let Some(content_type) = content_type {
                        builder = builder.header(header::CONTENT_TYPE, content_type);
                    }
                    builder.body(Body::from(body)).unwrap()
                }
            },
        ))
        .layer(CorsLayer::permissive());
    let (shutdown, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
    });
    (addr, request_rx, fault, fault_hits, shutdown, task)
}

fn proxy_recovery_fault(mode: usize) -> Option<(&'static str, bool)> {
    match mode {
        PROXY_FAULT_EXPIRED_WITH_REF => Some(("expired", true)),
        PROXY_FAULT_EXPIRED_WITHOUT_REF => Some(("expired", false)),
        PROXY_FAULT_CORRUPT_WITH_REF => Some(("corrupt", true)),
        PROXY_FAULT_CORRUPT_WITHOUT_REF => Some(("corrupt", false)),
        _ => None,
    }
}

fn corrupt_completed_hash(value: &mut serde_json::Value) -> bool {
    let terminal = if value["event"]["kind"] == "completed" {
        value
    } else if value["terminal_event"]["event"]["kind"] == "completed" {
        &mut value["terminal_event"]
    } else {
        return false;
    };
    let Some(hash) = terminal["event"]["data"]["final_text_hashes"]
        .as_array_mut()
        .and_then(|hashes| hashes.first_mut())
    else {
        return false;
    };
    hash["sha256"] = serde_json::Value::String("0".repeat(64));
    true
}

fn corrupt_completed_hash_in_sse(body: &str) -> String {
    let mut corrupted = false;
    let mut output = String::with_capacity(body.len());
    for line in body.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            let mut event = serde_json::from_str::<serde_json::Value>(data).unwrap();
            if corrupt_completed_hash(&mut event) {
                corrupted = true;
                output.push_str("data: ");
                output.push_str(&event.to_string());
                output.push('\n');
                continue;
            }
        }
        output.push_str(line);
        output.push('\n');
    }
    assert!(corrupted, "proxied SSE did not contain a completed event");
    output
}

fn chromedriver_binary() -> String {
    if let Ok(binary) = std::env::var("A4N_CHROMEDRIVER_BIN")
        && !binary.trim().is_empty()
    {
        return binary;
    }
    for binary in ["chromedriver", "chromium-driver"] {
        if Command::new(binary)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            return binary.to_string();
        }
    }
    panic!("real Chromium process test requires A4N_CHROMEDRIVER_BIN or chromedriver on PATH")
}

fn browser_process_value(dom: &str, name: &str) -> serde_json::Value {
    let marker = format!("data-a4n-{name}=\"");
    let encoded = dom
        .split_once(&marker)
        .and_then(|(_, tail)| tail.split_once('\"').map(|(value, _)| value))
        .unwrap_or_else(|| panic!("browser DOM is missing {marker}: {dom}"));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn webdriver_send_inspiration_message(
    browser: &reqwest::Client,
    webdriver_url: &str,
    session_id: &str,
    message: &str,
) {
    const ELEMENT_KEY: &str = "element-6066-11e4-a52e-4f735466cecf";
    let textarea = browser
        .post(format!("{webdriver_url}/session/{session_id}/element"))
        .json(&json!({
            "using": "css selector",
            "value": ".chat-composer textarea"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let textarea_id = textarea["value"][ELEMENT_KEY].as_str().unwrap();
    browser
        .post(format!(
            "{webdriver_url}/session/{session_id}/element/{textarea_id}/value"
        ))
        .json(&json!({"text": message}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let send_button = browser
        .post(format!("{webdriver_url}/session/{session_id}/element"))
        .json(&json!({"using": "xpath", "value": "//button[normalize-space(.)='发送']"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let send_button_id = send_button["value"][ELEMENT_KEY].as_str().unwrap();
    browser
        .post(format!(
            "{webdriver_url}/session/{session_id}/element/{send_button_id}/click"
        ))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

async fn webdriver_submit_textarea(
    browser: &reqwest::Client,
    webdriver_url: &str,
    session_id: &str,
    textarea_selector: &str,
    button_selector: &str,
    message: &str,
) {
    const ELEMENT_KEY: &str = "element-6066-11e4-a52e-4f735466cecf";
    let started = Instant::now();
    let textarea_id = loop {
        let textareas = browser
            .post(format!("{webdriver_url}/session/{session_id}/elements"))
            .json(&json!({"using": "css selector", "value": textarea_selector}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let mut found = None;
        for textarea in textareas["value"].as_array().unwrap() {
            let candidate = textarea[ELEMENT_KEY].as_str().unwrap();
            let displayed = browser
                .get(format!(
                    "{webdriver_url}/session/{session_id}/element/{candidate}/displayed"
                ))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let enabled = browser
                .get(format!(
                    "{webdriver_url}/session/{session_id}/element/{candidate}/enabled"
                ))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            if displayed["value"].as_bool() == Some(true)
                && enabled["value"].as_bool() == Some(true)
            {
                found = Some(candidate.to_string());
                break;
            }
        }
        if let Some(textarea_id) = found {
            break textarea_id;
        }
        if started.elapsed() >= Duration::from_secs(10) {
            let source = browser
                .get(format!("{webdriver_url}/session/{session_id}/source"))
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            let dom = source["value"].as_str().unwrap();
            let context = dom
                .find("输入问题或优化要求")
                .map(|index| dom[index..].chars().take(2_000).collect::<String>())
                .unwrap_or_default();
            panic!(
                "no interactable textarea matched {textarea_selector}: waiting_chapter={} loading={} current_chapter={} session={} running={} context={context}",
                dom.contains("写作桌当前没有可优化章节"),
                dom.contains("正在加载章节和内容优化会话"),
                dom.contains("章节产品页验证"),
                dom.contains("产品页优化验证"),
                dom.contains("运行中"),
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let response = browser
        .post(format!(
            "{webdriver_url}/session/{session_id}/element/{textarea_id}/value"
        ))
        .json(&json!({"text": message}))
        .send()
        .await
        .unwrap();
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        panic!("webdriver value failed for {textarea_selector}: status={status} body={body}");
    }
    let button = browser
        .post(format!("{webdriver_url}/session/{session_id}/element"))
        .json(&json!({"using": "css selector", "value": button_selector}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let button_id = button["value"][ELEMENT_KEY].as_str().unwrap();
    let response = browser
        .post(format!(
            "{webdriver_url}/session/{session_id}/element/{button_id}/click"
        ))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        panic!("webdriver click failed for {button_selector}: status={status} body={body}");
    }
}

async fn webdriver_click_button(
    browser: &reqwest::Client,
    webdriver_url: &str,
    session_id: &str,
    label: &str,
) {
    webdriver_click_button_by_xpath(
        browser,
        webdriver_url,
        session_id,
        &format!("//button[normalize-space(.)='{label}']"),
    )
    .await;
}

async fn webdriver_click_dialog_button(
    browser: &reqwest::Client,
    webdriver_url: &str,
    session_id: &str,
    label: &str,
) {
    webdriver_click_button_by_xpath(
        browser,
        webdriver_url,
        session_id,
        &format!("//*[@role='dialog']//button[normalize-space(.)='{label}']"),
    )
    .await;
}

async fn webdriver_click_button_by_xpath(
    browser: &reqwest::Client,
    webdriver_url: &str,
    session_id: &str,
    xpath: &str,
) {
    const ELEMENT_KEY: &str = "element-6066-11e4-a52e-4f735466cecf";
    let button_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let buttons = browser
                .post(format!("{webdriver_url}/session/{session_id}/elements"))
                .json(&json!({
                    "using": "xpath",
                    "value": xpath,
                }))
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
            for button in buttons["value"].as_array().unwrap() {
                let candidate = button[ELEMENT_KEY].as_str().unwrap();
                let displayed = browser
                    .get(format!(
                        "{webdriver_url}/session/{session_id}/element/{candidate}/displayed"
                    ))
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json::<serde_json::Value>()
                    .await
                    .unwrap();
                let enabled = browser
                    .get(format!(
                        "{webdriver_url}/session/{session_id}/element/{candidate}/enabled"
                    ))
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json::<serde_json::Value>()
                    .await
                    .unwrap();
                if displayed["value"].as_bool() == Some(true)
                    && enabled["value"].as_bool() == Some(true)
                {
                    return candidate.to_string();
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no displayed button matched {xpath}"));
    let started = Instant::now();
    loop {
        let response = browser
            .post(format!(
                "{webdriver_url}/session/{session_id}/element/{button_id}/click"
            ))
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        if response.status().is_success() {
            break;
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if body.contains("element click intercepted") && started.elapsed() < Duration::from_secs(10)
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        panic!("webdriver click failed for {xpath}: status={status} body={body}");
    }
}

async fn assert_completed_inspiration_exchange(
    db: &DatabaseConnection,
    project_id: i64,
    user_input: &str,
    assistant_output: &str,
) -> (generation_task_checkpoint::Model, i64) {
    let messages = a4n_db::inspiration::inspiration_messages(db, project_id, 80)
        .await
        .unwrap();
    let user_index = messages
        .iter()
        .position(|message| message.role == "user" && message.content == user_input)
        .expect("product page must persist its user message");
    let assistant = messages
        .get(user_index + 1)
        .expect("product page user message must be followed by an assistant message");
    assert_eq!(assistant.role, "assistant");
    assert_eq!(assistant.content, assistant_output);
    let task = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project_id))
        .filter(generation_task_checkpoint::Column::ResultKind.eq("inspiration_message"))
        .filter(generation_task_checkpoint::Column::ResultId.eq(assistant.id.to_string()))
        .one(db)
        .await
        .unwrap()
        .expect("product page generation task must retain its formal result reference");
    assert_eq!(task.status, "completed");
    assert_eq!(
        task.target_kind.as_deref(),
        Some("inspiration_conversation")
    );
    let expected_target_id = project_id.to_string();
    assert_eq!(task.target_id.as_deref(), Some(expected_target_id.as_str()));
    let expected_revision = format!("{:x}", sha2::Sha256::digest(assistant_output.as_bytes()));
    assert_eq!(
        task.result_revision.as_deref(),
        Some(expected_revision.as_str())
    );
    (task, assistant.id)
}

async fn assert_completed_blueprint(
    db: &DatabaseConnection,
    project_id: i64,
    expected_synopsis: &str,
) -> (generation_task_checkpoint::Model, novel_blueprint::Model) {
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .unwrap()
        .expect("product page must persist its formal blueprint");
    assert_eq!(blueprint.full_synopsis.as_deref(), Some(expected_synopsis));
    let task = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project_id))
        .filter(generation_task_checkpoint::Column::ResultKind.eq("novel_blueprint"))
        .filter(generation_task_checkpoint::Column::ResultId.eq(blueprint.id.to_string()))
        .filter(
            generation_task_checkpoint::Column::ResultRevision
                .eq(blueprint.current_version.clone()),
        )
        .one(db)
        .await
        .unwrap()
        .expect("product page blueprint task must retain its formal result reference");
    assert_eq!(task.status, "completed");
    assert_eq!(task.target_kind.as_deref(), Some("blueprint_generation"));
    let expected_target_id = project_id.to_string();
    assert_eq!(task.target_id.as_deref(), Some(expected_target_id.as_str()));
    (task, blueprint)
}

async fn assert_completed_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    expected_content: &str,
) -> (generation_task_checkpoint::Model, chapter::Model) {
    let chapter = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(1))
        .one(db)
        .await
        .unwrap()
        .expect("chapter product page must persist its formal chapter");
    assert_eq!(chapter.content, expected_content);
    assert_eq!(chapter.status, chapter::STATUS_DRAFT);
    let expected_revision = format!("{:x}", sha2::Sha256::digest(expected_content.as_bytes()));
    let task = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project_id))
        .filter(generation_task_checkpoint::Column::ResultKind.eq("chapter"))
        .filter(generation_task_checkpoint::Column::ResultId.eq(chapter.id.to_string()))
        .filter(generation_task_checkpoint::Column::ResultRevision.eq(expected_revision.clone()))
        .order_by_desc(generation_task_checkpoint::Column::CreatedAt)
        .one(db)
        .await
        .unwrap()
        .expect("chapter product page task must retain its formal result reference");
    assert_eq!(task.status, "completed");
    assert_eq!(task.target_kind.as_deref(), Some("chapter"));
    let expected_target_id = format!("{project_id}:1");
    assert_eq!(task.target_id.as_deref(), Some(expected_target_id.as_str()));
    assert_eq!(
        task.result_revision.as_deref(),
        Some(expected_revision.as_str())
    );
    (task, chapter)
}

async fn assert_completed_optimize_turn(
    db: &DatabaseConnection,
    project_id: i64,
    session_id: i64,
    user_text: &str,
    expected_reply: &str,
) -> (
    generation_task_checkpoint::Model,
    a4n_core::models::ai::optimization_turn::Model,
) {
    let turn = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(turn) = a4n_db::optimize::optimization_turns(db, session_id)
                .await
                .unwrap()
                .into_iter()
                .find(|turn| turn.user_text == user_text && turn.status == "completed")
            {
                break turn;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("optimize product page formal turn did not complete before timeout");
    assert_eq!(turn.status, "completed");
    assert_eq!(turn.final_reply.as_deref(), Some(expected_reply));
    let messages = a4n_db::optimize::optimization_session_messages(db, session_id)
        .await
        .unwrap();
    let user_index = messages
        .iter()
        .position(|message| message.role == "user" && message.content == user_text)
        .expect("optimize product page must persist its formal user message");
    let assistant = messages
        .get(user_index + 1)
        .expect("optimize product page user message must be followed by an assistant message");
    assert_eq!(assistant.role, "assistant");
    assert_eq!(assistant.content, expected_reply);
    let expected_revision = format!("{:x}", sha2::Sha256::digest(expected_reply.as_bytes()));
    let task = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::ProjectId.eq(project_id))
        .filter(generation_task_checkpoint::Column::ResultKind.eq("optimization_turn"))
        .filter(generation_task_checkpoint::Column::ResultId.eq(turn.id.to_string()))
        .one(db)
        .await
        .unwrap()
        .expect("optimize product page task must retain its formal result reference");
    assert_eq!(task.status, "completed");
    assert_eq!(task.target_kind.as_deref(), Some("optimization_chat"));
    let expected_target_id = session_id.to_string();
    assert_eq!(task.target_id.as_deref(), Some(expected_target_id.as_str()));
    assert_eq!(
        task.result_revision.as_deref(),
        Some(expected_revision.as_str())
    );
    (task, turn)
}

async fn read_generation_sse(
    response: &mut reqwest::Response,
    max_events: Option<usize>,
) -> Vec<serde_json::Value> {
    tokio::time::timeout(PROCESS_TIMEOUT, async {
        let mut buffer = String::new();
        let mut events = Vec::new();
        loop {
            let chunk = response
                .chunk()
                .await
                .unwrap()
                .expect("generation SSE closed before terminal event");
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(frame_end) = buffer.find("\n\n") {
                let frame = buffer[..frame_end].to_string();
                buffer.drain(..frame_end + 2);
                let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data: ")) else {
                    continue;
                };
                let event = serde_json::from_str::<serde_json::Value>(data).unwrap();
                let terminal = generation_event_is_terminal(&event);
                events.push(event);
                if terminal || max_events.is_some_and(|limit| events.len() >= limit) {
                    return events;
                }
            }
        }
    })
    .await
    .expect("generation SSE read timed out")
}

fn generation_event_is_terminal(event: &serde_json::Value) -> bool {
    matches!(
        event["event"]["kind"].as_str(),
        Some("completed" | "failed" | "cancelled" | "interrupted")
    )
}

fn generation_reservation_input(
    user_id: i64,
    project_id: i64,
    start_request_id: String,
    operation: &str,
) -> ReserveGenerationTaskInput {
    ReserveGenerationTaskInput {
        requested_by_user_id: user_id,
        start_request_id,
        business_key_json: json!({
            "version": 1,
            "project_id": project_id,
            "operation": operation,
        }),
        input_fingerprint_json: json!({"prompt": operation}),
        task_kind: TaskKind::TextGeneration,
        priority: TaskPriority::High,
        delivery_mode: GenerationDeliveryMode::Stream,
        batch_policy: None,
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        task_scope: Some("server_process_generation_test".to_string()),
        workflow_run_id: None,
        generation_run_id: None,
        target: None,
        root_task_id: None,
        parent_task_id: None,
        item_key: None,
        retry_of_task_id: None,
    }
}

async fn login_access_token(client: &reqwest::Client, addr: SocketAddr, username: &str) -> String {
    client
        .post(format!("http://{addr}/api/auth/login"))
        .json(&json!({
            "username": username,
            "password": "generation-process-password",
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap()["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn isolated_test_database() -> (DatabaseConnection, String) {
    let base_database_url = std::env::var("A4N_TEST_DATABASE_URL")
        .expect("cleanup process test requires A4N_TEST_DATABASE_URL");
    let db = a4n_migration::test_support::connect_test_database().await;
    let schema = db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT current_schema() AS schema",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "schema")
        .unwrap();
    let separator = if base_database_url.contains('?') {
        '&'
    } else {
        '?'
    };
    let database_url = format!("{base_database_url}{separator}options[search_path]={schema}");
    (db, database_url)
}

async fn install_upload_failure_triggers(db: &DatabaseConnection) {
    execute_sql(
        db,
        r#"CREATE FUNCTION cleanup_recovery_fail_generated_image_insert()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'test generated image db failure';
END;
$$"#,
    )
    .await;
    execute_sql(
        db,
        r#"CREATE TRIGGER cleanup_recovery_fail_generated_image_insert
BEFORE INSERT ON generated_image
FOR EACH ROW EXECUTE FUNCTION cleanup_recovery_fail_generated_image_insert()"#,
    )
    .await;
    execute_sql(
        db,
        &format!(
            r#"CREATE FUNCTION cleanup_recovery_fail_file_cleanup_insert()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cleanup_kind = '{}' THEN
        RAISE EXCEPTION 'test cleanup intent db failure';
    END IF;
    RETURN NEW;
END;
$$"#,
            a4n_db::cleanup_outbox::FILE_ARTIFACT_CLEANUP_KIND
        ),
    )
    .await;
    execute_sql(
        db,
        r#"CREATE TRIGGER cleanup_recovery_fail_file_cleanup_insert
BEFORE INSERT ON project_cleanup_outbox
FOR EACH ROW EXECUTE FUNCTION cleanup_recovery_fail_file_cleanup_insert()"#,
    )
    .await;
}

async fn remove_upload_failure_triggers(db: &DatabaseConnection) {
    for sql in [
        "DROP TRIGGER cleanup_recovery_fail_generated_image_insert ON generated_image",
        "DROP FUNCTION cleanup_recovery_fail_generated_image_insert()",
        "DROP TRIGGER cleanup_recovery_fail_file_cleanup_insert ON project_cleanup_outbox",
        "DROP FUNCTION cleanup_recovery_fail_file_cleanup_insert()",
    ] {
        execute_sql(db, sql).await;
    }
}

async fn install_upload_blocking_trigger(db: &DatabaseConnection, lock_key: i64) {
    execute_sql(
        db,
        &format!(
            r#"CREATE FUNCTION cleanup_recovery_block_generated_image_insert()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock({lock_key});
    RETURN NEW;
END;
$$"#
        ),
    )
    .await;
    execute_sql(
        db,
        r#"CREATE TRIGGER cleanup_recovery_block_generated_image_insert
BEFORE INSERT ON generated_image
FOR EACH ROW EXECUTE FUNCTION cleanup_recovery_block_generated_image_insert()"#,
    )
    .await;
}

async fn remove_upload_blocking_trigger(db: &DatabaseConnection) {
    for sql in [
        "DROP TRIGGER cleanup_recovery_block_generated_image_insert ON generated_image",
        "DROP FUNCTION cleanup_recovery_block_generated_image_insert()",
    ] {
        execute_sql(db, sql).await;
    }
}

async fn execute_sql(db: &DatabaseConnection, sql: &str) {
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        sql.to_string(),
    ))
    .await
    .unwrap();
}

fn spawn_server(
    database_url: &str,
    data_dir: &Path,
    frontend_dir: &Path,
    addr: SocketAddr,
) -> Child {
    server_command(database_url, data_dir, frontend_dir, addr)
        .spawn()
        .expect("start a4n-server cleanup process test")
}

fn server_command(
    database_url: &str,
    data_dir: &Path,
    frontend_dir: &Path,
    addr: SocketAddr,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_a4n-server"));
    command
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("A4N_DATABASE_URL", database_url)
        .env("A4N_JWT_SECRET", "cleanup-process-test-secret")
        .env("A4N_RUNTIME_PROFILE", "minimal")
        .env("A4N_DATA_DIR", data_dir)
        .env("A4N_CACHE_DIR", data_dir.join("cache"))
        .env("A4N_TEMP_DIR", data_dir.join("tmp"))
        .env("A4N_LOG_DIR", data_dir.join("logs"))
        .env("A4N_FRONTEND_DIST", frontend_dir)
        .env("A4N_WEB_ADDR", addr.to_string())
        .env("A4N_TEMP_MEDIA_UPLOAD_TOKEN", "temp-media-process-token")
        .env(
            "A4N_TEMP_MEDIA_PUBLIC_BASE_URL",
            "https://media.example.test/api/temp-media",
        )
        .env("A4N_TEMP_MEDIA_TTL_MINUTES", "360")
        .env("A4N_BOOTSTRAP_ADMIN_USERNAME", "cleanup_process_admin")
        .env(
            "A4N_BOOTSTRAP_ADMIN_EMAIL",
            "cleanup-process-admin@example.invalid",
        )
        .env("A4N_BOOTSTRAP_ADMIN_PASSWORD", "cleanup-process-password")
        .env("RUST_LOG", "error")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

fn unused_local_addr() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

async fn wait_for_listener(child: &mut Child, addr: SocketAddr) {
    let started = Instant::now();
    loop {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        if let Some(status) = child.try_wait().unwrap() {
            let stderr = child_stderr(child);
            panic!("a4n-server exited before startup completed: {status}\n{stderr}");
        }
        if started.elapsed() >= PROCESS_TIMEOUT {
            stop_server_for_timeout(child);
            panic!("a4n-server did not listen on {addr} before timeout");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_for_file_with_prefix(child: &mut Child, dir: &Path, prefix: &str) -> PathBuf {
    let started = Instant::now();
    loop {
        if let Ok(entries) = std::fs::read_dir(dir) {
            if let Some(path) = entries.flatten().map(|entry| entry.path()).find(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(prefix))
            }) {
                return path;
            }
        }
        if let Some(status) = child.try_wait().unwrap() {
            let stderr = child_stderr(child);
            panic!("a4n-server exited before creating upload file: {status}\n{stderr}");
        }
        if started.elapsed() >= PROCESS_TIMEOUT {
            stop_server_for_timeout(child);
            panic!("a4n-server did not create upload file before timeout");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_for_failure(mut child: Child) -> String {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let stderr = child_stderr(&mut child);
            assert!(
                !status.success(),
                "a4n-server unexpectedly exited successfully"
            );
            return stderr;
        }
        if started.elapsed() >= PROCESS_TIMEOUT {
            stop_server_for_timeout(&mut child);
            panic!("second a4n-server did not reject the occupied data directory");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn child_stderr(child: &mut Child) -> String {
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr).unwrap();
    }
    stderr
}

fn stop_server(mut child: Child) {
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    child.wait().unwrap();
}

fn stop_server_for_timeout(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}
