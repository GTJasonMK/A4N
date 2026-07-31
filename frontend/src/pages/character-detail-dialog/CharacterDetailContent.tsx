import React from "react";
import { AlertTriangle } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import type { CharacterProfile } from "../../api/character";
import { ArcView, ConflictView, VoiceView } from "./views";
import { ManageView } from "./ManageView";
import type {
  CharacterDetailInitialEventFilter,
  CharacterDetailTab,
} from "./characterDetailUtils";

interface CharacterDetailContentProps {
  tab: CharacterDetailTab;
  loadingTab: CharacterDetailTab | null;
  arc: unknown;
  voice: unknown;
  conflicts: unknown[] | null;
  character: CharacterProfile;
  initialManageSection?: "event";
  initialEventFilter?: CharacterDetailInitialEventFilter | null;
  onChanged?: () => void | Promise<void>;
  onRefresh: () => void | Promise<void>;
}

export const CharacterDetailContent: React.FC<CharacterDetailContentProps> = ({
  tab,
  loadingTab,
  arc,
  voice,
  conflicts,
  character,
  initialManageSection,
  initialEventFilter,
  onChanged,
  onRefresh,
}) => {
  if (loadingTab === tab) {
    return (
      <div className="py-10 flex flex-col items-center gap-2 text-book-text-muted">
        <LoadingSpinner size="lg" />
        <span className="text-xs">加载中…</span>
      </div>
    );
  }

  if (tab === "arc") {
    return <ArcView data={arc} />;
  }

  if (tab === "voice") {
    return <VoiceView data={voice} />;
  }

  if (tab === "conflict") {
    return conflicts == null ? (
      <div className="py-8 flex flex-col items-center gap-3 text-book-text-muted">
        <AlertTriangle size={18} />
        <div className="text-xs">尚未检测一致性</div>
        <BookButton variant="secondary" onClick={() => void onRefresh()}>
          开始检测
        </BookButton>
      </div>
    ) : (
      <ConflictView conflicts={conflicts} />
    );
  }

  return (
    <ManageView
      character={character}
      initialSection={initialManageSection}
      initialEventFilter={initialEventFilter}
      onChanged={onChanged}
    />
  );
};
