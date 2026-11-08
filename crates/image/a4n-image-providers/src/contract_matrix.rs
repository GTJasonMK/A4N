use a4n_domain::image::IMAGE_PROVIDER_CAPABILITIES;
use a4n_image_contract::{ImageGenerationRequest, ImageReference};
use a4n_test_support::{
    model_contract::assert_case,
    provider_http::{LocalProvider, ProviderResponse},
};
use base64::Engine as _;
use serde_json::{Value, json};

fn png() -> Vec<u8> {
    reference_png(0)
}

fn reference_png(index: usize) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(3, 2, image::Rgb([20 + index as u8, 40, 60]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

fn request(references: usize) -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "matrix prompt 镜头".into(),
        width: 1024,
        height: 768,
        model: "grok-imagine-image-2.0".into(),
        reference_images: (0..references)
            .map(|index| ImageReference {
                label: format!("binding-{index}"),
                data_uri: format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(reference_png(index))
                ),
            })
            .collect(),
    }
}

async fn fixture(output: Vec<u8>, status: u16, malformed: bool) -> LocalProvider {
    LocalProvider::start(move |request| {
        if status != 200 { return ProviderResponse::json(status,json!({"error":{"message":"injected adapter fault"}})); }
        if malformed { return ProviderResponse::bytes("application/json",b"{broken".to_vec()); }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&output);
        match request.uri.split('?').next().unwrap() {
            "/prompt" => ProviderResponse::json(200,json!({"prompt_id":"matrix-job"})),
            "/history/matrix-job" => ProviderResponse::json(200,json!({"matrix-job":{"outputs":{"9":{"images":[{"filename":"result.png","subfolder":"","type":"output"}]}}}})),
            "/view" => ProviderResponse::bytes("image/png",output.clone()),
            "/sdapi/v1/txt2img" => ProviderResponse::json(200,json!({"images":[b64]})),
            "/chat/completions" => ProviderResponse::json(200,json!({"choices":[{"message":{"content":[{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{b64}")}}]}}]})),
            "/images/generations" | "/images/edits" => ProviderResponse::json(200,json!({"data":[{"b64_json":b64}]})),
            uri => panic!("unexpected actual image endpoint: {uri}"),
        }
    }).await
}

#[tokio::test]
async fn model_contract_matrix_image_real_requests_and_decodable_results() {
    a4n_test_support::model_contract::emit_request_schema(
        "image.request",
        a4n_domain::json_contract::typed_output_schema::<ImageGenerationRequest>(),
        serde_json::to_value(request(2)).unwrap(),
    );
    for capability in IMAGE_PROVIDER_CAPABILITIES {
        for references in [0, 1, 2] {
            let server = fixture(png(), 200, false).await;
            let request = request(references);
            let variant = format!("refs-{references}");
            let result =
                super::pick_image_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap()
                    .generate(request.clone())
                    .await;
            if references > 0 && !capability.supports_reference_images {
                assert_case(
                    "image.generate",
                    capability.backend,
                    &variant,
                    "unsupported-before-request",
                    json!(true),
                    json!(result.is_err() && server.requests().is_empty()),
                    json!({"requests":server.requests().len()}),
                );
                continue;
            }
            let result =
                result.unwrap_or_else(|error| panic!("{} {variant}: {error}", capability.backend));
            assert_case(
                "image.generate",
                capability.backend,
                &variant,
                "decodable-artifact",
                json!([3, 2]),
                json!(a4n_media::image_dimensions_from_bytes(&result.bytes).unwrap()),
                json!({"bytes":result.bytes.len(),"format":result.format}),
            );
            let captured = server.requests();
            let first = &captured[0];
            let text = String::from_utf8_lossy(&first.body);
            assert!(text.contains(&request.prompt));
            assert!(text.contains(&request.model));
            if capability.requires_api_key {
                assert_eq!(first.headers["authorization"], "Bearer matrix-key");
            }
            if references > 0 {
                assert!(
                    text.find("binding-0").unwrap() < text.find("binding-1").unwrap_or(usize::MAX)
                );
                if capability.backend == "openai-compatible-image-edit" {
                    assert!(first.headers["content-type"].starts_with("multipart/form-data;"));
                    let positions: Vec<_> = (0..references)
                        .map(|index| {
                            let image = reference_png(index);
                            first
                                .body
                                .windows(image.len())
                                .position(|bytes| bytes == image)
                                .unwrap()
                        })
                        .collect();
                    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
                    assert!(text.contains("1024x768"));
                    assert!(text.contains(if references == 1 {
                        "name=\"image\""
                    } else {
                        "name=\"image[]\""
                    }));
                } else {
                    let body: Value = serde_json::from_slice(&first.body).unwrap();
                    let serialized = body.to_string();
                    assert_eq!(
                        serialized.matches("data:image/png;base64,").count(),
                        references
                    );
                    let positions: Vec<_> = request
                        .reference_images
                        .iter()
                        .map(|reference| serialized.find(&reference.data_uri).unwrap())
                        .collect();
                    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
                }
            }
            match capability.backend {
                "stable-diffusion" => {
                    let body: Value = serde_json::from_slice(&first.body).unwrap();
                    assert_eq!(body["width"], 1024);
                    assert_eq!(body["height"], 768);
                }
                "comfyui" => {
                    let body: Value = serde_json::from_slice(&first.body).unwrap();
                    assert_eq!(body["prompt"]["5"]["inputs"]["width"], 1024);
                    assert_eq!(body["prompt"]["5"]["inputs"]["height"], 768);
                }
                "cliproxy-image" => {
                    let body: Value = serde_json::from_slice(&first.body).unwrap();
                    assert_eq!(body["size"], "1024x768");
                    assert_eq!(body["resolution"], "1k");
                }
                _ => {
                    assert!(text.contains("1024x768"));
                }
            }
            assert_case(
                "image.generate",
                capability.backend,
                &variant,
                "actual-request-parameters",
                json!(true),
                json!(first.method == "POST"),
                json!({"uri":first.uri,"body_sha256":a4n_domain::source_text_hash(&base64::engine::general_purpose::STANDARD.encode(&first.body)),"requests":captured.len()}),
            );
        }
    }
}

#[tokio::test]
#[ignore = "run through the model runner with exported request cases"]
async fn model_contract_matrix_request_boundary() {
    let path = std::env::var("A4N_MODEL_CONTRACT_CASES").unwrap();
    let document: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(document["call_id"], "image.request");
    let server = fixture(png(), 200, false).await;
    let provider = super::pick_image_provider(
        "openai-compatible-image-edit",
        &server.base_url,
        "matrix-key",
    )
    .unwrap();
    for case in document["cases"].as_array().unwrap() {
        let before = server.requests().len();
        let result = match serde_json::from_value::<ImageGenerationRequest>(case["value"].clone()) {
            Ok(request) => provider.generate(request).await.map(|_| ()),
            Err(error) => Err(a4n_domain::ProviderError::InvalidArgument(
                error.to_string(),
            )),
        };
        let expected = case["expected"].as_bool().unwrap();
        assert_case(
            "image.request",
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
async fn model_contract_matrix_image_faults_never_become_results() {
    for capability in IMAGE_PROVIDER_CAPABILITIES {
        for (scenario, status, output, malformed) in [
            ("auth", 401, png(), false),
            ("rate-limit", 429, png(), false),
            ("server", 503, png(), false),
            ("malformed-envelope", 200, png(), true),
            ("empty", 200, Vec::new(), false),
            (
                "fake-image",
                200,
                b"<html>not an image</html>".to_vec(),
                false,
            ),
            ("truncated-image", 200, png()[..24].to_vec(), false),
        ] {
            let server = fixture(output, status, malformed).await;
            let result =
                super::pick_image_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap()
                    .generate(request(0))
                    .await;
            assert_case(
                "image.generate",
                capability.backend,
                "text",
                scenario,
                json!(true),
                json!(result.is_err()),
                json!({"requests":server.requests().len(),"error":result.err().map(|error|error.to_string())}),
            );
        }
        for (scenario, mut request) in [
            ("empty-prompt", request(0)),
            ("bad-width", request(0)),
            ("empty-model", request(0)),
            ("reference-base64", request(1)),
            ("reference-corrupt", request(1)),
            ("reference-mime", request(1)),
        ] {
            match scenario {
                "empty-prompt" => request.prompt = " \n ".into(),
                "bad-width" => request.width = 0,
                "reference-base64" => {
                    request.reference_images[0].data_uri = "data:image/png;base64,***".into()
                }
                "reference-corrupt" => {
                    request.reference_images[0].data_uri = format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(b"not an image")
                    )
                }
                "reference-mime" => {
                    request.reference_images[0].data_uri = request.reference_images[0]
                        .data_uri
                        .replace("image/png", "image/jpeg")
                }
                _ => request.model = String::new(),
            }
            let server = fixture(png(), 200, false).await;
            let result =
                super::pick_image_provider(capability.backend, &server.base_url, "matrix-key")
                    .unwrap()
                    .generate(request)
                    .await;
            assert_case(
                "image.generate",
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
