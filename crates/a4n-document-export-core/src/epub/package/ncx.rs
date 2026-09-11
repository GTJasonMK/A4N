use super::super::builder::EpubBuilder;
use super::super::xhtml::escape_xml;

mod navigation;

impl EpubBuilder {
    pub(in crate::epub) fn build_ncx(&self) -> String {
        let nav_points = navigation::build_nav_points(&self.chapters);

        format!(
            r##"<?xml version="1.0" encoding="utf-8"?>
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
</ncx>"##,
            uuid::Uuid::new_v4(),
            escape_xml(&self.metadata.title),
            nav_points,
        )
    }
}
