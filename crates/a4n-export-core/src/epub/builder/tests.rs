use super::EpubBuilder;
use crate::ExportRenderError;
use crate::epub::test_support::minimal_metadata;
use crate::epub::{EpubChapter, builtin_epub_stylesheet};

#[test]
fn builds_minimal_epub() {
    let mut builder = EpubBuilder::new(minimal_metadata(), builtin_epub_stylesheet().to_string());
    builder.add_chapter(EpubChapter {
        title: "第一章".to_string(),
        content: "这是第一章的测试内容。第二段内容。".to_string(),
        order: 1,
    });

    let epub_data = builder.build().expect("minimal EPUB should build");

    assert!(!epub_data.is_empty());
    assert_eq!(epub_data[0..2], [0x50, 0x4b]);
}

#[test]
fn rejects_unsupported_cover_mime() {
    let metadata = crate::epub::EpubMetadata {
        cover_image: Some(vec![1, 2, 3]),
        cover_mime: Some("image/gif".into()),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let err = builder.build().unwrap_err();

    assert!(matches!(&err, ExportRenderError::InvalidEpub(message) if message.contains("MIME")));
    assert_eq!(
        err.to_string(),
        "参数无效：EPUB 封面 MIME 类型不支持: image/gif"
    );
}

#[test]
fn rejects_incomplete_cover_metadata() {
    let metadata = crate::epub::EpubMetadata {
        cover_image: Some(vec![1, 2, 3]),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let err = builder.build().unwrap_err();

    assert!(
        matches!(&err, ExportRenderError::InvalidEpub(message) if message.contains("MIME 类型"))
    );
    assert_eq!(err.to_string(), "参数无效：EPUB 封面缺少 MIME 类型");
}

#[test]
fn rejects_empty_stylesheet() {
    let builder = EpubBuilder::new(minimal_metadata(), "  ".to_string());

    let err = builder.build().unwrap_err();

    assert!(
        matches!(&err, ExportRenderError::InvalidEpub(message) if message.contains("样式不能为空"))
    );
    assert_eq!(err.to_string(), "参数无效：EPUB 样式不能为空");
}
