use super::*;
use a4n_core::error::AppError;

fn minimal_metadata() -> EpubMetadata {
    EpubMetadata {
        title: "测试小说".to_string(),
        author: "测试作者".to_string(),
        language: "zh-CN".to_string(),
        publisher: None,
        date: None,
        description: None,
        cover_image: None,
        cover_mime: None,
    }
}

#[test]
fn test_build_minimal_epub() {
    let mut builder = EpubBuilder::new(minimal_metadata(), builtin_epub_stylesheet().to_string());
    builder.add_chapter(EpubChapter {
        title: "第一章".to_string(),
        content: "这是第一章的测试内容。第二段内容。".to_string(),
        order: 1,
    });

    let result = builder.build();
    assert!(result.is_ok());
    let epub_data = result.unwrap();
    assert!(!epub_data.is_empty());
    assert_eq!(epub_data[0..2], [0x50, 0x4b]);
}

#[test]
fn build_opf_omits_missing_publication_metadata() {
    let builder = EpubBuilder::new(minimal_metadata(), builtin_epub_stylesheet().to_string());

    let opf = builder.build_opf().unwrap();

    assert!(!opf.contains("<dc:date>"));
    assert!(!opf.contains("<dc:publisher>"));
    assert!(!opf.contains("2026-01-01"));
    assert!(opf.contains(r#"<meta property="dcterms:modified">"#));
}

#[test]
fn build_opf_writes_explicit_publication_metadata() {
    let metadata = EpubMetadata {
        publisher: Some("A4N & Co".into()),
        date: Some("2026-05-26".into()),
        description: Some("A < B & C".into()),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let opf = builder.build_opf().unwrap();

    assert!(opf.contains("<dc:date>2026-05-26</dc:date>"));
    assert!(opf.contains("<dc:publisher>A4N &amp; Co</dc:publisher>"));
    assert!(opf.contains("<dc:description>A &lt; B &amp; C</dc:description>"));
}

#[test]
fn build_rejects_unsupported_cover_mime() {
    let metadata = EpubMetadata {
        cover_image: Some(vec![1, 2, 3]),
        cover_mime: Some("image/gif".into()),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let err = builder.build().unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("MIME")));
}

#[test]
fn build_rejects_incomplete_cover_metadata() {
    let metadata = EpubMetadata {
        cover_image: Some(vec![1, 2, 3]),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let err = builder.build().unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("MIME 类型")));
}

#[test]
fn build_rejects_empty_stylesheet() {
    let builder = EpubBuilder::new(minimal_metadata(), "  ".to_string());

    let err = builder.build().unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("样式不能为空")));
}
