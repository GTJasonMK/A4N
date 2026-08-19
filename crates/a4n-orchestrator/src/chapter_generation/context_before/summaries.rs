use super::shared::{chapter_context_label, truncate_chars};
use crate::chapter_generation::non_empty_trimmed;
use a4n_core::models::chapter::chapter;

const RECENT_SUMMARY_CHAPTERS: i32 = 10;
const MAX_OLD_SUMMARY_LINES: usize = 80;
const OLD_SUMMARY_HEAD_LINES: usize = 20;

pub(super) fn layered_chapter_summary_lines(
    chapters: &[chapter::Model],
    current_chapter_number: i32,
) -> Vec<String> {
    if chapters.is_empty() {
        return Vec::new();
    }

    let recent_threshold = (current_chapter_number - RECENT_SUMMARY_CHAPTERS).max(1);
    let mut old_summaries = Vec::new();
    let mut recent_summaries = Vec::new();

    for chapter in chapters {
        let Some(summary) = chapter_summary_text(chapter) else {
            continue;
        };
        let label = chapter_context_label(chapter.chapter_number, chapter.title.as_deref());
        if chapter.chapter_number >= recent_threshold {
            recent_summaries.push(format!("- {}：{}", label, truncate_chars(summary, 280)));
        } else if let Some(summary) = brief_summary(summary, 30) {
            old_summaries.push(format!("- {}：{}", label, summary));
        }
    }

    let mut lines = Vec::new();
    let has_old_summaries = !old_summaries.is_empty();
    let has_recent_summaries = !recent_summaries.is_empty();
    if has_old_summaries {
        lines.push("【早期章节】".to_string());
        lines.extend(compact_old_summary_lines(old_summaries));
    }
    if has_old_summaries && has_recent_summaries {
        lines.push(String::new());
    }
    if has_recent_summaries {
        lines.push("【最近章节（详细）】".to_string());
        lines.extend(recent_summaries);
    }
    lines
}

fn compact_old_summary_lines(old_summaries: Vec<String>) -> Vec<String> {
    if old_summaries.len() <= MAX_OLD_SUMMARY_LINES {
        return old_summaries;
    }

    let head_count = OLD_SUMMARY_HEAD_LINES.min(MAX_OLD_SUMMARY_LINES);
    let tail_count = MAX_OLD_SUMMARY_LINES.saturating_sub(head_count);
    let omitted = old_summaries.len().saturating_sub(head_count + tail_count);

    let mut lines = Vec::with_capacity(MAX_OLD_SUMMARY_LINES + 1);
    lines.extend(old_summaries.iter().take(head_count).cloned());
    if omitted > 0 {
        lines.push(format!("- ……中间省略 {omitted} 章早期摘要"));
    }
    if tail_count > 0 {
        lines.extend(
            old_summaries
                .iter()
                .skip(old_summaries.len().saturating_sub(tail_count))
                .cloned(),
        );
    }
    lines
}

fn chapter_summary_text(chapter: &chapter::Model) -> Option<&str> {
    chapter.summary.as_deref().and_then(non_empty_trimmed)
}

fn brief_summary(summary: &str, max_chars: usize) -> Option<String> {
    let first_sentence = summary.split('。').next().and_then(non_empty_trimmed)?;
    Some(truncate_chars(first_sentence, max_chars))
}
