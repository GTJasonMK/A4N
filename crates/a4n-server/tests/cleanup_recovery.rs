use std::{
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime},
};

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
use std::{
    sync::{Arc, Mutex},
    time::UNIX_EPOCH,
};

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
#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
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
#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
use futures::{StreamExt as _, stream};
#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
use sea_orm::QueryOrder;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::json;
use sha2::Digest as _;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
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
async fn server_startup_restores_rejected_project_delete_gate() {
    let (db, database_url) = isolated_test_database().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let owner = a4n_db::auth::create_user(
        &db,
        &format!("delete_recovery_{suffix}"),
        &format!("delete_recovery_{suffix}@example.invalid"),
        "generation-process-password",
        ROLE_USER,
    )
    .await
    .unwrap();
    let root = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: format!("delete recovery root {suffix}"),
            description: String::new(),
            project_type: a4n_db::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("用于创建派生项目的正文。".to_string()),
        word_count: Set(12),
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
    let derived = a4n_db::project::create_derived_project_record(
        &db,
        owner.id,
        a4n_db::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: a4n_db::project::PROJECT_KIND_VIDEO.to_string(),
            title: format!("delete recovery derived {suffix}"),
            description: String::new(),
            chapter_scope: a4n_db::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE novel_project SET lifecycle_gate = 'deleting', updated_at = NOW() WHERE id = $1",
        [root.id.into()],
    ))
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

    let gate = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT lifecycle_gate FROM novel_project WHERE id = $1",
            [root.id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "lifecycle_gate")
        .unwrap();
    assert_eq!(gate, "active");
    for project_id in [root.id, derived.id] {
        assert!(
            a4n_core::models::project::novel_project::Entity::find_by_id(project_id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
    }
    stop_server(server);
}

#[tokio::test]
async fn server_process_recovers_only_registered_or_expired_cleanup() {
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
    let unregistered_media_paths = [
        "images/orphan.png",
        "videos/orphan.mp4",
        "portraits/orphan.png",
        "audio/orphan.mp3",
        "media/orphan.bin",
        "assets/orphan.bin",
    ]
    .map(|path| format!("projects/{project_id}/{path}"));
    let unregistered_project_path = format!("projects/{}/chapters/orphan.md", project_id + 1);
    let restart_path = format!("projects/{project_id}/custom/restart.bin");
    let failed_path = format!("projects/{project_id}/custom/not-a-file.bin");
    let expired_cache_output =
        format!("projects/{project_id}/videos/chapter_1/assembled_expired.mp4");
    let expired_cache_manifest = format!("{expired_cache_output}.cache.json");
    let expired_upload = data_dir.join("tmp/video_uploads/video_upload_expired.mp4");
    for path in unregistered_media_paths
        .iter()
        .chain([&unregistered_project_path, &restart_path])
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
    for path in &unregistered_media_paths {
        assert!(data_dir.join(path).exists(), "启动不应推断删除媒体: {path}");
    }
    assert!(data_dir.join(&unregistered_project_path).exists());
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
async fn server_process_reports_image_record_and_cleanup_intent_failures() {
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
        .post(format!("http://{first_addr}/api/commands/login_user"))
        .json(&serde_json::json!({
            "input": {
                "username": "cleanup_process_admin",
                "password": "cleanup-process-password",
            },
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
            "input": {
                "project_id": project.id,
                "file_name": "fault.png",
                "mime_type": "image/png",
                "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
                "image_type": "imported",
                "prompt": "fault injection",
                "chapter_id": null,
                "panel_id": null,
                "metadata_json": null
            }
        }))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let message = response.json::<serde_json::Value>().await.unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    stop_server(first);

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert!(message.contains("导入图片记录创建失败"), "{message}");
    assert!(message.contains("后续清理也失败"), "{message}");
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
}

#[tokio::test]
async fn server_process_kill_before_image_record_commit_leaves_no_database_state() {
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
        .post(format!("http://{first_addr}/api/commands/login_user"))
        .json(&serde_json::json!({
            "input": {
                "username": "cleanup_process_admin",
                "password": "cleanup-process-password",
            },
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

    let project_id = project.id;
    let request = tokio::spawn({
        let client = client.clone();
        let access_token = access_token.clone();
        async move {
            client
                .post(format!("http://{first_addr}/api/commands/upload_image"))
                .bearer_auth(access_token)
                .json(&serde_json::json!({
                    "input": {
                        "project_id": project_id,
                        "file_name": "kill-window.png",
                        "mime_type": "image/png",
                        "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
                        "image_type": "imported",
                        "prompt": "kill window",
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
    let message = response.json::<serde_json::Value>().await.unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    stop_server(server);
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert!(
        message.contains("generation_destructive_drain_timeout"),
        "{message}"
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
            a4n_db::config::VideoConfigRecordInput {
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
                duration_mode: a4n_db::config::VideoDurationMode::Range,
                supported_durations_json: None,
                default_fps: 24,
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
        a4n_db::config::LlmConfigRecordInput {
            name: format!("chapter-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Llm,
        llm.id,
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
        "client_rag_embedding_identity": null,
        "client_rag_query_fingerprint": null,
        "client_rag_query_vectors": null
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
        a4n_db::config::LlmConfigRecordInput {
            name: format!("outline-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Llm,
        llm.id,
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
        a4n_db::config::LlmConfigRecordInput {
            name: format!("inspiration-process-success-{suffix}"),
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 5,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Llm,
        llm.id,
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
            "runtime_config_snapshot": {
                "schema_version": 1,
                "llm_default_source": "system",
                "system_llm_api_key_encrypted": "process-test-key"
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
        a4n_core::config::CreateImageConfigParams {
            name: format!("image-process-success-{suffix}"),
            backend: "openai-compatible-image".to_string(),
            base_url: provider.base_url(),
            api_key_encrypted: Some("process-test-key".to_string()),
            model: "process-test-image-model".to_string(),
            max_concurrent: 1,
            default_width: 1024,
            default_height: 1024,
        },
    )
    .await
    .unwrap();
    let image_config_id = image_config.id;
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Image,
        image_config_id,
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
                "config_id": image_config_id,
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
        a4n_db::config::LlmConfigRecordInput {
            name: format!("inspiration-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Llm,
        llm.id,
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
        "message": "请帮我梳理主角动机"
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
    let permissions = serde_json::json!({
        "read_project_detail": true,
        "read_blueprint": true,
        "read_outlines": true,
        "read_character_profiles": true,
        "read_foreshadowing": true,
        "automatic_writeback": false
    });
    let session = a4n_db::optimize::create_optimization_session(
        &db,
        a4n_db::optimize::OptimizationSessionCreateInput {
            project_id: project.id,
            chapter_id: chapter.id,
            name: Some("进程失败测试".to_string()),
            status: "waiting_user".to_string(),
            issues: None,
            agent_permissions: Some(permissions.clone()),
            agent_instruction: None,
            fixes_applied: 0,
            summary: Some("普通交互会话".to_string()),
        },
    )
    .await
    .unwrap();
    let llm = a4n_db::config::create_llm_config_record(
        &db,
        a4n_db::config::LlmConfigRecordInput {
            name: format!("optimize-chat-process-{suffix}"),
            provider: "openai".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_key_encrypted: None,
            model: "process-test-model".to_string(),
            max_concurrent: 1,
            timeout_secs: 1,
            max_context_tokens: 128_000,
            max_output_tokens: 8_192,
            extra_body_json: None,
        },
    )
    .await
    .unwrap();
    a4n_db::config::set_default_provider_config(
        &db,
        a4n_db::config::ProviderConfigKind::Llm,
        llm.id,
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
            "permissions": permissions
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

#[cfg(any(feature = "embedded-capabilities", feature = "embedded-llm-provider"))]
async fn read_generation_sse_events(
    response: &mut reqwest::Response,
    max_events: usize,
) -> Vec<serde_json::Value> {
    assert!(max_events > 0);
    read_generation_sse(response, Some(max_events)).await
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
        .post(format!("http://{addr}/api/commands/login_user"))
        .json(&json!({
            "input": {
                "username": username,
                "password": "generation-process-password",
            },
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
        if let Ok(entries) = std::fs::read_dir(dir)
            && let Some(path) = entries.flatten().map(|entry| entry.path()).find(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(prefix))
            })
        {
            return path;
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
