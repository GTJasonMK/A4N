use a4n_test_support::{
    model_contract::{AUDIO_ONLY_MP4, VALID_VIDEO_MP4, assert_case},
    provider_http::{LocalProvider, ProviderResponse},
};
use a4n_video_provider_contract::{VIDEO_PROVIDER_CAPABILITIES, VideoJobState, VideoJobStatus};
use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};
use base64::Engine as _;
use serde_json::{Value, json};

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

fn request(mode: &str) -> VideoGenerationRequest {
    let media = match mode {
        "text" => VideoMediaInput::None,
        "first" => VideoMediaInput::Frames {
            first: PNG.into(),
            last: None,
            references: vec![],
        },
        "first-last" => VideoMediaInput::Frames {
            first: PNG.into(),
            last: Some(PNG.into()),
            references: vec![],
        },
        "references" => VideoMediaInput::References {
            images: vec![PNG.into(), PNG.into()],
            videos: vec![],
            audio: vec![],
        },
        "first-and-references" => VideoMediaInput::Frames {
            first: PNG.into(),
            last: None,
            references: vec![PNG.into()],
        },
        "video-audio" => VideoMediaInput::References {
            images: vec![],
            videos: vec!["https://example.invalid/ref.mp4".into()],
            audio: vec!["https://example.invalid/ref.wav".into()],
        },
        _ => panic!("unknown video fixture mode"),
    };
    VideoGenerationRequest {
        prompt: "matrix video prompt 镜头".into(),
        negative_prompt: Some("matrix negative".into()),
        width: 1280,
        height: 720,
        duration_secs: 6,
        fps: 24,
        model: "matrix-video".into(),
        media,
    }
}

async fn fixture(backend: &'static str, scenario: &'static str) -> LocalProvider {
    LocalProvider::start(move |request| {
        let status=match scenario {"auth"=>401,"rate-limit"=>429,"server"=>503,_=>200};
        if status!=200 {return ProviderResponse::json(status,json!({"error":{"message":"injected video fault"}}));}
        if scenario=="malformed-envelope" {return ProviderResponse::bytes("application/json",b"{invalid".to_vec());}
        if scenario=="disconnect" {let mut response=ProviderResponse::bytes("application/json",b"{".to_vec());response.stream_error=true;return response;}
        if request.uri.starts_with("/fixture") || request.uri.contains("/content") {
            if matches!(scenario,"download-failure"|"completed-no-artifact") {return ProviderResponse::json(404,json!({"error":"gone"}));}
            let bytes=match scenario {"corrupt-artifact"=>&VALID_VIDEO_MP4[..32],"no-video-stream"=>AUDIO_ONLY_MP4,"empty-artifact"=>b"".as_slice(),_=>VALID_VIDEO_MP4};
            return ProviderResponse::bytes("video/mp4",bytes.to_vec());
        }
        let url=format!("http://{}/fixture.mp4",request.headers["host"]);
        if request.method=="DELETE" {return ProviderResponse::json(200,json!({"deleted":true}));}
        if request.method=="POST" {
            if scenario=="missing-job" {return ProviderResponse::json(200,json!({"status":"pending"}));}
            return match backend {
                "gemini-veo-video"=>ProviderResponse::json(200,json!({"name":"operations/matrix-job"})),
                "openai-chat-video"=>ProviderResponse::json(200,json!({"choices":[{"message":{"content":url}}]})),
                _=>ProviderResponse::json(200,json!({"id":"matrix-job","status":"pending"})),
            };
        }
        if backend=="gemini-veo-video" {
            return ProviderResponse::json(200,match scenario {
                "unknown-status"=>json!({"name":"operations/matrix-job","done":"yes"}),
                "pending"=>json!({"name":"operations/matrix-job","done":false}),
                "failed"=>json!({"name":"operations/matrix-job","done":true,"error":{"message":"failed by provider"}}),
                "completed-no-artifact"=>json!({"name":"operations/matrix-job","done":true}),
                _=>json!({"name":"operations/matrix-job","done":true,"response":{"generatedVideos":[{"video":{"uri":url}}]}}),
            });
        }
        let state=match scenario {"unknown-status"=>"alien-state","pending"=>"pending","failed"=>"failed",_=>"completed"};
        if scenario=="completed-no-artifact" {return ProviderResponse::json(200,json!({"id":"matrix-job","status":state}));}
        let b64=base64::engine::general_purpose::STANDARD.encode(VALID_VIDEO_MP4);
        let mut body=json!({"id":"matrix-job","status":state,"error_message":"failed by provider","error":{"message":"failed by provider"},"video_url":url});
        if backend=="openai-compatible-video" {body["data"]=json!([{"url":url,"format":"mp4"}]);}
        if backend=="newapi-video" {body["url"]=json!(url);body["format"]=json!("mp4");}
        if scenario=="pending" || scenario=="failed" {body["b64_json"]=json!(b64);}
        ProviderResponse::json(200,body)
    }).await
}

#[tokio::test]
async fn model_contract_matrix_video_actual_parameters_and_artifacts() {
    a4n_test_support::model_contract::emit_request_schema(
        "video.request",
        a4n_domain::json_contract::typed_output_schema::<VideoGenerationRequest>(),
        serde_json::to_value(request("first-last")).unwrap(),
    );
    for capability in VIDEO_PROVIDER_CAPABILITIES {
        for mode in [
            "text",
            "first",
            "first-last",
            "references",
            "first-and-references",
            "video-audio",
        ] {
            let server = fixture(capability.backend, "valid").await;
            let provider =
                super::pick_video_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap();
            let req = request(mode);
            let supported = (mode != "first-last" || capability.last_frame)
                && (!matches!(mode, "references" | "first-and-references")
                    || capability.max_reference_images != Some(0))
                && (mode != "first-and-references" || capability.first_frame_with_references)
                && (mode != "video-audio" || capability.reference_video_audio);
            let key = "a".repeat(64);
            let submitted = provider.generate(req.clone(), Some(&key)).await;
            if !supported {
                assert_case(
                    "video.generate",
                    capability.backend,
                    mode,
                    "unsupported-before-request",
                    json!(true),
                    json!(submitted.is_err() && server.requests().is_empty()),
                    json!({"requests":server.requests().len()}),
                );
                continue;
            }
            let submitted =
                submitted.unwrap_or_else(|error| panic!("{} {mode}: {error}", capability.backend));
            let result = if capability.asynchronous {
                let job = submitted
                    .provider_job_id
                    .as_deref()
                    .expect("submitted async job identity must survive");
                assert!(submitted.bytes.is_empty());
                let state = provider
                    .poll(job)
                    .await
                    .unwrap_or_else(|error| panic!("{} poll: {error}", capability.backend));
                let VideoJobState::Completed(result) = state else {
                    panic!("expected completed real artifact")
                };
                let before = server.requests().len();
                assert!(matches!(
                    provider.status(job).await.unwrap(),
                    VideoJobStatus::Completed
                ));
                assert_eq!(
                    server.requests().len(),
                    before + 1,
                    "status must only query metadata"
                );
                assert!(!server.requests().last().unwrap().uri.contains("/fixture"));
                result
            } else {
                submitted
            };
            assert_case(
                "video.generate",
                capability.backend,
                mode,
                "real-artifact",
                json!(true),
                json!(result.bytes == VALID_VIDEO_MP4),
                json!({"format":result.format,"bytes":result.bytes.len(),"job_id":result.provider_job_id}),
            );
            let captured = server.requests();
            let post = captured
                .iter()
                .find(|request| request.method == "POST")
                .unwrap();
            assert_eq!(post.headers["idempotency-key"], key);
            let body: Value = serde_json::from_slice(&post.body).unwrap();
            let text = body.to_string();
            assert!(text.contains(&req.prompt));
            assert!(text.contains(req.negative_prompt.as_deref().unwrap()));
            match capability.backend {
                "openai-compatible-video" => {
                    assert_eq!(body["width"], 1280);
                    assert_eq!(body["height"], 720);
                    assert_eq!(body["duration_secs"], 6);
                    assert_eq!(body["fps"], 24);
                }
                "openai-chat-video" => {
                    assert_eq!(body["size"], "1280x720");
                    assert_eq!(body["duration_secs"], 6);
                    assert_eq!(body["fps"], 24);
                }
                "newapi-video" => {
                    assert_eq!(body["duration"], 6);
                    assert_eq!(body["aspect_ratio"], "16:9");
                }
                "gemini-veo-video" => {
                    assert_eq!(body["parameters"]["durationSeconds"], 6);
                    assert_eq!(body["parameters"]["aspectRatio"], "16:9");
                    assert_eq!(body["parameters"]["resolution"], "720p");
                }
                "seedance2-video" => {
                    assert_eq!(body["seconds"], "6");
                    assert_eq!(body["size"], "1280x720");
                }
                "cliproxy-openai-video" => {
                    assert_eq!(body["duration"], 6);
                    assert_eq!(body["resolution"], "720p");
                    assert_eq!(body["aspect_ratio"], "16:9");
                }
                _ => unreachable!(),
            }
            if capability.backend == "gemini-veo-video" {
                assert!(post.uri.contains("matrix-video:predictLongRunning"));
                assert_eq!(post.headers["x-goog-api-key"], "matrix-key");
            } else {
                assert_eq!(body["model"], "matrix-video");
                assert_eq!(post.headers["authorization"], "Bearer matrix-key");
            }
            if mode != "text" {
                assert!(text.contains(if mode == "video-audio" {
                    "ref.mp4"
                } else {
                    "image"
                }));
            }
            assert_case(
                "video.generate",
                capability.backend,
                mode,
                "actual-request-parameters",
                json!(true),
                json!(
                    captured
                        .iter()
                        .filter(|request| request.method == "POST")
                        .count()
                        == 1
                ),
                json!({"uri":post.uri,"request_sha256":a4n_domain::source_text_hash(&text),"requests":captured.len()}),
            );
        }
    }
}

#[tokio::test]
#[ignore = "run through the model runner with exported request cases"]
async fn model_contract_matrix_request_boundary() {
    let path = std::env::var("A4N_MODEL_CONTRACT_CASES").unwrap();
    let document: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(document["call_id"], "video.request");
    let server = fixture("openai-compatible-video", "valid").await;
    let provider =
        super::pick_video_provider("openai-compatible-video", &server.base_url, "matrix-key")
            .unwrap();
    for case in document["cases"].as_array().unwrap() {
        let before = server.requests().len();
        let result = match serde_json::from_value::<VideoGenerationRequest>(case["value"].clone()) {
            Ok(request) => provider.generate(request, None).await.map(|_| ()),
            Err(error) => Err(a4n_domain::ProviderError::InvalidArgument(
                error.to_string(),
            )),
        };
        let expected = case["expected"].as_bool().unwrap();
        assert_case(
            "video.request",
            "runtime-request",
            case["vector_key"].as_str().unwrap(),
            "before-http",
            json!({"accepted":expected,"requests":if expected {1} else {0}}),
            json!({"accepted":result.is_ok(),"requests":server.requests().len()-before}),
            json!({"vector_case_id":case["case_id"]}),
        );
    }
}

#[tokio::test]
async fn model_contract_matrix_video_status_delete_and_empty_job_are_explicit() {
    for capability in VIDEO_PROVIDER_CAPABILITIES {
        let server = fixture(capability.backend, "valid").await;
        let provider =
            super::pick_video_provider(capability.backend, &server.base_url, "matrix-key").unwrap();
        let deleted = provider.delete("matrix-job").await;
        assert_case(
            "video.generate",
            capability.backend,
            "job",
            "delete-capability",
            json!(capability.delete),
            json!(deleted.is_ok()),
            json!({"requests":server.requests().len()}),
        );
        if !capability.delete {
            assert!(server.requests().is_empty());
        }
        let before = server.requests().len();
        assert!(provider.poll(" ").await.is_err());
        assert!(provider.status("").await.is_err());
        assert!(provider.delete("").await.is_err());
        assert_case(
            "video.generate",
            capability.backend,
            "job",
            "empty-job-before-request",
            json!(before),
            json!(server.requests().len()),
            json!({"requests":server.requests().len()}),
        );
        if !capability.status_query {
            assert!(provider.status("matrix-job").await.is_err());
            assert_eq!(server.requests().len(), before);
        }
    }
}

#[tokio::test]
async fn model_contract_matrix_submit_status_controls_media_adoption() {
    for backend in ["openai-compatible-video", "newapi-video"] {
        for status in [
            "pending",
            "processing",
            "failed",
            "canceled",
            "alien-state",
            "completed",
        ] {
            let server = LocalProvider::start(move |request| {
                if request.method == "GET" {
                    return ProviderResponse::bytes("video/mp4", VALID_VIDEO_MP4.to_vec());
                }
                let url = format!("http://{}/fixture.mp4", request.headers["host"]);
                let mut value = json!({
                    "id":"submit-job", "status":status,
                    "error_message":"injected submit failure", "error":{"message":"injected submit failure"}
                });
                if backend == "openai-compatible-video" {
                    value["data"] = json!([{"url":url,"format":"mp4"}]);
                } else {
                    value["url"] = json!(url);
                    value["format"] = json!("mp4");
                }
                ProviderResponse::json(200, value)
            }).await;
            let provider = super::pick_video_provider(backend, &server.base_url, "key").unwrap();
            let result = provider.generate(request("text"), None).await;
            match status {
                "pending" | "processing" => {
                    let result = result.unwrap();
                    assert!(result.bytes.is_empty());
                    assert_eq!(result.provider_job_id.as_deref(), Some("submit-job"));
                }
                "completed" => assert_eq!(result.unwrap().bytes, VALID_VIDEO_MP4),
                _ => assert!(result.is_err(), "{backend}/{status}"),
            }
            assert_eq!(
                server.requests().len(),
                if status == "completed" { 2 } else { 1 },
                "{backend}/{status}"
            );
        }
        for status in ["completed", "done"] {
            let server = LocalProvider::start(move |_| {
                ProviderResponse::json(200, json!({"id":"job","status":status}))
            })
            .await;
            let provider = super::pick_video_provider(backend, &server.base_url, "key").unwrap();
            assert!(provider.generate(request("text"), None).await.is_err());
            assert_eq!(server.requests().len(), 1);
        }
    }
}

#[tokio::test]
async fn model_contract_matrix_poll_and_status_share_state_rules() {
    for backend in ["openai-compatible-video", "newapi-video"] {
        for status in ["queued", "processing", "error", "canceled", "alien-state"] {
            let server = LocalProvider::start(move |_| ProviderResponse::json(200, json!({
                "status":status, "error_message":"provider failed", "error":{"message":"provider failed"}
            }))).await;
            let provider = super::pick_video_provider(backend, &server.base_url, "key").unwrap();
            let polled = provider.poll("job").await;
            let metadata = provider.status("job").await;
            match status {
                "queued" => {
                    assert!(matches!(polled.unwrap(), VideoJobState::Pending));
                    assert!(matches!(metadata.unwrap(), VideoJobStatus::Pending));
                }
                "processing" => {
                    assert!(matches!(polled.unwrap(), VideoJobState::Running));
                    assert!(matches!(metadata.unwrap(), VideoJobStatus::Running));
                }
                "error" | "canceled" => {
                    assert!(matches!(polled.unwrap(), VideoJobState::Failed(_)));
                    assert!(matches!(
                        metadata.unwrap(),
                        VideoJobStatus::Failed(_) | VideoJobStatus::Cancelled
                    ));
                }
                _ => assert!(polled.is_err() && metadata.is_err()),
            }
            assert_eq!(server.requests().len(), 2);
        }
    }
}

#[tokio::test]
async fn model_contract_matrix_rejects_duplicate_video_envelope_fields() {
    for backend in ["openai-compatible-video", "newapi-video"] {
        let server = LocalProvider::start(move |_| {
            ProviderResponse::bytes(
                "application/json",
                br#"{"id":"job","status":"failed","status":"pending"}"#.to_vec(),
            )
        })
        .await;
        let provider = super::pick_video_provider(backend, &server.base_url, "key").unwrap();
        assert!(provider.generate(request("text"), None).await.is_err());
        assert_eq!(server.requests().len(), 1);
    }
}

#[tokio::test]
async fn model_contract_matrix_video_provider_failure_and_job_states() {
    for capability in VIDEO_PROVIDER_CAPABILITIES {
        for scenario in [
            "auth",
            "rate-limit",
            "server",
            "malformed-envelope",
            "disconnect",
            "missing-job",
            "download-failure",
            "completed-no-artifact",
            "corrupt-artifact",
            "no-video-stream",
            "empty-artifact",
            "unknown-status",
            "pending",
            "failed",
        ] {
            if !capability.asynchronous
                && matches!(scenario, "unknown-status" | "pending" | "failed")
            {
                continue;
            }
            let server = fixture(capability.backend, scenario).await;
            let provider =
                super::pick_video_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap();
            let result = provider
                .generate(request("text"), Some(&"b".repeat(64)))
                .await;
            let passed = match result {
                Err(_) => !matches!(scenario, "pending" | "failed"),
                Ok(result) => {
                    if let Some(job) = result.provider_job_id {
                        match provider.poll(&job).await {
                            Err(_) => !matches!(scenario, "pending" | "failed"),
                            Ok(VideoJobState::Pending | VideoJobState::Running) => {
                                scenario == "pending"
                            }
                            Ok(VideoJobState::Failed(_)) => scenario == "failed",
                            Ok(VideoJobState::Completed(result)) => {
                                rejects_artifact(scenario, &result.bytes)
                            }
                        }
                    } else {
                        rejects_artifact(scenario, &result.bytes)
                    }
                }
            };
            assert_case(
                "video.generate",
                capability.backend,
                "fault",
                scenario,
                json!(true),
                json!(passed),
                json!({"requests":server.requests().len(),"submissions":server.requests().iter().filter(|request|request.method=="POST").count()}),
            );
            assert_eq!(
                server
                    .requests()
                    .iter()
                    .filter(|request| request.method == "POST")
                    .count(),
                1,
                "uncertain jobs must not be resubmitted"
            );
        }
        for scenario in ["prompt", "model", "width", "fps", "duration"] {
            let server = fixture(capability.backend, "valid").await;
            let provider =
                super::pick_video_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap();
            let mut req = request("text");
            match scenario {
                "prompt" => req.prompt = " \n ".into(),
                "model" => req.model = " ".into(),
                "width" => req.width = 0,
                "fps" => req.fps = 0,
                _ => req.duration_secs = capability.max_duration_secs + 1,
            }
            let result = provider.generate(req, None).await;
            assert_case(
                "video.generate",
                capability.backend,
                "invalid-input",
                scenario,
                json!(true),
                json!(result.is_err() && server.requests().is_empty()),
                json!({"requests":server.requests().len()}),
            );
        }
    }
}

fn rejects_artifact(scenario: &str, bytes: &[u8]) -> bool {
    if !matches!(
        scenario,
        "corrupt-artifact" | "no-video-stream" | "empty-artifact"
    ) {
        return false;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("candidate.mp4");
    std::fs::write(&path, bytes).unwrap();
    a4n_media_processing_core::probe_video_properties(&path).is_err()
}
