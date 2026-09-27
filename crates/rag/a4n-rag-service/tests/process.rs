use a4n_service_protocol::{
    BodyLimitKind, CapabilityTaskEventKind, RAG_INGEST_EVENTS_PATH, RAG_METADATA_BY_IDS_PATH,
    SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode, ServiceRequest,
};
use a4n_test_support::process::{
    CompletenessByIdsPayload, DeleteFilterPayload, DeleteRecordsPayload, DropCollectionPayload,
    EmbeddingRecordPayload, HttpRagClient, IngestPayload, MetadataByFilterPayload,
    MetadataByIdsPayload, QueryPayload, RAG_INDEX_CAPABILITY_VERSION, RAG_SERVICE_NAME, RagClient,
    SIDECAR_PROCESS_TEST_TOKEN, SearchPayload, SearchTextPayload, ServiceStatus,
    SidecarProcessTestDir, assert_process_concurrency_saturation,
    assert_protected_route_rejects_invalid_auth, assert_route_rejects_malformed_json,
    assert_route_rejects_oversized_json, assert_route_rejects_unknown_payload_field,
    assert_route_rejects_version_mismatch, assert_sidecar_rejects_non_loopback_without_auth,
    finish_sidecar, force_finish_sidecar, request_context, sidecar_base_url,
    sidecar_manager_with_dir, start_sidecar, trusted_remote_request_context, unused_local_addr,
};
use serde_json::{Value, json};
use std::time::Duration;

const COLLECTION: &str = "42";
const MUTATION_MARKER: &str = ".a4n-rag-mutation-in-progress";

fn rag_request(label: &str, payload: IngestPayload) -> ServiceRequest<IngestPayload> {
    request_context(
        label,
        &format!("{label}-trace"),
        RAG_INDEX_CAPABILITY_VERSION,
        payload,
    )
}

async fn ingest_records(client: &HttpRagClient, request: ServiceRequest<IngestPayload>) -> usize {
    let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    client
        .ingest_events(request, events_tx)
        .await
        .expect("RAG ingest events should cross the independent process");
    while let Some(event) = events_rx.recv().await {
        if event.event_kind == CapabilityTaskEventKind::Completed {
            let written = event
                .payload
                .expect("RAG completed payload")
                .as_u64()
                .expect("RAG completed written count");
            return usize::try_from(written).expect("RAG written count fits usize");
        }
        assert!(
            !event.event_kind.is_terminal(),
            "RAG ingest failed: {:?}",
            event.error
        );
    }
    panic!("RAG ingest event stream ended without completion");
}

fn canonical_records() -> Vec<EmbeddingRecordPayload> {
    vec![
        EmbeddingRecordPayload {
            id: "doc-1".to_string(),
            embedding: vec![1.0, 0.0, 0.0],
            metadata: json!({"data_type": "chapter", "chapter_number": 1}),
            content: "第一条重建哨兵正文".to_string(),
        },
        EmbeddingRecordPayload {
            id: "doc-2".to_string(),
            embedding: vec![0.0, 1.0, 0.0],
            metadata: json!({"data_type": "chapter", "chapter_number": 2}),
            content: "第二条删除哨兵正文".to_string(),
        },
        EmbeddingRecordPayload {
            id: "doc-3".to_string(),
            embedding: vec![0.0, 0.0, 1.0],
            metadata: json!({"data_type": "chapter", "chapter_number": 3}),
            content: "第三条断连哨兵正文".to_string(),
        },
    ]
}

async fn assert_complete<T: AsRef<str>>(client: &HttpRagClient, expected_ids: &[T]) {
    let expected_ids = expected_ids
        .iter()
        .map(|id| id.as_ref().to_string())
        .collect::<Vec<_>>();
    let expected_count = expected_ids.len();
    let payload = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let response = client
                .completeness_report_by_ids(request_context(
                    "rag-process-completeness",
                    "rag-process-completeness-trace",
                    RAG_INDEX_CAPABILITY_VERSION,
                    CompletenessByIdsPayload {
                        collection: COLLECTION.to_string(),
                        expected_ids: expected_ids.clone(),
                    },
                ))
                .await
                .expect("RAG completeness request should cross the independent process");
            if response.status == ServiceStatus::Ok
                && response
                    .payload
                    .as_ref()
                    .is_some_and(|payload| payload.is_complete)
            {
                return response.payload.expect("RAG completeness payload");
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("RAG indexes should become complete before timeout");
    assert_eq!(payload.actual_vector, expected_count);
    assert_eq!(payload.actual_bm25, expected_count);
}

async fn search_ids(client: &HttpRagClient, query: &str) -> Vec<String> {
    let response = client
        .search_text(request_context(
            "rag-process-search",
            "rag-process-search-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            SearchTextPayload {
                collection: COLLECTION.to_string(),
                query_text: query.to_string(),
                top_k: 10,
                filters: None,
            },
        ))
        .await
        .expect("RAG text search should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    response
        .payload
        .expect("RAG search payload")
        .iter()
        .map(|row| row.id.clone())
        .collect()
}

#[tokio::test]
async fn rag_service_process_persists_rebuilds_and_recovers() {
    let addr = unused_local_addr();
    let temp_dir = SidecarProcessTestDir::new("rag");
    let rag_env = [
        (
            "A4N_RAG_SERVICE_DATA_DIR".to_string(),
            temp_dir.path().to_string_lossy().into_owned(),
        ),
        (
            "A4N_QDRANT_URL".to_string(),
            std::env::var("A4N_QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6334".to_string()),
        ),
        (
            "A4N_QDRANT_COLLECTION".to_string(),
            format!("a4n_rag_test_{}", std::process::id()),
        ),
    ];
    assert_sidecar_rejects_non_loopback_without_auth(
        "rag-non-loopback",
        env!("CARGO_BIN_EXE_a4n-rag-service"),
        "A4N_RAG_SERVICE_ADDR",
        rag_env.clone(),
    )
    .await;
    let manager = sidecar_manager_with_dir(
        "rag",
        env!("CARGO_BIN_EXE_a4n-rag-service"),
        addr,
        "A4N_RAG_SERVICE_ADDR",
        &temp_dir,
        rag_env,
    );
    let process = start_sidecar(&manager, RAG_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, RAG_SERVICE_NAME, RAG_METADATA_BY_IDS_PATH).await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, RAG_METADATA_BY_IDS_PATH).await;
    assert_route_rejects_malformed_json(&base_url, RAG_METADATA_BY_IDS_PATH).await;
    assert_route_rejects_oversized_json(
        &base_url,
        RAG_METADATA_BY_IDS_PATH,
        RAG_SERVICE_NAME,
        BodyLimitKind::Json,
    )
    .await;
    let client = HttpRagClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let request = request_context(
        "rag-process-metadata",
        "rag-process-trace",
        RAG_INDEX_CAPABILITY_VERSION,
        MetadataByIdsPayload {
            collection: COLLECTION.to_string(),
            ids: Vec::new(),
        },
    );
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        RAG_METADATA_BY_IDS_PATH,
        &request,
    )
    .await;
    let response = client
        .metadata_by_ids(request)
        .await
        .expect("typed RAG metadata lookup should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    assert!(response.payload.expect("RAG metadata payload").is_empty());

    let records = canonical_records();
    let ingest_request = trusted_remote_request_context(
        "rag-process-ingest",
        "rag-process-ingest-trace",
        RAG_INDEX_CAPABILITY_VERSION,
        IngestPayload {
            collection: COLLECTION.to_string(),
            records: records[..2].to_vec(),
        },
    );
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        RAG_INGEST_EVENTS_PATH,
        &ingest_request,
    )
    .await;
    assert_eq!(ingest_records(&client, ingest_request).await, 2);
    assert_complete(&client, &["doc-1", "doc-2"]).await;
    let metadata = client
        .metadata_by_filter(request_context(
            "rag-process-metadata-filter",
            "rag-process-metadata-filter-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            MetadataByFilterPayload {
                collection: COLLECTION.to_string(),
                filter: DeleteFilterPayload {
                    ids: vec!["doc-1".to_string()],
                    id_prefixes: Vec::new(),
                },
            },
        ))
        .await
        .expect("typed RAG metadata filter should cross the independent process");
    assert_eq!(metadata.status, ServiceStatus::Ok);
    assert!(
        metadata
            .payload
            .expect("RAG metadata filter payload")
            .contains_key("doc-1")
    );
    assert_eq!(search_ids(&client, "重建").await, vec!["doc-1"]);

    let backpressure_client =
        HttpRagClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    let (backpressure_tx, mut backpressure_rx) = tokio::sync::mpsc::channel(1);
    let backpressure = tokio::spawn(async move {
        backpressure_client
            .ingest_events(
                rag_request(
                    "rag-process-backpressure",
                    IngestPayload {
                        collection: COLLECTION.to_string(),
                        records: vec![EmbeddingRecordPayload {
                            id: "doc-backpressure".to_string(),
                            embedding: vec![1.0, 1.0, 0.0],
                            metadata: json!({"data_type": "chapter", "chapter_number": 4}),
                            content: "背压完成后仍应持久化".to_string(),
                        }],
                    },
                ),
                backpressure_tx,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while backpressure_rx.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("RAG stream should fill the bounded event queue");
    assert!(
        !backpressure.is_finished(),
        "a full RAG event queue must apply backpressure"
    );
    loop {
        let event = backpressure_rx.recv().await.expect("RAG terminal event");
        if event.event_kind.is_terminal() {
            assert_eq!(
                event.event_kind,
                a4n_service_protocol::CapabilityTaskEventKind::Completed
            );
            break;
        }
    }
    backpressure
        .await
        .expect("RAG backpressure task should join")
        .expect("RAG stream should finish after queue capacity is released");
    let mut expected_ids = vec![
        "doc-1".to_string(),
        "doc-2".to_string(),
        "doc-backpressure".to_string(),
    ];
    assert_complete(&client, &expected_ids).await;

    let failed_query = client
        .query(request_context(
            "rag-process-query-failure",
            "rag-process-query-failure-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            QueryPayload {
                collection: COLLECTION.to_string(),
                query_embedding: vec![1.0, 0.0],
                top_k: 1,
                filters: None,
            },
        ))
        .await
        .expect("RAG dimension failure should return a typed response");
    assert_eq!(failed_query.status, ServiceStatus::Error);
    assert_eq!(
        failed_query.error.expect("RAG query error").code,
        ServiceErrorCode::ProviderError
    );

    let queried = client
        .query(request_context(
            "rag-process-query",
            "rag-process-query-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            QueryPayload {
                collection: COLLECTION.to_string(),
                query_embedding: vec![1.0, 0.0, 0.0],
                top_k: 1,
                filters: None,
            },
        ))
        .await
        .expect("RAG vector query should cross the independent process");
    assert_eq!(queried.status, ServiceStatus::Ok);
    assert_eq!(queried.payload.expect("RAG query payload")[0].id, "doc-1");

    let hybrid = client
        .search(request_context(
            "rag-process-hybrid-search",
            "rag-process-hybrid-search-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            SearchPayload {
                collection: COLLECTION.to_string(),
                query_text: "重建".to_string(),
                query_embedding: vec![1.0, 0.0, 0.0],
                top_k: 1,
                filters: None,
            },
        ))
        .await
        .expect("RAG hybrid search should cross the independent process");
    assert_eq!(hybrid.status, ServiceStatus::Ok);
    assert_eq!(
        hybrid.payload.expect("RAG hybrid search payload")[0].id,
        "doc-1"
    );

    let mut idle_records = Vec::with_capacity(512);
    for index in 0..512 {
        let id = format!("stream-idle-{index}");
        expected_ids.push(id.clone());
        idle_records.push(EmbeddingRecordPayload {
            id,
            embedding: vec![0.0, 0.0, 1.0],
            metadata: json!({"data_type": "chapter", "chapter_number": 3}),
            content: format!("stream idle 索引一致性哨兵 {index}"),
        });
    }
    let idle_client =
        HttpRagClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN)
            .with_test_timeouts(
                Duration::from_secs(1),
                Duration::from_secs(30),
                Duration::from_millis(10),
                Duration::from_secs(1),
            );
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::channel(1);
    let idle_task = tokio::spawn(async move {
        idle_client
            .ingest_events(
                rag_request(
                    "rag-process-stream-idle",
                    IngestPayload {
                        collection: COLLECTION.to_string(),
                        records: idle_records,
                    },
                ),
                idle_tx,
            )
            .await
    });
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), idle_rx.recv())
            .await
            .expect("RAG should emit progress before the idle deadline")
            .expect("RAG should emit progress before the idle deadline")
            .event_kind,
        a4n_service_protocol::CapabilityTaskEventKind::Progress
    );
    let idle_error = idle_task
        .await
        .expect("RAG stream idle client task should join")
        .expect_err("a stalled RAG event stream must hit the idle deadline");
    assert_eq!(idle_error.code, ServiceErrorCode::ServiceUnavailable);
    assert!(idle_error.message.contains("event stream idle timeout"));
    assert_complete(&client, &expected_ids).await;
    assert!(!temp_dir.path().join(MUTATION_MARKER).exists());

    let event_client =
        HttpRagClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(1);
    let event_task = tokio::spawn(async move {
        event_client
            .ingest_events(
                rag_request(
                    "rag-process-disconnect",
                    IngestPayload {
                        collection: COLLECTION.to_string(),
                        records: vec![records[2].clone()],
                    },
                ),
                event_tx,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while event_rx.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("RAG event ingest should emit initial progress");
    drop(event_rx);
    let disconnect = event_task
        .await
        .expect("RAG event client task should join")
        .expect_err("closed event receiver should cancel the HTTP stream");
    assert_eq!(disconnect.code, ServiceErrorCode::Cancelled);
    expected_ids.push("doc-3".to_string());
    assert_complete(&client, &expected_ids).await;
    assert!(
        search_ids(&client, "第三条")
            .await
            .contains(&"doc-3".to_string())
    );
    assert!(!temp_dir.path().join(MUTATION_MARKER).exists());

    finish_sidecar(process, &manager, RAG_SERVICE_NAME, addr).await;

    let restarted = start_sidecar(&manager, RAG_SERVICE_NAME).await;
    assert_complete(&client, &expected_ids).await;
    assert!(
        search_ids(&client, "stream idle")
            .await
            .contains(&"stream-idle-0".to_string())
    );

    let deleted = client
        .delete(request_context(
            "rag-process-delete",
            "rag-process-delete-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            DeleteRecordsPayload {
                collection: COLLECTION.to_string(),
                filter: DeleteFilterPayload {
                    ids: vec!["doc-2".to_string()],
                    id_prefixes: Vec::new(),
                },
            },
        ))
        .await
        .expect("RAG delete should cross the independent process");
    assert_eq!(deleted.status, ServiceStatus::Ok);
    assert_eq!(deleted.payload.expect("RAG delete payload"), 1);
    let remaining_after_delete = search_ids(&client, "删除").await;
    assert!(
        remaining_after_delete.is_empty(),
        "deleted RAG records remained searchable: {remaining_after_delete:?}"
    );

    let dropped = client
        .drop_collection(request_context(
            "rag-process-drop",
            "rag-process-drop-trace",
            RAG_INDEX_CAPABILITY_VERSION,
            DropCollectionPayload {
                collection: COLLECTION.to_string(),
            },
        ))
        .await
        .expect("RAG collection drop should cross the independent process");
    assert_eq!(dropped.status, ServiceStatus::Ok);
    assert!(dropped.payload.is_some());
    let rebuilt = ingest_records(
        &client,
        rag_request(
            "rag-process-rebuild",
            IngestPayload {
                collection: COLLECTION.to_string(),
                records: canonical_records(),
            },
        ),
    )
    .await;
    assert_eq!(rebuilt, 3);
    assert_complete(&client, &["doc-1", "doc-2", "doc-3"]).await;
    force_finish_sidecar(restarted, &manager, RAG_SERVICE_NAME, addr).await;

    let recovered = start_sidecar(&manager, RAG_SERVICE_NAME).await;
    // The recovered process gets a new pool instead of idle sockets left by SIGKILL.
    let client =
        HttpRagClient::new_with_auth_token(sidecar_base_url(addr), SIDECAR_PROCESS_TEST_TOKEN);
    assert_complete(&client, &["doc-1", "doc-2", "doc-3"]).await;
    finish_sidecar(recovered, &manager, RAG_SERVICE_NAME, addr).await;

    std::fs::write(temp_dir.path().join(MUTATION_MARKER), b"").unwrap();
    assert!(manager.start_sidecar(RAG_SERVICE_NAME).await.is_err());
    // Qdrant 为外部存储；清理本地 mutation 标记后即可从 canonical records 显式重建
    std::fs::remove_file(temp_dir.path().join(MUTATION_MARKER)).unwrap();

    let explicit_rebuild = start_sidecar(&manager, RAG_SERVICE_NAME).await;
    let rebuilt = ingest_records(
        &client,
        rag_request(
            "rag-process-explicit-rebuild",
            IngestPayload {
                collection: COLLECTION.to_string(),
                records: canonical_records(),
            },
        ),
    )
    .await;
    assert_eq!(rebuilt, 3);
    assert_complete(&client, &["doc-1", "doc-2", "doc-3"]).await;
    finish_sidecar(explicit_rebuild, &manager, RAG_SERVICE_NAME, addr).await;

    let manifest_path = temp_dir.path().join(".a4n-rag-index-data.json");
    let manifest = std::fs::read(&manifest_path).expect("RAG data manifest should exist");
    let mut incompatible: Value =
        serde_json::from_slice(&manifest).expect("RAG data manifest should be valid JSON");
    incompatible["format_version"] = json!(0);
    std::fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&incompatible).unwrap(),
    )
    .unwrap();
    assert!(manager.start_sidecar(RAG_SERVICE_NAME).await.is_err());
    std::fs::write(&manifest_path, b"{").unwrap();
    assert!(manager.start_sidecar(RAG_SERVICE_NAME).await.is_err());
    std::fs::write(&manifest_path, manifest).unwrap();
    let final_process = start_sidecar(&manager, RAG_SERVICE_NAME).await;
    finish_sidecar(final_process, &manager, RAG_SERVICE_NAME, addr).await;
}
