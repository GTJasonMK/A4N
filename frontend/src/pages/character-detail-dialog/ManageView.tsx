import React, { useEffect, useState } from "react";
import { type CharacterProfile } from "../../api/character";
import { DeletionManager } from "./DeletionManager";
import { BehaviorManager } from "./BehaviorManager";
import { EventManager } from "./EventManager";
import { ImplicitReviewManager } from "./ImplicitReviewManager";
import { SnapshotManager } from "./SnapshotManager";

type ManageSection = "behavior" | "implicit" | "event" | "snapshot" | "deletion";

const MANAGE_SECTIONS: { id: ManageSection; label: string; shortLabel: string }[] = [
  { id: "behavior", label: "行为记录", shortLabel: "行为" },
  { id: "implicit", label: "隐性复审", shortLabel: "复审" },
  { id: "event", label: "属性事件", shortLabel: "属性" },
  { id: "snapshot", label: "角色快照", shortLabel: "快照" },
  { id: "deletion", label: "删除标记", shortLabel: "删除" },
];

export const ManageView: React.FC<{
  character: CharacterProfile;
  initialSection?: ManageSection;
  initialEventFilter?: { category?: string | null; key?: string | null } | null;
  onChanged?: () => void | Promise<void>;
}> = ({ character, initialSection, initialEventFilter, onChanged }) => {
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
          <button
            key={s.id}
            type="button"
            onClick={() => setSec(s.id)}
            data-active={sec === s.id ? "1" : undefined}
            className="pyqt-tab shrink-0 !text-caption"
            title={s.label}
            aria-label={s.label}
          >
            {s.shortLabel}
          </button>
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
