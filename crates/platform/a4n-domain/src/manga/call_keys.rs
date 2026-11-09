use super::{MANGA_PAGE_SLOT_LIMIT, MangaPipelineArtifactStage, MangaPreprocessingStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaModelCall {
    Preprocessing(MangaPreprocessingStep),
    Outline,
    PageContent(u32),
    PageAdaptation(u32),
    Storyboard(u32),
    ReferenceDiscovery(u32),
    DrawingPrompt(u32),
}

impl MangaModelCall {
    pub fn call_key(self, chapter_id: i64) -> String {
        match self {
            Self::Preprocessing(step) => step.call_key(chapter_id),
            Self::Outline => format!("manga_pipeline:{chapter_id}:manga_page_outline:v1"),
            Self::PageContent(page) => {
                format!("manga_pipeline:{chapter_id}:manga_page_content.{page}:v1")
            }
            Self::PageAdaptation(page) => {
                format!("manga_pipeline:{chapter_id}:page:{page}:adaptation")
            }
            Self::Storyboard(page) => format!("manga_pipeline:{chapter_id}:page:{page}:storyboard"),
            Self::ReferenceDiscovery(page) => {
                format!("manga_pipeline:{chapter_id}:page:{page}:reference_discovery")
            }
            Self::DrawingPrompt(page) => {
                format!("manga_pipeline:{chapter_id}:manga_page_drawing_prompt:page:{page}")
            }
        }
    }

    fn stage(self) -> MangaPipelineArtifactStage {
        use MangaPipelineArtifactStage as Stage;
        match self {
            Self::Preprocessing(_) => Stage::Preprocessing,
            Self::Outline => Stage::PageSlots,
            Self::PageContent(_) => Stage::PageContent,
            Self::PageAdaptation(_) => Stage::PageAdaptation,
            Self::Storyboard(_) => Stage::Storyboard,
            Self::ReferenceDiscovery(_) => Stage::ReferenceDiscovery,
            Self::DrawingPrompt(_) => Stage::DrawingPrompt,
        }
    }
}

/// Reuse the model-call identities and artifact dependency graph when discarding checkpoints.
pub fn manga_deleted_checkpoint_keys(
    chapter_id: i64,
    deleted_stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Vec<String> {
    let mut calls = MangaPreprocessingStep::ALL
        .map(MangaModelCall::Preprocessing)
        .to_vec();
    calls.push(MangaModelCall::Outline);
    let pages = page_number.map_or_else(
        || (1..=MANGA_PAGE_SLOT_LIMIT as u32).collect(),
        |page| vec![page],
    );
    for page in pages {
        calls.extend([
            MangaModelCall::PageContent(page),
            MangaModelCall::PageAdaptation(page),
            MangaModelCall::Storyboard(page),
            MangaModelCall::ReferenceDiscovery(page),
            MangaModelCall::DrawingPrompt(page),
        ]);
    }
    calls
        .into_iter()
        .filter(|call| call.stage() == deleted_stage || call.stage().depends_on(deleted_stage))
        .map(|call| call.call_key(chapter_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_keys_preserve_upstream_and_other_pages_without_prefix_collisions() {
        use MangaPipelineArtifactStage as Stage;
        let keys = manga_deleted_checkpoint_keys(9, Stage::Storyboard, Some(1));
        assert_eq!(
            keys,
            [
                "manga_pipeline:9:page:1:storyboard",
                "manga_pipeline:9:page:1:reference_discovery",
                "manga_pipeline:9:manga_page_drawing_prompt:page:1",
            ]
        );
        assert!(manga_deleted_checkpoint_keys(9, Stage::Image, Some(1)).is_empty());
        let slots = manga_deleted_checkpoint_keys(9, Stage::PageSlots, None);
        assert_eq!(slots.len(), 1 + MANGA_PAGE_SLOT_LIMIT * 5);
        assert!(slots.contains(&"manga_pipeline:9:manga_page_content.100:v1".into()));
        assert!(!slots.iter().any(|key| key.contains("manga_preprocessing")));
        let preprocessing = manga_deleted_checkpoint_keys(9, Stage::Preprocessing, None);
        assert_eq!(preprocessing.len(), slots.len() + 4);
        let content = manga_deleted_checkpoint_keys(9, Stage::PageContent, Some(10));
        assert!(content.contains(&"manga_pipeline:9:manga_page_content.10:v1".into()));
        assert!(content.contains(&"manga_pipeline:9:page:10:adaptation".into()));
        assert!(!content.contains(&MangaModelCall::Storyboard(1).call_key(9)));
        assert!(!content.contains(&MangaModelCall::Storyboard(10).call_key(19)));
    }
}
