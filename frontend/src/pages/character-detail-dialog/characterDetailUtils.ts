export type CharacterDetailTab = "arc" | "voice" | "conflict" | "manage";

export interface CharacterDetailInitialEventFilter {
  category?: string | null;
  key?: string | null;
}

import type { MessageTranslator } from "../../i18n";

export const syncedChapterText = (
  chapterNumber: number | null | undefined,
  t: MessageTranslator,
): string => {
  if (chapterNumber == null || chapterNumber <= 0) return t("characterDetail.syncedNotYet");
  return t("characterDetail.syncedToChapter", { chapter: chapterNumber });
};
