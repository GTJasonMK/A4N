import { extractErrorMessage } from "../../api";
import type { NovelProject, ProjectKind } from "../../api/project";
import { isCancelledOperation } from "../../utils/cancellation";
import {
  PROJECT_ACCESS_ROLE_LABELS,
  PROJECT_KIND_LABELS,
} from "../../utils/projectRouting";

export { PROJECT_KIND_LABELS };

export interface Quote {
  zh: string;
  en: string;
}

const CREATIVE_QUOTES: Quote[] = [
  { zh: "落笔成章，灵感无疆", en: "Let words flow, let imagination soar" },
  { zh: "一念成世界，字里藏山河", en: "A thought becomes a world" },
  { zh: "以墨为舟，载梦远航", en: "Sail your dreams with ink" },
  { zh: "每个故事，都值得被讲述", en: "Every story deserves to be told" },
  { zh: "笔墨之间，万千世界", en: "Infinite worlds between the lines" },
  { zh: "让灵感落地，让想象生长", en: "Ground your inspiration, grow your vision" },
  { zh: "文字有灵，故事不朽", en: "Words have soul, stories live forever" },
  { zh: "文章千古事，得失寸心知", en: "— 杜甫《偶题》" },
  { zh: "文章本天成，妙手偶得之", en: "— 陆游《文章》" },
  { zh: "看似寻常最奇崛，成如容易却艰辛", en: "— 王安石" },
  { zh: "山重水复疑无路，柳暗花明又一村", en: "— 陆游《游山西村》" },
  { zh: "落红不是无情物，化作春泥更护花", en: "— 龚自珍《己亥杂诗》" },
  {
    zh: "心中若有未讲的故事，便是最大的痛苦",
    en: "There is no greater agony than bearing an untold story — Maya Angelou",
  },
  {
    zh: "初稿不过是你讲给自己听的故事",
    en: "The first draft is just you telling yourself the story — Terry Pratchett",
  },
  {
    zh: "一个词接着一个词，便是力量",
    en: "A word after a word after a word is power — Margaret Atwood",
  },
  { zh: "想象力比知识更重要", en: "Imagination is more important than knowledge — Einstein" },
  { zh: "世界对于有想象力的人来说，只是一块画布", en: "The world is but a canvas to imagination — Thoreau" },
];

export type ProjectTab = "recent" | "all";
export type ProjectKindFilter = "all" | ProjectKind;

export interface ProjectFamily {
  key: string;
  root: NovelProject | null;
  projects: NovelProject[];
  title: string;
  updatedAtMs: number;
}

export const PROJECT_TABS: readonly ProjectTab[] = ["recent", "all"] as const;
export const PROJECT_KIND_FILTERS: readonly ProjectKindFilter[] = [
  "all",
  "text",
  "comic",
  "video",
  "galgame",
] as const;

export const PROJECT_TAB_LABELS: Record<ProjectTab, string> = {
  recent: "最近项目",
  all: "全部项目",
};

export const PROJECT_KIND_FILTER_LABELS: Record<ProjectKindFilter, string> = {
  all: "全部类型",
  ...PROJECT_KIND_LABELS,
};

export const PROJECT_ROLE_LABELS = PROJECT_ACCESS_ROLE_LABELS;

export const pickRandomQuote = (): Quote =>
  CREATIVE_QUOTES[Math.floor(Math.random() * CREATIVE_QUOTES.length)];

const titleSortKey = (title: string): string => {
  const first = title[0]!.toUpperCase();
  if (/[A-Z]/.test(first)) return first;
  if (/[0-9]/.test(first)) return "#";
  return first;
};

const projectUpdatedAtMs = (project: NovelProject): number =>
  new Date(project.updated_at).getTime();

const PROJECT_KIND_ORDER: Record<ProjectKind, number> = {
  text: 0,
  comic: 1,
  video: 2,
  galgame: 3,
};

const compareFamilyProjects = (a: NovelProject, b: NovelProject): number => {
  const kindDifference = PROJECT_KIND_ORDER[a.project_kind] - PROJECT_KIND_ORDER[b.project_kind];
  if (kindDifference !== 0) return kindDifference;
  const updatedDifference = projectUpdatedAtMs(b) - projectUpdatedAtMs(a);
  if (updatedDifference !== 0) return updatedDifference;
  return a.title.localeCompare(b.title, "zh-Hans-u-co-pinyin");
};

const buildProjectFamilies = (projects: NovelProject[]): ProjectFamily[] => {
  const familyProjects = new Map<string, NovelProject[]>();
  projects.forEach((project) => {
    const familyId = project.root_project_id ?? project.id;
    const key = String(familyId);
    const family = familyProjects.get(key);
    if (family) family.push(project);
    else familyProjects.set(key, [project]);
  });

  return [...familyProjects.entries()].map(([key, items]) => {
    const familyId = Number(key);
    const root = items.find((project) => project.id === familyId) ?? null;
    const sortedProjects = [...items].sort((a, b) => {
      if (root) {
        if (a.id === root.id) return -1;
        if (b.id === root.id) return 1;
      }
      return compareFamilyProjects(a, b);
    });
    return {
      key,
      root,
      projects: sortedProjects,
      title:
        root?.title ??
        items.find((project) => project.root_project_title?.trim())?.root_project_title?.trim() ??
        sortedProjects[0]!.title,
      updatedAtMs: Math.max(...items.map(projectUpdatedAtMs)),
    };
  });
};

export const getRecentProjectFamilies = (projects: NovelProject[]): ProjectFamily[] =>
  buildProjectFamilies(projects)
    .sort((a, b) => b.updatedAtMs - a.updatedAtMs)
    .slice(0, 10);

export const groupProjectFamiliesByTitle = (
  projects: NovelProject[],
): [string, ProjectFamily[]][] => {
  const groups = new Map<string, ProjectFamily[]>();
  buildProjectFamilies(projects)
    .sort((a, b) => a.title.localeCompare(b.title, "zh-Hans-u-co-pinyin"))
    .forEach((family) => {
      const key = titleSortKey(family.title);
      const group = groups.get(key);
      if (group) {
        group.push(family);
      } else {
        groups.set(key, [family]);
      }
    });
  return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
};

export const trySyncTextProjectMaterials = async (
  project: Pick<NovelProject, "id" | "project_kind">,
  sync: (projectId: number) => Promise<void>,
): Promise<string | null> => {
  if (project.project_kind !== "text") return null;
  try {
    await sync(project.id);
    return null;
  } catch (err) {
    const message = extractErrorMessage(err, "资料索引同步失败");
    return isCancelledOperation(message)
      ? "资料索引同步已停止，可在项目操作中重新同步"
      : `资料索引待恢复：${message}；可在项目操作中重新同步`;
  }
};
