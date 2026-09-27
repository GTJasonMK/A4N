import React from "react";
import { AlertTriangle } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { BookButton } from "../../components/ui/BookButton";
import type { CharacterConflictSummary, CharacterProfile } from "../../api/character";
import { ArcView, ConflictView, VoiceView } from "./views";
import { ManageView } from "./ManageView";
import type {
  CharacterDetailInitialEventFilter,
  CharacterDetailTab,
} from "./characterDetailUtils";
import { useI18n } from "../../i18n";

interface CharacterDetailContentProps {
  tab: CharacterDetailTab;
  loadingTab: CharacterDetailTab | null;
  arc: unknown;
  voice: unknown;
  conflicts: CharacterConflictSummary[] | null;
  character: CharacterProfile;
  initialManageSection?: "event";
  initialEventFilter?: CharacterDetailInitialEventFilter | null;
  onChanged?: () => boolean | void | Promise<boolean | void>;
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
  const { t } = useI18n();
  if (loadingTab === tab) {
    return (
      <div className="py-10 flex flex-col items-center gap-2 text-book-text-muted">
        <LoadingSpinner size="lg" />
        <span className="text-xs">{t("characterDetail.loading")}</span>
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
        <div className="text-xs">{t("characterDetail.notChecked")}</div>
        <BookButton variant="secondary" onClick={() => void onRefresh()}>
          {t("characterDetail.startCheck")}
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
