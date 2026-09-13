use super::file_response;
use axum::http::{StatusCode, header};
use futures::StreamExt;

#[tokio::test]
async fn media_responses_preserve_ranges_and_stream_large_files_in_bounded_chunks() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("media.mp4");
    std::fs::write(&path, b"0123456789").unwrap();
    for (range, expected, content_range) in [
        ("bytes=2-5", "2345", "bytes 2-5/10"),
        ("bytes=7-", "789", "bytes 7-9/10"),
        ("bytes=-2", "89", "bytes 8-9/10"),
    ] {
        let response = file_response(&path, Some(&header::HeaderValue::from_static(range)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_RANGE], content_range);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "video/mp4");
        let body = axum::body::to_bytes(response.into_body(), 10)
            .await
            .unwrap();
        assert_eq!(body, expected.as_bytes());
    }
    let response = file_response(&path, Some(&header::HeaderValue::from_static("bytes=100-")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);

    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    let size = 32 * 1024 * 1024;
    file.set_len(size).unwrap();
    drop(file);
    for range in [None, Some(header::HeaderValue::from_static("bytes=0-"))] {
        let response = file_response(&path, range.as_ref()).await.unwrap();
        assert_eq!(response.headers()[header::CONTENT_LENGTH], size.to_string());
        let first = response
            .into_body()
            .into_data_stream()
            .next()
            .await
            .unwrap()
            .unwrap();
        assert!(!first.is_empty());
        assert!(
            first.len() <= 64 * 1024,
            "a large response must not buffer the entire file"
        );
    }
}
