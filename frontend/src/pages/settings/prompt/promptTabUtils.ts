import type { PromptEntry } from "../../../api/prompt";

interface PromptFilters {
  activeCategory: string;
  keyword: string;
}

const searchableText = (value: string | null | undefined): string =>
  value == null ? "" : value.toLowerCase();

export const filterPrompts = (
  prompts: PromptEntry[],
  filters: PromptFilters,
): PromptEntry[] => {
  const keyword = filters.keyword.trim().toLowerCase();
  return prompts.filter((prompt) => {
    if (filters.activeCategory !== "all" && prompt.category !== filters.activeCategory) {
      return false;
    }
    if (!keyword) return true;
    return (
      prompt.name.toLowerCase().includes(keyword) ||
      searchableText(prompt.title).includes(keyword) ||
      searchableText(prompt.description).includes(keyword)
    );
  });
};
