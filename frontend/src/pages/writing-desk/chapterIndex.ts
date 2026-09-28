import type { ChapterIndexEntry, ChapterSummary } from "../../api/chapter";
import { countChapterBodyCharacters } from "./wordCount";

export function chapterIndexEntry(chapter: ChapterSummary): ChapterIndexEntry {
  const { content, edit_revision, style_tags, analysis_data, ...metadata } = chapter;
  return { ...metadata, body_character_count: countChapterBodyCharacters(content) };
}

export type ReloadChapters = (options?: { force?: boolean; chapterId?: number }) => Promise<{
  index: ChapterIndexEntry[];
  chapter: ChapterSummary | null;
} | null>;
