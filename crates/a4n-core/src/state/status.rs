//! 项目状态枚举与持久化状态值。
//!
//! # 合法转换路径
//!
//! ```text
//! Draft → BlueprintReady → PartOutlinesReady → ChapterOutlinesReady → Writing → Completed
//! Draft → BlueprintReady → ChapterOutlinesReady → Writing → Completed  (短篇跳过)
//! ```

use serde::{Deserialize, Serialize};

/// 数据库持久化的项目状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    BlueprintReady,
    PartOutlinesReady,
    ChapterOutlinesReady,
    Writing,
    Completed,
}

impl ProjectStatus {
    /// 所有可能的状态列表
    pub fn all() -> &'static [ProjectStatus] {
        &[
            Self::Draft,
            Self::BlueprintReady,
            Self::PartOutlinesReady,
            Self::ChapterOutlinesReady,
            Self::Writing,
            Self::Completed,
        ]
    }

    /// 数据库和前端使用的规范化状态值。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::BlueprintReady => "blueprint_ready",
            Self::PartOutlinesReady => "part_outlines_ready",
            Self::ChapterOutlinesReady => "chapter_outlines_ready",
            Self::Writing => "writing",
            Self::Completed => "completed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|status| status.as_str() == value)
    }

    /// 状态推进顺序，用于命令层判断自动推进是否需要跳过。
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Draft => 0,
            Self::BlueprintReady => 1,
            Self::PartOutlinesReady => 2,
            Self::ChapterOutlinesReady => 3,
            Self::Writing => 4,
            Self::Completed => 5,
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Draft => "草稿",
            Self::BlueprintReady => "蓝图就绪",
            Self::PartOutlinesReady => "分部大纲就绪",
            Self::ChapterOutlinesReady => "章节大纲就绪",
            Self::Writing => "写作中",
            Self::Completed => "已完成",
        };
        write!(f, "{s}")
    }
}
