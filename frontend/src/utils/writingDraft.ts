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
export const getWritingDraftProjectPrefix = (projectId: string) =>
  `${WRITING_DRAFT_PREFIX}${projectId}:`;
export const getWritingDraftKey = (projectId: string, chapterNumber: number) =>
  `${getWritingDraftProjectPrefix(projectId)}${chapterNumber}`;

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

/** 获取指定项目中存在草稿的章节编号集合 */
export const getProjectWritingDraftSet = (
  projectId: string,
): ReadonlySet<number> => {
  const normalizedProjectId = String(projectId || "").trim();
  if (!normalizedProjectId) return new Set<number>();

  const chapterNumbers = new Set<number>();
  for (const key of listPortableKeys(WRITING_DRAFT_PREFIX)) {
    const parsed = parseWritingDraftKey(key);
    if (!parsed || parsed.projectId !== normalizedProjectId) continue;
    chapterNumbers.add(parsed.chapterNumber);
  }

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
export const writeWritingDraft = async (key: string, content: string): Promise<boolean> => {
  const payload: LocalDraft = { content, updatedAt: new Date().toISOString() };
  const saved = await writePortableItem(key, JSON.stringify(payload));
  return saved;
};

/** 删除写作草稿 */
export const removeWritingDraft = async (key: string): Promise<void> => {
  await removePortableItem(key);
};
