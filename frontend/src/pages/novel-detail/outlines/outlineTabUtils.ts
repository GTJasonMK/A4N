import type { ChapterOutlineListEntry, PartOutline } from "../../../api/outline";
import type { MessageTranslator } from "../../../i18n";
import { isPositiveSafeInteger } from "../../../utils/numberRange";
import { isPositiveI32 } from "../../../utils/numeric";
import { nonEmptyText } from "../../../utils/text";

export type NumberInputValue = number | "";
export const MAX_CHAPTER_OUTLINE_GENERATION_COUNT = 50;

export const isValidChapterOutlineCount = (
  value: NumberInputValue,
): value is number =>
  isPositiveSafeInteger(value) &&
  value >= 1 &&
  value <= MAX_CHAPTER_OUTLINE_GENERATION_COUNT;

export const isValidDeletionCount = (
  value: NumberInputValue,
  available: number,
): value is number =>
  isPositiveSafeInteger(value) &&
  value >= 1 &&
  value <= available;

export const parseNumberInput = (value: string): NumberInputValue => {
  const text = value.trim();
  return text === "" ? "" : Number(text);
};

export const isPositiveInteger = (value: number | null): value is number =>
  value !== null && isPositiveI32(value);

export const partGenerationInputError = (
  partCount: number | null,
  chaptersPerPart: number | null,
): "partCount" | "chaptersPerPart" | null => {
  if (!isPositiveInteger(partCount)) return "partCount";
  if (chaptersPerPart !== null && !isPositiveInteger(chaptersPerPart)) {
    return "chaptersPerPart";
  }
  return null;
};

export const outlineOptimizationVariables = (
  direction: string,
): Record<string, unknown> | null => {
  const trimmed = direction.trim();
  return trimmed ? { optimization_direction: trimmed } : null;
};

export const DELETE_LATEST_PARTS_ACTION = "delete-latest-parts";
export const DELETE_LATEST_CHAPTER_OUTLINES_ACTION =
  "delete-latest-chapter-outlines";

export const deleteChapterOutlinesFromAction = (outlineId: number): string =>
  `delete-chapter-outlines-from-${outlineId}`;

export const partHeading = (part: PartOutline, t: MessageTranslator): string => {
  const title = nonEmptyText(part.title);
  const heading = t("outline.partSection.part", { part: part.part_number });
  return title ? `${heading}：${title}` : heading;
};

export const chapterOutlineHeading = (
  outline: ChapterOutlineListEntry,
  t: MessageTranslator,
): string => {
  const title = nonEmptyText(outline.title);
  const heading = t("outline.chapterSection.chapter", {
    chapter: outline.chapter_number,
  });
  return title ? `${heading}：${title}` : heading;
};
