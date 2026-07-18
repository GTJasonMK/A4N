#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MangaPipelineStartStage {
    CharacterBinding,
    SpeechSelection,
    VisualAnchorExtraction,
    ItemExtraction,
    ComicSynthesis,
    PagePlanning,
    DrawingPrompt,
}

impl MangaPipelineStartStage {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "character_binding" => Some(Self::CharacterBinding),
            "speech_selection" => Some(Self::SpeechSelection),
            "visual_anchor_extraction" => Some(Self::VisualAnchorExtraction),
            "item_extraction" => Some(Self::ItemExtraction),
            "comic_synthesis" => Some(Self::ComicSynthesis),
            "page_planning" => Some(Self::PagePlanning),
            "drawing_prompt" => Some(Self::DrawingPrompt),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::CharacterBinding => "character_binding",
            Self::SpeechSelection => "speech_selection",
            Self::VisualAnchorExtraction => "visual_anchor_extraction",
            Self::ItemExtraction => "item_extraction",
            Self::ComicSynthesis => "comic_synthesis",
            Self::PagePlanning => "page_planning",
            Self::DrawingPrompt => "drawing_prompt",
        }
    }

    pub(in crate::manga) fn status(self) -> (&'static str, i32, &'static str) {
        match self {
            Self::CharacterBinding => (self.key(), 1, "正在绑定出场角色身份与外观"),
            Self::SpeechSelection => (self.key(), 2, "正在选择关键对白、想法与旁白"),
            Self::VisualAnchorExtraction => (self.key(), 3, "正在提取场景和视觉锚点"),
            Self::ItemExtraction => (self.key(), 4, "正在抽取漫画关键物品"),
            Self::ComicSynthesis => (self.key(), 5, "正在综合漫画章节节奏"),
            Self::PagePlanning => (self.key(), 6, "正在生成漫画分页规划"),
            Self::DrawingPrompt => (self.key(), 7, "正在生成整页漫画提示词"),
        }
    }
}

pub(in crate::manga) fn should_restore_manga_stage_checkpoint(
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
    stage_order: i32,
) -> bool {
    if let Some(start_stage) = start_from_stage {
        return stage_order < start_stage.status().1;
    }
    resume_from_checkpoint
}
