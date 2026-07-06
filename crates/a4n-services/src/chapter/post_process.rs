//! 章节内容后处理
//!
//! 提供风格标签提取、词数统计和质量评分功能。

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use chrono::Utc;
use regex::Regex;
use sea_orm::*;
use std::sync::Arc;

/// 章节后处理结果
#[derive(Debug, Clone)]
pub struct PostProcessResult {
    /// 风格标签
    pub style_tags: Vec<String>,
    /// 字符数
    pub char_count: usize,
    /// 中文字数（去除非中文）
    pub chinese_word_count: usize,
    /// 段落数
    pub paragraph_count: usize,
    /// 对话比例
    pub dialogue_ratio: f64,
    /// 质量评分（1-100）
    pub quality_score: f64,
}

/// 章节后处理器
pub struct ChapterPostProcessor {
    db: Arc<DatabaseConnection>,
}

impl ChapterPostProcessor {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 对章节执行完整后处理流程
    pub async fn process_chapter(&self, chapter_id: i64) -> Result<PostProcessResult, AppError> {
        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("章节不存在".into()))?;

        let content = &ch.content;
        let result = analyze_content(content);

        // 更新章节的风格标签
        self.persist_tags(chapter_id, &result.style_tags).await?;

        Ok(result)
    }

    async fn persist_tags(&self, chapter_id: i64, tags: &[String]) -> Result<(), AppError> {
        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("章节不存在".into()))?;

        let mut active: chapter::ActiveModel = ch.into();
        active.style_tags = Set(Some(serde_json::json!(tags)));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}

/// 分析文本内容，提取风格标签和统计信息
fn analyze_content(content: &str) -> PostProcessResult {
    let char_count = content.chars().count();

    // 中文字数统计（仅统计中文字符）
    let chinese_re = Regex::new(r"[\u{4e00}-\u{9fff}\u{3400}-\u{4dbf}]").unwrap();
    let chinese_word_count = chinese_re.find_iter(content).count();

    // 段落数
    let paragraph_count = content
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .count();

    // 对话比例（引号内容占比）
    let dialogue_re = Regex::new(r#"[""][^""""]+[""']|「[^」]+」|『[^』]+』"#).unwrap();
    let dialogue_chars: usize = dialogue_re
        .find_iter(content)
        .map(|m| m.as_str().chars().count())
        .sum();
    let dialogue_ratio = if char_count > 0 {
        dialogue_chars as f64 / char_count as f64
    } else {
        0.0
    };

    // 风格标签提取
    let mut style_tags = Vec::new();

    // 对话密集度
    if dialogue_ratio > 0.3 {
        style_tags.push("对话密集".to_string());
    } else if dialogue_ratio < 0.05 {
        style_tags.push("叙事为主".to_string());
    }

    // 段落风格
    let avg_para_len = if paragraph_count > 0 {
        chinese_word_count as f64 / paragraph_count as f64
    } else {
        0.0
    };
    if avg_para_len > 500.0 {
        style_tags.push("长段叙事".to_string());
    } else if avg_para_len < 80.0 && paragraph_count > 3 {
        style_tags.push("碎片化".to_string());
    }

    // 质量评分（基于多项指标）
    let mut quality_score: f64 = 70.0;

    // 段落长度合理性（30-300字为佳）
    if (30.0..=300.0).contains(&avg_para_len) {
        quality_score += 10.0;
    }

    // 对话比例合理性（0.1-0.5为佳）
    if (0.1..=0.5).contains(&dialogue_ratio) {
        quality_score += 10.0;
    }

    // 长度惩罚（过短或过长）
    if chinese_word_count < 500 {
        quality_score -= 20.0;
    } else if chinese_word_count > 20000 {
        quality_score -= 5.0;
    }

    quality_score = quality_score.clamp(0.0, 100.0);

    PostProcessResult {
        style_tags,
        char_count,
        chinese_word_count,
        paragraph_count,
        dialogue_ratio,
        quality_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_chinese_content() {
        let content = "这是第一章的内容。\n\n包含多个段落。\n\n「这是一段对话」主角说道。";
        let result = analyze_content(content);
        assert!(result.chinese_word_count > 0);
        assert_eq!(result.paragraph_count, 3);
        assert!(result.dialogue_ratio > 0.0);
    }

    #[test]
    fn test_quality_score_range() {
        let content = "测试内容，用于验证质量评分范围。".repeat(50);
        let result = analyze_content(&content);
        assert!((0.0..=100.0).contains(&result.quality_score));
    }
}
