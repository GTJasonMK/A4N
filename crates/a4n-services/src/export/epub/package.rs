use super::builder::EpubBuilder;
use super::cover::cover_metadata;
use super::xhtml::{escape_xml, optional_metadata_tag};
use a4n_core::error::AppError;
use chrono::{SecondsFormat, Utc};

impl EpubBuilder {
    pub(super) fn build_opf(&self) -> Result<String, AppError> {
        let mut manifest = String::new();
        let mut spine = String::new();

        manifest
            .push_str(r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#);
        manifest.push_str(r#"<item id="css" href="style.css" media-type="text/css"/>"#);

        for chapter in &self.chapters {
            let id = format!("chapter_{}", chapter.order);
            let href = format!("chapter_{}.xhtml", chapter.order);
            manifest.push_str(&format!(
                r#"<item id="{id}" href="{href}" media-type="application/xhtml+xml"/>"#
            ));
            spine.push_str(&format!(r#"<itemref idref="{id}"/>"#));
        }

        if let Some(cover) = cover_metadata(&self.metadata)? {
            manifest.push_str(&format!(
                r#"<item id="cover-image" href="cover.{}" media-type="{}" properties="cover-image"/>"#,
                cover.extension, cover.mime
            ));
        }

        let date = optional_metadata_tag("dc:date", self.metadata.date.as_deref());
        let publisher = optional_metadata_tag("dc:publisher", self.metadata.publisher.as_deref());
        let description =
            optional_metadata_tag("dc:description", self.metadata.description.as_deref());
        let modified = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        Ok(format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">urn:uuid:{}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:creator>{}</dc:creator>
    <dc:language>{}</dc:language>
{date}{publisher}{description}    <meta property="dcterms:modified">{}</meta>
  </metadata>
  <manifest>
    {}
  </manifest>
  <spine toc="ncx">
    {}
  </spine>
</package>"#,
            uuid::Uuid::new_v4(),
            escape_xml(&self.metadata.title),
            escape_xml(&self.metadata.author),
            escape_xml(&self.metadata.language),
            escape_xml(&modified),
            manifest,
            spine,
        ))
    }

    pub(super) fn build_ncx(&self) -> String {
        let mut nav_points = String::new();

        for chapter in &self.chapters {
            let id = format!("chapter_{}", chapter.order);
            nav_points.push_str(&format!(
                r#"<navPoint id="nav-{id}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="chapter_{}.xhtml"/></navPoint>"#,
                chapter.order,
                escape_xml(&chapter.title),
                chapter.order,
            ));
        }

        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="urn:uuid:{}"/>
    <meta name="dtb:depth" content="1"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>{}</text></docTitle>
  <navMap>
    {}
  </navMap>
</ncx>"#,
            uuid::Uuid::new_v4(),
            escape_xml(&self.metadata.title),
            nav_points,
        )
    }
}
