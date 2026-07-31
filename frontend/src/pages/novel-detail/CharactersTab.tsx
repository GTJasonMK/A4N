import React, { useCallback, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { CharacterProfilesPanel } from "./characters/CharacterProfilesPanel";
import { CharacterPortraitsPanel } from "./characters/CharacterPortraitsPanel";
import type { CharactersSubTab } from "./characters/characterUtils";
import { useChapterAnalysisLoader } from "./useNovelDetailLoader";

export const CharactersTab: React.FC<{ projectId: number }> = ({ projectId }) => {
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [subTab, setSubTab] = useState<CharactersSubTab>("profile");
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);

  const loadProfiles = useCallback(
    async (isCurrent: () => boolean) => {
      try {
        const nextProfiles = await listProfiles(projectId);
        if (!isCurrent()) return;
        setProfiles(nextProfiles);
        setLoadedProjectId(projectId);
      } catch (err) {
        if (isCurrent()) {
          setProfiles([]);
          setLoadedProjectId(projectId);
        }
        throw err;
      }
    },
    [projectId],
  );
  const { loading, load } = useChapterAnalysisLoader(
    projectId,
    loadProfiles,
    "加载角色失败",
  );

  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  if (displayLoading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <LoadingSpinner size="sm" /> 加载角色…
      </div>
    );
  }

  return (
    <div className="content-main flex min-h-0 flex-col gap-3">
      <div className="flex items-center">
        <SegmentedControl
          ariaLabel="人物工作区"
          value={subTab}
          onChange={setSubTab}
          options={[
            { id: "profile" as const, label: "档案" },
            { id: "portrait" as const, label: "立绘" },
          ]}
        />
      </div>

      {subTab === "profile" ? (
        <CharacterProfilesPanel
          projectId={projectId}
          profiles={displayProfiles}
          onChanged={() => load({ silent: true, force: true })}
        />
      ) : (
        <CharacterPortraitsPanel projectId={projectId} />
      )}
    </div>
  );
};
