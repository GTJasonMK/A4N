use super::model::{EpubChapter, EpubMetadata};
use crate::DocumentExportError;

mod archive;
#[cfg(test)]
mod tests;

pub struct EpubBuilder {
    pub(super) metadata: EpubMetadata,
    pub(super) chapters: Vec<EpubChapter>,
    pub(super) stylesheet: String,
}

impl EpubBuilder {
    pub fn new(metadata: EpubMetadata, stylesheet: String) -> Self {
        Self {
            metadata,
            chapters: Vec::new(),
            stylesheet,
        }
    }

    pub fn add_chapter(&mut self, chapter: EpubChapter) -> &mut Self {
        self.chapters.push(chapter);
        self
    }

    pub fn build(&self) -> Result<Vec<u8>, DocumentExportError> {
        archive::build_epub(self)
    }
}
