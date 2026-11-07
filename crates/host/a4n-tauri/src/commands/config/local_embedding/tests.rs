use super::*;
use std::path::Path;
use tauri::{Manager, test::MockRuntime};

fn local_app(cache: &Path) -> tauri::App<MockRuntime> {
    let mut config = AppConfig::default();
    config.cache_dir = cache.to_path_buf();
    tauri::test::mock_builder()
        .manage(LocalEmbeddingRuntimeState::new(&config))
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("desktop test app")
}

#[test]
fn desktop_lists_local_models_without_a_server_or_sidecar() {
    let cache = tempfile::tempdir().unwrap();
    let app = local_app(cache.path());

    let models = local_list_embedding_models(app.state()).expect("local model catalog is compiled");
    assert!(models.iter().any(|model| model.name == "BGESmallZHV15"));
    assert!(models.iter().all(|model| !model.downloaded));
    assert!(models.iter().all(|model| model.size_bytes == 0));
}

#[tokio::test]
async fn desktop_model_commands_report_model_errors_and_release_failed_downloads() {
    let cache = tempfile::tempdir().unwrap();
    let app = local_app(cache.path());
    let invalid_model = "a4n-unsupported-embedding-model";
    let input = || EmbeddingModelInput {
        model: invalid_model.into(),
    };

    let error = local_get_embedding_model_download_info(app.state(), input())
        .await
        .unwrap_err();
    assert!(error.contains("不支持的 fastembed 模型"), "{error}");
    let error = local_download_embedding_model(
        app.state(),
        DownloadEmbeddingModelInput {
            model: invalid_model.into(),
            task_id: "desktop-embedding-test".into(),
        },
    )
    .await
    .unwrap_err();
    assert!(error.contains("不支持的 fastembed 模型"), "{error}");
    assert!(
        app.state::<LocalEmbeddingRuntimeState>()
            .download_tasks
            .lock()
            .await
            .is_empty()
    );
    let error = local_delete_embedding_model(app.state(), input())
        .await
        .unwrap_err();
    assert!(error.contains("不支持的 fastembed 模型"), "{error}");

    let model = local_list_embedding_models(app.state())
        .unwrap()
        .into_iter()
        .find(|model| model.name == "BGESmallZHV15")
        .unwrap();
    let config = LocalEmbeddingConfigInput {
        backend: "fastembed".into(),
        model: model.name,
        dimension: model.dimension,
        batch_size: 1,
        local_model_path: None,
    };
    let error = local_test_embedding_config(app.state(), config.clone())
        .await
        .unwrap_err();
    assert!(error.contains("尚未下载"), "{error}");
    let error = local_embed_texts(
        app.state(),
        LocalEmbedTextsInput {
            config,
            texts: vec!["旧车站的七分钟".into()],
        },
    )
    .await
    .unwrap_err();
    assert!(error.contains("尚未下载"), "{error}");
}
