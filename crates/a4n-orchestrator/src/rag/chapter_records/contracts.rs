#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChapterRagChunkingMode {
    Semantic,
    ClientDesktop,
}

impl ChapterRagChunkingMode {
    pub(super) fn is_semantic(self) -> bool {
        self == Self::Semantic
    }
}
