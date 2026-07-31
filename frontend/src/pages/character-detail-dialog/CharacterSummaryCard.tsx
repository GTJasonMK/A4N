import React from "react";
import type { CharacterProfile } from "../../api/character";
import { syncedChapterText } from "./characterDetailUtils";

interface CharacterSummaryCardProps {
  character: CharacterProfile;
}

export const CharacterSummaryCard: React.FC<CharacterSummaryCardProps> = ({ character }) => {
  const metadata = [
    character.gender?.trim(),
    character.age?.trim(),
    syncedChapterText(character.last_synced_chapter),
  ].filter((item): item is string => Boolean(item));

  return (
    <div className="shrink-0 border-b border-book-border/45 pb-3">
      <div className="mb-2 text-small text-book-text-muted">{metadata.join(" · ")}</div>
      {character.personality && (
        <div className="mb-1 text-xs leading-relaxed text-book-text-sub">
          <span className="text-book-text-muted">性格：</span>
          {character.personality}
        </div>
      )}
      {character.background && (
        <div className="line-clamp-2 text-xs leading-relaxed text-book-text-sub">
          <span className="text-book-text-muted">背景：</span>
          {character.background}
        </div>
      )}
    </div>
  );
};
