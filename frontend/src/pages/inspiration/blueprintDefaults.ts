import type { NovelProject } from "../../api/project";

const DEFAULT_BLUEPRINT_CHAPTER_COUNT = 100;
const PROJECT_TYPE_GENRE_LABELS: Record<string, string> = {
  novel_long: "长篇小说",
  novel_short: "短篇故事",
  script: "剧本",
  prose: "散文",
};

const validChapterCount = (value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isInteger(value)) return null;
  if (value < 5 || value > 10000) return null;
  return value;
};

export const defaultChapterCountForProject = (project: NovelProject): number =>
  validChapterCount(project.target_chapters) ?? DEFAULT_BLUEPRINT_CHAPTER_COUNT;

export const defaultGenreForProject = (project: NovelProject): string => {
  const projectType = project.project_type.trim();
  return PROJECT_TYPE_GENRE_LABELS[projectType] ?? (projectType || "小说");
};

export const buildFallbackTranscript = (project: NovelProject): string => {
  const rows = [`项目标题: ${project.title}`];
  const description = project.description?.trim();
  if (description) rows.push(`项目简介: ${description}`);
  rows.push("备注: 灵感对话信息不足，请基于项目信息自行补足缺失设定，生成一版可继续编辑的草稿蓝图。");
  return rows.join("\n\n");
};
