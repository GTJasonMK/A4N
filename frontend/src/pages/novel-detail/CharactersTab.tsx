import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  canListenAppEvents,
  listenAppEvent,
  type UnlistenFn,
} from "../../api/runtime";
import { listProfiles, type CharacterProfile } from "../../api/character";
import { useToast } from "../../components/feedback/Toast";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { CharacterProfilesPanel } from "./characters/CharacterProfilesPanel";
import { CharacterPortraitsPanel } from "./characters/CharacterPortraitsPanel";
import type { CharactersSubTab } from "./characters/characterUtils";
import type { ChapterAnalysisUpdatedPayload } from "./shared";

interface CharacterProfilesLoadOptions {
  silent?: boolean;
  force?: boolean;
}

export const CharactersTab: React.FC<{ projectId: number }> = ({ projectId }) => {
  const { addToast } = useToast();
  const [profiles, setProfiles] = useState<CharacterProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [subTab, setSubTab] = useState<CharactersSubTab>("profile");
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const activeProjectIdRef = useRef(projectId);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    projectId: number;
    token: symbol;
    promise: Promise<void>;
  } | null>(null);

  activeProjectIdRef.current = projectId;

  const isCurrentLoad = useCallback((requestProjectId: number, seq: number) => {
    return activeProjectIdRef.current === requestProjectId && loadSeqRef.current === seq;
  }, []);

  const load = useCallback(async (options: CharacterProfilesLoadOptions = {}) => {
    const requestProjectId = projectId;
    const silent = options.silent ?? false;
    if (!options.force && loadInFlightRef.current?.projectId === requestProjectId) {
      return loadInFlightRef.current.promise;
    }
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    const requestToken = Symbol(String(requestProjectId));
    const loadPromise = (async () => {
      if (!silent) setLoading(true);
      try {
        const nextProfiles = await listProfiles(requestProjectId);
        if (!isCurrentLoad(requestProjectId, requestSeq)) return;
        setProfiles(nextProfiles);
      } catch (err) {
        if (!isCurrentLoad(requestProjectId, requestSeq)) return;
        setProfiles([]);
        addToast(extractErrorMessage(err, "加载角色失败"), "error");
      } finally {
        if (isCurrentLoad(requestProjectId, requestSeq)) {
          setLoadedProjectId(requestProjectId);
          if (!silent) setLoading(false);
        }
        if (
          loadInFlightRef.current?.projectId === requestProjectId &&
          loadInFlightRef.current.token === requestToken
        ) {
          loadInFlightRef.current = null;
        }
      }
    })();
    loadInFlightRef.current = {
      projectId: requestProjectId,
      token: requestToken,
      promise: loadPromise,
    };
    return loadPromise;
  }, [addToast, isCurrentLoad, projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!canListenAppEvents()) return;

    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    void listenAppEvent<ChapterAnalysisUpdatedPayload>("chapter-analysis-updated", (event) => {
      if (Number(event.payload?.project_id) === projectId) {
        void load({ silent: true, force: true });
      }
    }).then((detach) => {
      if (disposed) {
        detach();
      } else {
        unlisten = detach;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load, projectId]);

  useEffect(() => {
    const refreshOnFocus = () => {
      if (document.visibilityState === "visible") void load({ silent: true });
    };
    window.addEventListener("focus", refreshOnFocus);
    document.addEventListener("visibilitychange", refreshOnFocus);
    return () => {
      window.removeEventListener("focus", refreshOnFocus);
      document.removeEventListener("visibilitychange", refreshOnFocus);
    };
  }, [load]);

  const displayProfiles = loadedProjectId === projectId ? profiles : [];
  const displayLoading = loading || loadedProjectId !== projectId;

  if (displayLoading) {
    return (
      <div className="flex items-center gap-2 text-xs text-book-text-muted">
        <Loader2 size={12} className="animate-spin" /> 加载角色…
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
