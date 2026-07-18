use a4n_core::{
    models::chapter::chapter_manga_prompt,
    source_spans::{
        ChapterSourceContext, metadata_source_content_sha256, source_metadata,
        source_text_hash as source_content_sha256,
    },
};
use serde_json::Value;

pub fn image_source_metadata(content: &str) -> Value {
    source_metadata(ChapterSourceContext { content })
}

pub fn ensure_manga_page_panels_match_current_source(
    panels: &[chapter_manga_prompt::Model],
    current_content: &str,
) -> Result<(), String> {
    let current_hash = source_content_sha256(current_content);
    for panel in panels {
        let source = panel
            .layout
            .as_ref()
            .and_then(|layout| layout.get("source"));
        if source.is_none() {
            return Err(format!(
                "漫画提示词 #{} 缺少章节来源信息，请重新生成漫画提示词",
                panel.id
            ));
        }
        let Some(source_hash) = metadata_source_content_sha256(panel.layout.as_ref()) else {
            return Err(format!(
                "漫画提示词 #{} 缺少章节正文指纹，请重新生成漫画提示词",
                panel.id
            ));
        };
        if source_hash != current_hash {
            return Err(format!(
                "漫画提示词 #{} 基于旧章节正文，请重新生成漫画提示词",
                panel.id
            ));
        }
    }
    Ok(())
}
