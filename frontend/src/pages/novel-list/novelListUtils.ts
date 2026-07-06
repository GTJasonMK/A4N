import type { NovelProject, ProjectAccessRole, ProjectKind } from "../../api/project";

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

export const PROJECT_KIND_LABELS: Record<ProjectKind, string> = {
  text: "文本项目",
  comic: "漫画项目",
  video: "视频项目",
  galgame: "GALGame 项目",
};

export const PROJECT_KIND_FILTER_LABELS: Record<ProjectKindFilter, string> = {
  all: "全部类型",
  ...PROJECT_KIND_LABELS,
};

export const PROJECT_ROLE_LABELS: Record<ProjectAccessRole, string> = {
  owner: "所有者",
  manager: "管理员",
  editor: "编辑者",
  viewer: "只读成员",
};

export const pickRandomQuote = (): Quote =>
  CREATIVE_QUOTES[Math.floor(Math.random() * CREATIVE_QUOTES.length)];

const titleSortKey = (title: string): string => {
  if (!title) return "#";
  const first = title[0].toUpperCase();
  if (/[A-Z]/.test(first)) return first;
  if (/[0-9]/.test(first)) return "#";
  return first;
};

const projectUpdatedAtMs = (project: NovelProject): number => {
  if (project.updated_at == null) return 0;
  const timestamp = new Date(project.updated_at).getTime();
  return Number.isNaN(timestamp) ? 0 : timestamp;
};

export const getRecentProjects = (projects: NovelProject[]): NovelProject[] =>
  [...projects].sort((a, b) => projectUpdatedAtMs(b) - projectUpdatedAtMs(a)).slice(0, 10);

export const groupProjectsByTitle = (projects: NovelProject[]): [string, NovelProject[]][] => {
  const groups = new Map<string, NovelProject[]>();
  [...projects]
    .sort((a, b) => a.title.localeCompare(b.title, "zh-Hans-u-co-pinyin"))
    .forEach((project) => {
      const key = titleSortKey(project.title);
      const group = groups.get(key);
      if (group) {
        group.push(project);
      } else {
        groups.set(key, [project]);
      }
    });
  return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
};
