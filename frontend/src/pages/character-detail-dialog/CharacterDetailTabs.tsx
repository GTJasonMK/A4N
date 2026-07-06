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
}

export const CharacterDetailTabs: React.FC<CharacterDetailTabsProps> = ({
  activeTab,
  loadingTab,
  conflictCount,
  onTabChange,
  onRefresh,
}) => {
  return (
    <div className="flex items-center gap-1 border-b border-book-border/40 shrink-0">
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
        一致性（{conflictCount ?? "?"}）
      </TabBtn>
      <TabBtn
        active={activeTab === "manage"}
        onClick={() => onTabChange("manage")}
        icon={<Wrench size={12} />}
      >
        管理
      </TabBtn>
      <div className="flex-1" />
      {activeTab !== "manage" && (
        <button
          type="button"
          onClick={() => void onRefresh()}
          disabled={loadingTab != null}
          className="p-1.5 text-book-text-muted hover:text-book-text-main"
          title="刷新当前 Tab"
        >
          <RefreshCw size={12} className={loadingTab != null ? "animate-spin" : ""} />
        </button>
      )}
    </div>
  );
};
