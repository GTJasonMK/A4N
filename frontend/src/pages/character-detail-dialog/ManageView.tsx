import React, { useEffect, useState } from "react";
import { type CharacterProfile } from "../../api/character";
import { DeletionManager } from "./DeletionManager";
import { BehaviorManager } from "./BehaviorManager";
import { EventManager } from "./EventManager";
import { ImplicitReviewManager } from "./ImplicitReviewManager";
import { SnapshotManager } from "./SnapshotManager";
import { useI18n, type MessageKey } from "../../i18n";

type ManageSection = "behavior" | "implicit" | "event" | "snapshot" | "deletion";

const MANAGE_SECTIONS = [
  { id: "behavior", labelKey: "behavior", shortLabelKey: "behaviorShort" },
  { id: "implicit", labelKey: "implicit", shortLabelKey: "implicitShort" },
  { id: "event", labelKey: "event", shortLabelKey: "eventShort" },
  { id: "snapshot", labelKey: "snapshot", shortLabelKey: "snapshotShort" },
  { id: "deletion", labelKey: "deletion", shortLabelKey: "deletionShort" },
] as const satisfies ReadonlyArray<{
  id: ManageSection;
  labelKey: string;
  shortLabelKey: string;
}>;

export const ManageView: React.FC<{
  character: CharacterProfile;
  initialSection?: ManageSection;
  initialEventFilter?: { category?: string | null; key?: string | null } | null;
  onChanged?: () => void | Promise<void>;
}> = ({ character, initialSection, initialEventFilter, onChanged }) => {
  const { t } = useI18n();
  const [sec, setSec] = useState<ManageSection>(initialSection ?? "behavior");
  const defaultChapter =
    character.last_synced_chapter > 0 ? character.last_synced_chapter : 1;
  useEffect(() => {
    if (initialSection) setSec(initialSection);
  }, [character.id, initialSection, initialEventFilter?.category, initialEventFilter?.key]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex min-w-0 shrink-0 items-center gap-1 border-b border-book-border/40">
        {MANAGE_SECTIONS.map((s) => (
          (() => {
            const label = t(`characterDetail.sections.${s.labelKey}` as MessageKey);
            const shortLabel = t(`characterDetail.sections.${s.shortLabelKey}` as MessageKey);
            return (
          <button
            key={s.id}
            type="button"
            onClick={() => setSec(s.id)}
            data-active={sec === s.id ? "1" : undefined}
            className="pyqt-tab shrink-0 !text-caption"
            title={label}
            aria-label={label}
          >
            {shortLabel}
          </button>
            );
          })()
        ))}
      </div>
      <div>
        {sec === "behavior" && <BehaviorManager profileId={character.id} />}
        {sec === "implicit" && (
          <ImplicitReviewManager
            profileId={character.id}
            defaultChapter={defaultChapter}
            onChanged={onChanged}
          />
        )}
        {sec === "event" && (
          <EventManager
            profileId={character.id}
            defaultChapter={defaultChapter}
            initialCategory={initialEventFilter?.category ?? undefined}
            initialKey={initialEventFilter?.key ?? undefined}
            onChanged={onChanged}
          />
        )}
        {sec === "snapshot" && (
          <SnapshotManager profileId={character.id} onChanged={onChanged} />
        )}
        {sec === "deletion" && (
          <DeletionManager
            profileId={character.id}
            defaultChapter={defaultChapter}
            onChanged={onChanged}
          />
        )}
      </div>
    </div>
  );
};
