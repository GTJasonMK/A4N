use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::import_reverse::ReverseContextKind;
use a4n_domain::project::ProjectStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseEngineeringTarget {
    Blueprint,
    Outline,
    PartOutline,
}

impl ReverseEngineeringTarget {
    pub fn from_prompt_name(value: &str) -> Option<Self> {
        match value {
            "reverse_blueprint" => Some(Self::Blueprint),
            "reverse_outline" => Some(Self::Outline),
            "reverse_part_outline" => Some(Self::PartOutline),
            _ => None,
        }
    }

    pub(crate) fn context_kind(self) -> ReverseContextKind {
        match self {
            Self::Blueprint => ReverseContextKind::Blueprint,
            Self::Outline => ReverseContextKind::Outline,
            Self::PartOutline => ReverseContextKind::PartOutline,
        }
    }

    pub fn prompt_name(self) -> &'static str {
        match self {
            Self::Blueprint => "reverse_blueprint",
            Self::Outline => "reverse_outline",
            Self::PartOutline => "reverse_part_outline",
        }
    }

    pub fn user_instruction(self) -> &'static str {
        match self {
            Self::Blueprint => "请从以下内容反推蓝图（世界观/角色/剧情大纲/主题，输出 JSON）：",
            Self::Outline => "请从以下章节内容反推章节大纲（输出 JSON）：",
            Self::PartOutline => "请从以下内容反推分部大纲结构（输出 JSON）：",
        }
    }

    pub fn max_tokens_profile(self) -> &'static str {
        match self {
            Self::Blueprint => llm_token_profiles::BLUEPRINT,
            Self::Outline | Self::PartOutline => llm_token_profiles::OUTLINE,
        }
    }

    pub fn preview_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求预览",
            Self::Outline => "反推章节大纲 LLM 请求预览",
            Self::PartOutline => "反推分部大纲 LLM 请求预览",
        }
    }

    pub fn preview_step_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求",
            Self::Outline => "反推章节大纲 LLM 请求",
            Self::PartOutline => "反推分部大纲 LLM 请求",
        }
    }

    pub fn initial_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在准备蓝图归纳",
            Self::Outline => "正在准备章节大纲候选",
            Self::PartOutline => "正在准备分部大纲候选",
        }
    }

    pub fn success_message(self) -> &'static str {
        match self {
            Self::Blueprint => "蓝图已归纳并写入正式资料",
            Self::Outline => "章节大纲候选已生成",
            Self::PartOutline => "分部大纲候选已生成",
        }
    }

    pub fn cancelled_message(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图已停止",
            Self::Outline => "反推章节大纲已停止",
            Self::PartOutline => "反推分部大纲已停止",
        }
    }

    pub(crate) fn preparing_context_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在准备反推蓝图上下文",
            Self::Outline => "正在准备反推章节大纲上下文",
            Self::PartOutline => "正在准备反推分部大纲上下文",
        }
    }

    pub(crate) fn calling_model_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在调用模型反推蓝图",
            Self::Outline => "正在调用模型反推章节大纲",
            Self::PartOutline => "正在调用模型反推分部大纲",
        }
    }

    pub(crate) fn organizing_message(self) -> &'static str {
        match self {
            Self::Blueprint => "模型已返回，正在整理反推蓝图",
            Self::Outline => "模型已返回，正在整理章节大纲",
            Self::PartOutline => "模型已返回，正在整理分部大纲",
        }
    }

    pub(crate) fn ready_message(self) -> &'static str {
        match self {
            Self::Blueprint => "蓝图已归纳并写入正式资料",
            Self::Outline => "章节大纲候选已生成，等待用户确认",
            Self::PartOutline => "分部大纲候选已生成，等待用户确认",
        }
    }

    pub(crate) fn applied_status(self) -> ProjectStatus {
        match self {
            Self::Blueprint => ProjectStatus::BlueprintReady,
            Self::Outline => ProjectStatus::ChapterOutlinesReady,
            Self::PartOutline => ProjectStatus::PartOutlinesReady,
        }
    }
}
