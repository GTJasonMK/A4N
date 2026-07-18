use a4n_core::models::{
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    media::{media_asset, media_binding, media_provenance},
    project::{source_snapshot_chapter, text_anchor},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameMediaBindingResult {
    pub binding: media_binding::Model,
    pub asset: media_asset::Model,
}

#[derive(Debug, Clone, Default)]
pub struct GalgameMediaBindingReadModel {
    pub bindings: Vec<media_binding::Model>,
    pub assets: Vec<media_asset::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct GalgamePipelinePromptContextRecord {
    pub chapters: Vec<GalgamePipelinePromptChapterRecord>,
    pub speaker_profiles: Vec<protagonist_profile::Model>,
}

#[derive(Debug, Clone)]
pub struct GalgamePipelinePromptChapterRecord {
    pub chapter: source_snapshot_chapter::Model,
    pub text_anchors: Vec<text_anchor::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct GalgamePersistValidationScopeReadModel {
    pub source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
    pub speaker_profiles: Vec<protagonist_profile::Model>,
    pub text_anchors: Vec<text_anchor::Model>,
}

#[derive(Debug, Clone)]
pub struct GalgameScriptDetailReadModel {
    pub script: galgame_script::Model,
    pub speaker_profiles: Vec<protagonist_profile::Model>,
    pub text_anchors: Vec<text_anchor::Model>,
    pub media_bindings: GalgameMediaBindingReadModel,
    pub scenes: Vec<galgame_scene::Model>,
    pub lines: Vec<galgame_line::Model>,
    pub choices: Vec<galgame_choice::Model>,
}

#[derive(Debug, Clone)]
pub struct GalgameAssetPlanningReadModel {
    pub script: galgame_script::Model,
    pub scenes: Vec<galgame_scene::Model>,
    pub lines: Vec<galgame_line::Model>,
    pub source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
}

#[derive(Debug, Clone)]
pub struct CompletedGalgameVisualBindingRecords {
    pub binding: media_binding::Model,
    pub asset: media_asset::Model,
    pub provenance: media_provenance::Model,
}
