export type CharactersSubTab = "profile" | "portrait";

export const firstProfileName = (profiles: Array<{ name: string }>): string => {
  const first = profiles[0];
  return first == null ? "" : first.name;
};
