import React from "react";
import type { MessageKey } from "../../i18n";
import {
  Activity,
  Bot,
  Brain,
  CircleUserRound,
  FileText,
  Film,
  Globe2,
  Image as ImageIcon,
  Palette,
  ScrollText,
  Shield,
  Sparkles,
  Wrench,
} from "lucide-react";

export type TabId =
  | "llm"
  | "agent"
  | "embedding"
  | "image"
  | "video"
  | "web-search"
  | "queue"
  | "prompt"
  | "theme"
  | "advanced"
  | "admin"
  | "logs"
  | "about";

interface TabDef {
  id: TabId;
  labelKey: MessageKey;
  icon: React.ReactNode;
  groupKey: "settings.local" | "settings.cloud" | "settings.diagnostics";
}

export const TABS: TabDef[] = [
  { id: "llm", labelKey: "settings.llm", icon: <Bot size={14} />, groupKey: "settings.local" },
  { id: "web-search", labelKey: "settings.webSearch", icon: <Globe2 size={14} />, groupKey: "settings.local" },
  { id: "agent", labelKey: "settings.agent", icon: <Sparkles size={14} />, groupKey: "settings.local" },
  { id: "embedding", labelKey: "settings.embedding", icon: <Brain size={14} />, groupKey: "settings.local" },
  { id: "image", labelKey: "settings.image", icon: <ImageIcon size={14} />, groupKey: "settings.local" },
  { id: "video", labelKey: "settings.video", icon: <Film size={14} />, groupKey: "settings.local" },
  { id: "prompt", labelKey: "settings.prompt", icon: <FileText size={14} />, groupKey: "settings.local" },
  { id: "theme", labelKey: "settings.theme", icon: <Palette size={14} />, groupKey: "settings.local" },
  { id: "advanced", labelKey: "settings.advanced", icon: <Wrench size={14} />, groupKey: "settings.local" },
  { id: "admin", labelKey: "settings.admin", icon: <Shield size={14} />, groupKey: "settings.cloud" },
  { id: "queue", labelKey: "settings.queue", icon: <Activity size={14} />, groupKey: "settings.diagnostics" },
  { id: "logs", labelKey: "settings.logs", icon: <ScrollText size={14} />, groupKey: "settings.diagnostics" },
  { id: "about", labelKey: "settings.account", icon: <CircleUserRound size={14} />, groupKey: "settings.local" },
];

export const TAB_IDS = TABS.map((tab) => tab.id) as readonly TabId[];

export const resolveVisibleSettingsTab = (
  requestedTab: string,
  lastTab: TabId,
  visibleTabIds: readonly TabId[],
): TabId => {
  const requested = visibleTabIds.find((id) => id === requestedTab);
  if (requested) return requested;

  const previous = visibleTabIds.find((id) => id === lastTab);
  if (previous) return previous;

  const firstVisible = visibleTabIds[0];
  if (!firstVisible) throw new Error("Settings must expose at least one tab");
  return firstVisible;
};
