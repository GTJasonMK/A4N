import React from "react";
import { AlertTriangle, Brain, RefreshCw, Sparkles, Wrench } from "lucide-react";
import { TabBtn } from "./views";
import type { CharacterDetailTab } from "./characterDetailUtils";

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
  return (
    <div className="flex min-w-0 shrink-0 items-center gap-1 border-b border-book-border/40">
      <TabBtn active={activeTab === "arc"} onClick={() => onTabChange("arc")} icon={<Brain size={12} />}>
        弧光
      </TabBtn>
      <TabBtn
        active={activeTab === "voice"}
        onClick={() => onTabChange("voice")}
        icon={<Sparkles size={12} />}
      >
        声音指纹
      </TabBtn>
      <TabBtn
        active={activeTab === "conflict"}
        onClick={() => onTabChange("conflict")}
        icon={<AlertTriangle size={12} />}
      >
        {conflictCount == null ? "一致性" : `一致性 ${conflictCount}`}
      </TabBtn>
      <TabBtn
        active={activeTab === "manage"}
        onClick={() => onTabChange("manage")}
        icon={<Wrench size={12} />}
      >
        管理
      </TabBtn>
      <div className="flex-1" />
      {actions}
      {activeTab !== "manage" && (
        <button
          type="button"
          onClick={() => void onRefresh()}
          disabled={loadingTab != null}
          className="pyqt-btn-icon !h-9 !w-9 sm:!h-7 sm:!w-7"
          title="刷新当前页面"
          aria-label="刷新当前页面"
        >
          <RefreshCw size={12} className={loadingTab != null ? "animate-spin" : ""} />
        </button>
      )}
    </div>
  );
};
