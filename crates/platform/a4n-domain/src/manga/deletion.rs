use super::{
    MangaPageGenerationLayoutV4, MangaPagePipelineStage, MangaPagePipelineStatus,
    MangaPipelineArtifactStage,
};

impl MangaPageGenerationLayoutV4 {
    /// Clear projections by dependency, including outputs published before their artifact row.
    pub fn clear_deleted_stage(&mut self, deleted_stage: MangaPipelineArtifactStage) {
        for stage in MangaPipelineArtifactStage::all()
            .iter()
            .copied()
            .filter(|stage| *stage == deleted_stage || stage.depends_on(deleted_stage))
        {
            match stage {
                MangaPipelineArtifactStage::PageContent => self.pipeline.page_content = None,
                MangaPipelineArtifactStage::PageAdaptation => self.pipeline.page_adaptation = None,
                MangaPipelineArtifactStage::Storyboard => self.pipeline.storyboard = None,
                MangaPipelineArtifactStage::ReferenceBindings => self.reference_bindings.clear(),
                MangaPipelineArtifactStage::ReferenceDiscovery => {
                    self.pipeline.reference_discovery = None
                }
                MangaPipelineArtifactStage::DrawingPrompt => self.pipeline.drawing_prompt = None,
                MangaPipelineArtifactStage::Image => self.pipeline.image = None,
                MangaPipelineArtifactStage::Preprocessing
                | MangaPipelineArtifactStage::PageSlots => continue,
            }
            self.pipeline
                .stage_fingerprints
                .remove(stage.fingerprint_key());
        }
        self.pipeline.status = MangaPagePipelineStatus::Pending;
        self.pipeline.error = None;
        self.pipeline.stage = match deleted_stage {
            MangaPipelineArtifactStage::Preprocessing
            | MangaPipelineArtifactStage::PageSlots
            | MangaPipelineArtifactStage::PageContent => MangaPagePipelineStage::Content,
            MangaPipelineArtifactStage::PageAdaptation => MangaPagePipelineStage::Adaptation,
            MangaPipelineArtifactStage::Storyboard => MangaPagePipelineStage::Storyboard,
            MangaPipelineArtifactStage::ReferenceBindings
            | MangaPipelineArtifactStage::ReferenceDiscovery => {
                MangaPagePipelineStage::ReferenceDiscovery
            }
            MangaPipelineArtifactStage::DrawingPrompt => MangaPagePipelineStage::DrawingPrompt,
            MangaPipelineArtifactStage::Image => MangaPagePipelineStage::Image,
        };
    }
}
