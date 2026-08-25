import React from "react";
import { AlertTriangle, Brain, RefreshCw, Sparkles, Wrench } from "lucide-react";
import { TabBtn } from "./views";
import type { CharacterDetailTab } from "./characterDetailUtils";
import { useI18n } from "../../i18n";

interface CharacterDetailTabsProps {
  activeTab: CharacterDetailTab;
  loadingTab: CharacterDetailTab | null;
  conflictCount: number | null;
  onTabChange: (tab: CharacterDetailTab) => void;
  onRefresh: () => void | Promise<void>;
  actions?: React.ReactNode;
}

export const CharacterDetailTabs: React.FC<CharacterDetailTabsProps> = ({
  activeTab,
  loadingTab,
  conflictCount,
  onTabChange,
  onRefresh,
  actions,
}) => {
  const { t } = useI18n();
  return (
    <div className="flex min-w-0 shrink-0 items-center gap-1 border-b border-book-border/40">
      <TabBtn active={activeTab === "arc"} onClick={() => onTabChange("arc")} icon={<Brain size={12} />}>
        {t("characterDetail.arc")}
      </TabBtn>
      <TabBtn
        active={activeTab === "voice"}
        onClick={() => onTabChange("voice")}
        icon={<Sparkles size={12} />}
      >
        {t("characterDetail.voice")}
      </TabBtn>
      <TabBtn
        active={activeTab === "conflict"}
        onClick={() => onTabChange("conflict")}
        icon={<AlertTriangle size={12} />}
      >
        {conflictCount == null ? t("characterDetail.consistency") : t("characterDetail.consistencyCount", { count: conflictCount })}
      </TabBtn>
      <TabBtn
        active={activeTab === "manage"}
        onClick={() => onTabChange("manage")}
        icon={<Wrench size={12} />}
      >
        {t("characterDetail.manage")}
      </TabBtn>
      <div className="flex-1" />
      {actions}
      {activeTab !== "manage" && (
        <button
          type="button"
          onClick={() => void onRefresh()}
          disabled={loadingTab != null}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title={t("characterDetail.refreshCurrent")}
          aria-label={t("characterDetail.refreshCurrent")}
        >
          <RefreshCw size={12} className={loadingTab != null ? "animate-spin" : ""} />
        </button>
      )}
    </div>
  );
};
