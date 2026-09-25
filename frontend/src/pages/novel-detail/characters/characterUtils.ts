import type { MessageTranslator } from "../../../i18n";

export type CharactersSubTab = "profile" | "portrait";

export const firstProfileName = (profiles: Array<{ name: string }>): string => {
  const first = profiles[0];
  return first == null ? "" : first.name;
};

export function portraitTypeLabel(type: string, t: MessageTranslator): string {
  switch (type) {
    case "full_body": return t("characterDetail.portrait.typeFullBody");
    case "half_body": return t("characterDetail.portrait.typeHalfBody");
    case "face": return t("characterDetail.portrait.typeFace");
    case "chibi": return t("characterDetail.portrait.typeChibi");
    default: return type;
  }
}
