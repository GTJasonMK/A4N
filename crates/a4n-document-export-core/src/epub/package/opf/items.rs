use super::super::super::builder::EpubBuilder;
use super::super::super::cover::cover_metadata;
use crate::DocumentExportError;

pub(super) struct OpfItems {
    pub(super) manifest: String,
    pub(super) spine: String,
}

pub(super) fn build_opf_items(builder: &EpubBuilder) -> Result<OpfItems, DocumentExportError> {
    let mut manifest =
        String::from(r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#);
    manifest.push_str(r#"<item id="css" href="style.css" media-type="text/css"/>"#);

    let mut spine = String::new();
    for chapter in &builder.chapters {
        let id = format!("chapter_{}", chapter.order);
        let href = format!("chapter_{}.xhtml", chapter.order);
        manifest.push_str(&format!(
            r#"<item id="{id}" href="{href}" media-type="application/xhtml+xml"/>"#
        ));
        spine.push_str(&format!(r#"<itemref idref="{id}"/>"#));
    }

    if let Some(cover) = cover_metadata(&builder.metadata)? {
        manifest.push_str(&format!(
            r#"<item id="cover-image" href="cover.{}" media-type="{}" properties="cover-image"/>"#,
            cover.extension, cover.mime
        ));
    }

    Ok(OpfItems { manifest, spine })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epub::{EpubChapter, EpubMetadata};

    #[test]
    fn builds_manifest_and_spine_in_package_order() {
        let metadata = EpubMetadata {
            title: "测试小说".into(),
            author: "测试作者".into(),
            language: "zh-CN".into(),
            publisher: None,
            date: None,
            description: None,
            cover_image: Some(vec![1, 2, 3]),
            cover_mime: Some("image/png".into()),
        };
        let mut builder = EpubBuilder::new(metadata, "body {}".into());
        builder
            .add_chapter(EpubChapter {
                title: "第二章".into(),
                content: "内容".into(),
                order: 2,
            })
            .add_chapter(EpubChapter {
                title: "第一章".into(),
                content: "内容".into(),
                order: 1,
            });

        let items = build_opf_items(&builder).unwrap();

        assert_eq!(
            items.manifest,
            concat!(
                r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#,
                r#"<item id="css" href="style.css" media-type="text/css"/>"#,
                r#"<item id="chapter_2" href="chapter_2.xhtml" media-type="application/xhtml+xml"/>"#,
                r#"<item id="chapter_1" href="chapter_1.xhtml" media-type="application/xhtml+xml"/>"#,
                r#"<item id="cover-image" href="cover.png" media-type="image/png" properties="cover-image"/>"#,
            )
        );
        assert_eq!(
            items.spine,
            r#"<itemref idref="chapter_2"/><itemref idref="chapter_1"/>"#
        );
    }
}
