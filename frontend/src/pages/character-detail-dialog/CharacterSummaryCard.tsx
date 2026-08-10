import React from "react";
import type { CharacterProfile } from "../../api/character";
import { characterAttributeText } from "../../utils/characterAttributes";
import { syncedChapterText } from "./characterDetailUtils";

interface CharacterSummaryCardProps {
  character: CharacterProfile;
}

export const CharacterSummaryCard: React.FC<CharacterSummaryCardProps> = ({ character }) => {
  const gender = characterAttributeText(character, "explicit", "gender");
  const age = characterAttributeText(character, "explicit", "age");
  const personality = characterAttributeText(character, "implicit", "personality");
  const background = characterAttributeText(character, "explicit", "background");
  const metadata = [
    gender,
    age,
    syncedChapterText(character.last_synced_chapter),
  ].filter((item): item is string => Boolean(item));

  return (
    <div className="shrink-0 border-b border-book-border/45 pb-3">
      <div className="mb-2 text-small text-book-text-muted">{metadata.join(" · ")}</div>
      {personality && (
        <div className="mb-1 text-xs leading-relaxed text-book-text-sub">
          <span className="text-book-text-muted">性格：</span>
          {personality}
        </div>
      )}
      {background && (
        <div className="line-clamp-2 text-xs leading-relaxed text-book-text-sub">
          <span className="text-book-text-muted">背景：</span>
          {background}
        </div>
      )}
    </div>
  );
};
