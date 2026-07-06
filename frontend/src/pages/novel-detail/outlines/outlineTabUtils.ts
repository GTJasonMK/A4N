import type { ChapterOutline, PartOutline } from "../../../api/outline";
import { nonEmptyText } from "../../../utils/text";

export type NumberInputValue = number | "";

export const parseNumberInput = (value: string): NumberInputValue => {
  const text = value.trim();
  return text === "" ? "" : Number(text);
};

export const isPositiveInteger = (value: number | null): value is number =>
  value !== null && Number.isInteger(value) && value > 0;

export const DELETE_LATEST_PARTS_ACTION = "delete-latest-parts";
export const DELETE_LATEST_CHAPTER_OUTLINES_ACTION =
  "delete-latest-chapter-outlines";

export const deleteChapterOutlinesFromAction = (outlineId: number): string =>
  `delete-chapter-outlines-from-${outlineId}`;

export const partHeading = (part: PartOutline): string => {
  const title = nonEmptyText(part.title);
  return title ? `第 ${part.part_number} 部：${title}` : `第 ${part.part_number} 部`;
};

export const chapterOutlineHeading = (outline: ChapterOutline): string => {
  const title = nonEmptyText(outline.title);
  return title
    ? `第 ${outline.chapter_number} 章：${title}`
    : `第 ${outline.chapter_number} 章`;
};
