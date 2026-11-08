use super::*;
use a4n_test_support::{
    model_contract::assert_case,
    provider_http::{LocalProvider, ProviderResponse},
};
use serde_json::json;

fn provider(base_url: &str) -> HttpEmbeddingProvider {
    HttpEmbeddingProvider::new_with_batch(
        base_url,
        "matrix-embedding",
        2,
        Some("matrix-key".into()),
        2,
    )
}

#[tokio::test]
async fn model_contract_matrix_embedding_http_batch_identity_and_order() {
    let server = LocalProvider::start(|request| {
        assert_eq!(request.uri,"/embeddings");
        assert_eq!(request.headers["authorization"],"Bearer matrix-key");
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"],"matrix-embedding");
        let input = body["input"].as_array().unwrap();
        assert!((1..=2).contains(&input.len()));
        ProviderResponse::json(200,json!({"data":input.iter().enumerate().rev().map(|(index,text)|json!({"index":index,"embedding":[text.as_str().unwrap().parse::<f32>().unwrap(),1.0]})).collect::<Vec<_>>()}))
    }).await;
    let (vectors, identity) = provider(&server.base_url)
        .embed_with_identity((0..5).map(|index| index.to_string()).collect())
        .await
        .unwrap();
    assert_case(
        "embedding.embed",
        "api",
        "batch",
        "order-and-identity",
        json!({"vectors":[[0.0,1.0],[1.0,1.0],[2.0,1.0],[3.0,1.0],[4.0,1.0]],"backend":"Api","model":"matrix-embedding","dimension":2}),
        json!({"vectors":vectors,"backend":identity.backend,"model":identity.model,"dimension":identity.dimension}),
        json!({"actual_requests":server.requests().len()}),
    );
    assert_case(
        "embedding.embed",
        "api",
        "batch",
        "actual-request-order",
        json!(["0", "1", "2", "3", "4"]),
        json!(
            server
                .requests()
                .iter()
                .flat_map(|request| {
                    serde_json::from_slice::<Value>(&request.body).unwrap()["input"]
                        .as_array()
                        .unwrap()
                        .clone()
                })
                .collect::<Vec<_>>()
        ),
        json!({"batch_sizes":server.requests().iter().map(|request|serde_json::from_slice::<Value>(&request.body).unwrap()["input"].as_array().unwrap().len()).collect::<Vec<_>>()}),
    );
}

#[tokio::test]
async fn model_contract_matrix_embedding_http_response_faults() {
    for (scenario, status, body) in [
        ("auth", 401, json!({"error":"unauthorized"})),
        ("rate-limit", 429, json!({"error":"busy"})),
        ("server", 503, json!({"error":"unavailable"})),
        ("missing-data", 200, json!({})),
        ("count", 200, json!({"data":[]})),
        (
            "dimension",
            200,
            json!({"data":[{"index":0,"embedding":[1.0]}]}),
        ),
        (
            "missing-index",
            200,
            json!({"data":[{"embedding":[1.0,2.0]}]}),
        ),
        (
            "index-type",
            200,
            json!({"data":[{"index":"0","embedding":[1.0,2.0]}]}),
        ),
        (
            "index-range",
            200,
            json!({"data":[{"index":1,"embedding":[1.0,2.0]}]}),
        ),
        (
            "value-type",
            200,
            json!({"data":[{"index":0,"embedding":["1",2.0]}]}),
        ),
        (
            "float-overflow",
            200,
            json!({"data":[{"index":0,"embedding":[1e100,2.0]}]}),
        ),
    ] {
        let server =
            LocalProvider::start(move |_| ProviderResponse::json(status, body.clone())).await;
        let result = provider(&server.base_url).embed(vec!["text".into()]).await;
        assert_case(
            "embedding.embed",
            "api",
            "response",
            scenario,
            json!(true),
            json!(result.is_err()),
            json!({"requests":server.requests().len(),"error":result.err().map(|error|error.to_string())}),
        );
        assert_eq!(
            server.requests().len(),
            1,
            "embedding must not blindly repeat a failed batch"
        );
    }
    let server = LocalProvider::start(|_| {
        ProviderResponse::json(
            200,
            json!({"data":[{"index":0,"embedding":[1.0,2.0]},{"index":0,"embedding":[3.0,4.0]}]}),
        )
    })
    .await;
    let result = provider(&server.base_url)
        .embed(vec!["first".into(), "second".into()])
        .await;
    assert_case(
        "embedding.embed",
        "api",
        "response",
        "duplicate-index",
        json!(true),
        json!(result.is_err()),
        json!({"requests":server.requests().len()}),
    );
}

#[tokio::test]
async fn model_contract_matrix_embedding_invalid_input_does_not_send() {
    let server = LocalProvider::start(|_| panic!("invalid embedding input reached HTTP")).await;
    for (scenario, dimension, batch, model) in [
        ("batch", 2, 0, "model"),
        ("dimension", 0, 2, "model"),
        ("model", 2, 2, " "),
    ] {
        let result =
            HttpEmbeddingProvider::new_with_batch(&server.base_url, model, dimension, None, batch)
                .embed(vec!["text".into()])
                .await;
        assert_case(
            "embedding.embed",
            "api",
            "invalid-input",
            scenario,
            json!(true),
            json!(result.is_err() && server.requests().is_empty()),
            json!({"requests":server.requests().len()}),
        );
    }
    assert!(
        provider(&server.base_url)
            .embed(Vec::new())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(server.requests().is_empty());
}
