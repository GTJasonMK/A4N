import {
  listPortableKeys,
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "./portableStorage";
import { getActiveLocale, translateMessage } from "../i18n";

type LocalDraft = {
  content: string;
  updatedAt: string;
};

const WRITING_DRAFT_PREFIX = "a4n:writing_draft:";
const projectDraftIndex = new Map<string, Set<number>>();

export const getWritingDraftKey = (projectId: string, chapterNumber: number) =>
  `${WRITING_DRAFT_PREFIX}${projectId}:${chapterNumber}`;

const parseWritingDraftKey = (
  key: string,
): { projectId: string; chapterNumber: number } | null => {
  const raw = String(key || "");
  if (!raw.startsWith(WRITING_DRAFT_PREFIX)) return null;
  const rest = raw.slice(WRITING_DRAFT_PREFIX.length);
  const lastColon = rest.lastIndexOf(":");
  if (lastColon <= 0) return null;

  const projectId = rest.slice(0, lastColon).trim();
  const chapterNumber = Number(rest.slice(lastColon + 1));
  if (!projectId || !Number.isFinite(chapterNumber) || chapterNumber <= 0)
    return null;
  return { projectId, chapterNumber };
};

const updateProjectDraftIndex = (key: string, present: boolean) => {
  const parsed = parseWritingDraftKey(key);
  if (!parsed) return;

  const existing = projectDraftIndex.get(parsed.projectId);
  if (!existing) return;

  if (present) {
    existing.add(parsed.chapterNumber);
    return;
  }

  existing.delete(parsed.chapterNumber);
};

/** 获取指定项目中存在草稿的章节编号集合 */
export const getProjectWritingDraftSet = (
  projectId: string,
): ReadonlySet<number> => {
  const normalizedProjectId = String(projectId || "").trim();
  if (!normalizedProjectId) return new Set<number>();

  const cached = projectDraftIndex.get(normalizedProjectId);
  if (cached) return cached;

  const chapterNumbers = new Set<number>();
  for (const key of listPortableKeys(WRITING_DRAFT_PREFIX)) {
    const parsed = parseWritingDraftKey(key);
    if (!parsed || parsed.projectId !== normalizedProjectId) continue;
    chapterNumbers.add(parsed.chapterNumber);
  }

  projectDraftIndex.set(normalizedProjectId, chapterNumbers);
  return chapterNumbers;
};

/** 读取写作草稿 */
export const readWritingDraft = (key: string): LocalDraft | null => {
  const raw = readPortableItem(key);
  if (!raw) return null;
  const parsed = JSON.parse(raw) as Partial<LocalDraft>;
  if (
    !parsed ||
    typeof parsed.content !== "string" ||
    typeof parsed.updatedAt !== "string"
  ) {
    throw new Error(translateMessage(getActiveLocale(), "errors.writingDraftInvalid"));
  }
  return {
    content: parsed.content,
    updatedAt: parsed.updatedAt,
  };
};

/** 保存写作草稿到便携数据目录 */
export const writeWritingDraft = (key: string, content: string): boolean => {
  const payload: LocalDraft = { content, updatedAt: new Date().toISOString() };
  const saved = writePortableItem(key, JSON.stringify(payload));
  updateProjectDraftIndex(key, true);
  return saved;
};

/** 删除写作草稿 */
export const removeWritingDraft = (key: string): void => {
  removePortableItem(key);
  updateProjectDraftIndex(key, false);
};
