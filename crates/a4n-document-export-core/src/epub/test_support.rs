use super::EpubMetadata;

pub(super) fn minimal_metadata() -> EpubMetadata {
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
