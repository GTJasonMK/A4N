import { useCallback, useEffect, useRef, useState } from "react";
import { extractErrorMessage } from "../../api";
import type { ToastType } from "../../components/feedback/Toast";
import type { MessageTranslator } from "../../i18n";

interface ProjectProfile {
  id: number;
  project_id: number;
}

interface ProfileLoadOptions {
  silent?: boolean;
  force?: boolean;
  projectId?: number;
  notify?: boolean;
}

export function useProjectProfileCollection<T extends ProjectProfile>({
  projectId,
  canEditProfiles,
  entityLabel,
  t,
  listProfiles,
  addToast,
}: {
  projectId: number;
  canEditProfiles: boolean;
  entityLabel: string;
  t: MessageTranslator;
  listProfiles: (projectId: number) => Promise<T[]>;
  addToast: (message: string, type: ToastType) => void;
}) {
  const [profiles, setProfiles] = useState<T[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadedProjectId, setLoadedProjectId] = useState<number | null>(null);
  const [loadError, setLoadError] = useState<{ projectId: number; message: string } | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [detailFor, setDetailFor] = useState<T | null>(null);
  const [editingProfile, setEditingProfile] = useState<T | "new" | null>(null);
  const mountedRef = useRef(false);
  const activeProjectIdRef = useRef(projectId);
  const profilesRef = useRef<T[]>([]);
  const loadSeqRef = useRef(0);
  const loadInFlightRef = useRef<{
    projectId: number;
    token: symbol;
    promise: Promise<boolean>;
  } | null>(null);
  const actionSeqRef = useRef(0);
  const actionBusyRef = useRef(false);

  activeProjectIdRef.current = projectId;

  const isCurrentProject = useCallback(
    (requestProjectId: number) =>
      mountedRef.current && activeProjectIdRef.current === requestProjectId,
    [],
  );

  const isCurrentAction = useCallback(
    (requestProjectId: number, requestSeq: number) =>
      isCurrentProject(requestProjectId) && actionSeqRef.current === requestSeq,
    [isCurrentProject],
  );

  const commitProfile = useCallback((saved: T) => {
    if (!isCurrentProject(saved.project_id)) return;
    const current = profilesRef.current;
    const next = current.some((profile) => profile.id === saved.id)
      ? current.map((profile) => profile.id === saved.id ? saved : profile)
      : [saved, ...current];
    profilesRef.current = next;
    setProfiles(next);
  }, [isCurrentProject]);

  const removeProfile = useCallback((profileId: number) => {
    const next = profilesRef.current.filter((profile) => profile.id !== profileId);
    if (next.length === profilesRef.current.length) return;
    profilesRef.current = next;
    setProfiles(next);
  }, []);

  const beginAction = useCallback(
    (requestProjectId: number): number | null => {
      if (!isCurrentProject(requestProjectId)) return null;
      if (!canEditProfiles) {
        addToast(t("novel.profile.readonlyCannotModify", { name: entityLabel }), "error");
        return null;
      }
      if (actionBusyRef.current) return null;
      actionBusyRef.current = true;
      actionSeqRef.current += 1;
      return actionSeqRef.current;
    },
    [addToast, canEditProfiles, entityLabel, isCurrentProject, t],
  );

  const finishAction = useCallback((requestSeq: number) => {
    if (actionSeqRef.current === requestSeq) {
      actionBusyRef.current = false;
    }
  }, []);

  const load = useCallback(
    async (options: ProfileLoadOptions = {}): Promise<boolean> => {
      const requestProjectId = options.projectId ?? projectId;
      const silent = options.silent ?? false;
      const notify = options.notify ?? true;
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
          if (!isCurrentProject(requestProjectId) || loadSeqRef.current !== requestSeq) return false;
          if (nextProfiles.some((profile) => profile.project_id !== requestProjectId)) {
            throw new Error(t("novel.profile.foreignProjectData", { name: entityLabel }));
          }
          profilesRef.current = nextProfiles;
          setProfiles(nextProfiles);
          setLoadedProjectId(requestProjectId);
          setLoadError(null);
          return true;
        } catch (err) {
          if (isCurrentProject(requestProjectId) && loadSeqRef.current === requestSeq) {
            const message = extractErrorMessage(err, t("novel.profile.loadFailed", { name: entityLabel }));
            setLoadedProjectId(requestProjectId);
            setLoadError({ projectId: requestProjectId, message });
            if (notify) addToast(message, "error");
          }
          return false;
        } finally {
          if (!silent && isCurrentProject(requestProjectId) && loadSeqRef.current === requestSeq) {
            setLoading(false);
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
    },
    [addToast, entityLabel, isCurrentProject, listProfiles, projectId, t],
  );

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      loadSeqRef.current += 1;
      loadInFlightRef.current = null;
      actionSeqRef.current += 1;
      actionBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeProjectIdRef.current = projectId;
    loadSeqRef.current += 1;
    loadInFlightRef.current = null;
    actionSeqRef.current += 1;
    actionBusyRef.current = false;
    profilesRef.current = [];
    setProfiles([]);
    setLoadedProjectId(null);
    setLoadError(null);
    setDetailFor(null);
    setEditingProfile(null);
    setBusyId(null);
    setLoading(true);
    void load({ projectId, force: true });
  }, [load, projectId]);

  useEffect(() => {
    profilesRef.current = profiles;
    setDetailFor((current) => {
      if (!current) return null;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
    setEditingProfile((current) => {
      if (!current || current === "new") return current;
      return profiles.find((profile) => profile.id === current.id) ?? null;
    });
  }, [profiles]);

  useEffect(() => {
    if (canEditProfiles) return;
    setEditingProfile(null);
  }, [canEditProfiles]);

  const currentProject = loadedProjectId === projectId;

  return {
    profiles: currentProject ? profiles : [],
    profilesRef,
    loading: loading || !currentProject,
    loadError: loadError?.projectId === projectId ? loadError.message : null,
    busyId,
    setBusyId,
    detailFor,
    setDetailFor,
    editingProfile,
    setEditingProfile,
    load,
    isCurrentAction,
    beginAction,
    finishAction,
    commitProfile,
    removeProfile,
  };
}
