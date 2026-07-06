export type CharacterDetailTab = "arc" | "voice" | "conflict" | "manage";

export interface CharacterDetailInitialEventFilter {
  category?: string | null;
  key?: string | null;
}

export const syncedChapterText = (chapterNumber: number | null | undefined): string => {
  if (chapterNumber == null || chapterNumber <= 0) return "尚未同步";
  return `已同步至第 ${chapterNumber} 章`;
};
