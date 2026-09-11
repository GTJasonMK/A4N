use super::super::super::model::EpubChapter;
use super::super::super::xhtml::escape_xml;

pub(super) fn build_nav_points(chapters: &[EpubChapter]) -> String {
    chapters
        .iter()
        .map(|chapter| {
            let id = format!("chapter_{}", chapter.order);
            format!(
                r#"<navPoint id="nav-{id}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="chapter_{}.xhtml"/></navPoint>"#,
                chapter.order,
                escape_xml(&chapter.title),
                chapter.order,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_navigation_nodes_in_chapter_order() {
        let chapters = vec![
            EpubChapter {
                title: "第二章 & 后续".into(),
                content: String::new(),
                order: 2,
            },
            EpubChapter {
                title: "第一章 <开始>".into(),
                content: String::new(),
                order: 1,
            },
        ];

        let nav_points = build_nav_points(&chapters);

        assert_eq!(
            nav_points,
            concat!(
                r#"<navPoint id="nav-chapter_2" playOrder="2"><navLabel><text>第二章 &amp; 后续</text></navLabel><content src="chapter_2.xhtml"/></navPoint>"#,
                r#"<navPoint id="nav-chapter_1" playOrder="1"><navLabel><text>第一章 &lt;开始&gt;</text></navLabel><content src="chapter_1.xhtml"/></navPoint>"#,
            )
        );
    }
}
