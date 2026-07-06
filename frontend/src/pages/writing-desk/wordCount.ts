export const countChapterBodyCharacters = (
  content: string | null | undefined,
): number => Array.from(content ?? "").length;
