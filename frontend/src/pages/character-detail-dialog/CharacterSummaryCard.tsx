import React from "react";
import type { CharacterProfile } from "../../api/character";
import { syncedChapterText } from "./characterDetailUtils";

interface CharacterSummaryCardProps {
  character: CharacterProfile;
}

export const CharacterSummaryCard: React.FC<CharacterSummaryCardProps> = ({ character }) => {
  return (
    <div className="pyqt-card p-3 shrink-0">
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <span className="text-sm font-semibold text-book-text-main">{character.name}</span>
        {character.gender && <span className="pyqt-pill !text-[10px]">{character.gender}</span>}
        {character.age && <span className="pyqt-pill !text-[10px]">{character.age}</span>}
        <span className="pyqt-pill !text-[10px]">
          {syncedChapterText(character.last_synced_chapter)}
        </span>
      </div>
      {character.personality && (
        <div className="text-xs text-book-text-sub mb-1">
          <span className="text-book-text-muted">性格：</span>
          {character.personality}
        </div>
      )}
      {character.background && (
        <div className="text-xs text-book-text-sub line-clamp-2">
          <span className="text-book-text-muted">背景：</span>
          {character.background}
        </div>
      )}
    </div>
  );
};
