use super::super::builder::EpubBuilder;
use super::super::xhtml::{escape_xml, optional_metadata_tag};
use crate::DocumentExportError;
use chrono::{SecondsFormat, Utc};

mod items;
#[cfg(test)]
mod tests;

impl EpubBuilder {
    pub(in crate::epub) fn build_opf(&self) -> Result<String, DocumentExportError> {
        let items::OpfItems { manifest, spine } = items::build_opf_items(self)?;

        let date = optional_metadata_tag("dc:date", self.metadata.date.as_deref());
        let publisher = optional_metadata_tag("dc:publisher", self.metadata.publisher.as_deref());
        let description =
            optional_metadata_tag("dc:description", self.metadata.description.as_deref());
        let modified = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        Ok(format!(
            r##"<?xml version="1.0" encoding="utf-8"?>
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
</package>"##,
            uuid::Uuid::new_v4(),
            escape_xml(&self.metadata.title),
            escape_xml(&self.metadata.author),
            escape_xml(&self.metadata.language),
            escape_xml(&modified),
            manifest,
            spine,
        ))
    }
}
