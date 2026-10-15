import { useCallback, useEffect, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import {
  authorizePageSession,
  controlPageSession,
  createPageSession,
  heartbeatPageSession,
  subscribePageEvents,
  updatePageContext,
  type PageContext,
  type PageSessionEvent,
  type PageSessionView,
} from "../api/pageSessions";
import { extractErrorMessage } from "../api";
import { useAuthStore } from "../store/auth";
import { shouldUseHttpBackend } from "../utils/runtimeMode";
import { isPositiveSafeInteger } from "../utils/numberRange";
import { useI18n } from "../i18n";
import { useDirtyRegistry, useDirtyRegistryVersion } from "../components/navigation/dirtyRegistry";
import { isStandaloneWindow } from "../utils/standaloneWindow";

const PAGE_CLIENT = "a4n-web";
const TAB_ID_KEY = "a4n:page-session-tab-id";
const HEARTBEAT_MS = 12_000;

export type PageControlAction = "pause" | "resume" | "cancel" | "takeover" | "detach";

export interface PageContextRouteInput {
  pathname: string;
  search?: string;
  hash?: string;
}

export function pageKindForPath(pathname: string): string {
  if (pathname === "/") return "novel-list";
  if (/^\/inspiration\/\d+(?:\/|$)/.test(pathname)) return "inspiration-chat";
  if (/^\/novel\/\d+(?:\/|$)/.test(pathname)) return "novel-detail";
  if (/^\/write\/\d+(?:\/|$)/.test(pathname)) return "writing-desk";
  if (/^\/canvas-project\/\d+(?:\/|$)/.test(pathname)) return "canvas-project";
  if (/^\/video-project\/\d+(?:\/|$)/.test(pathname)) return "video-project";
  if (/^\/comic-project\/\d+(?:\/|$)/.test(pathname)) return "comic-project";
  if (/^\/galgame-project\/\d+(?:\/|$)/.test(pathname)) return "galgame-project";
  if (/^\/settings(?:\/|$)/.test(pathname)) return "settings";
  if (/^\/agents(?:\/|$)/.test(pathname)) return "agent-monitor";
  return "page";
}

function positiveId(value: string | null): number | null {
  if (!value || !/^\d+$/.test(value)) return null;
  const id = Number(value);
  return isPositiveSafeInteger(id) ? id : null;
}

function projectIdForPath(pathname: string): number | null {
  const match = pathname.match(
    /^\/(?:inspiration|novel|write|video-project|canvas-project|comic-project|galgame-project)\/(\d+)(?:\/|$)/,
  );
  return match ? positiveId(match[1]) : null;
}

export function buildPageContext(
  input: PageContextRouteInput,
  contextRevision: number,
  previous?: Pick<PageContext, "selected_ids" | "dirty_state" | "local_state">,
): PageContext {
  const search = new URLSearchParams(input.search ?? "");
  const hash = (input.hash ?? "").replace(/^#/, "");
  return {
    page_kind: pageKindForPath(input.pathname),
    route: `${input.pathname}${input.search ?? ""}${input.hash ?? ""}`,
    project_id: projectIdForPath(input.pathname),
    chapter_id: positiveId(search.get("chapter") ?? search.get("chapter_id")),
    active_tab: hash || null,
    selected_ids: previous?.selected_ids ?? {},
    dirty_state: previous?.dirty_state ?? false,
    local_state: previous?.local_state ?? {},
    context_revision: contextRevision,
  };
}

export function pageSessionWithCurrentContext(
  session: PageSessionView,
  current: PageContext,
): PageSessionView {
  if (session.context_revision >= current.context_revision) return session;
  return {
    ...session,
    page_kind: current.page_kind,
    route: current.route,
    project_id: current.project_id,
    chapter_id: current.chapter_id,
    active_tab: current.active_tab,
    selected_ids: current.selected_ids,
    dirty_state: current.dirty_state,
    local_state: current.local_state,
    context_revision: current.context_revision,
  };
}

function tabId(): string {
  if (typeof window === "undefined") return "server-rendered";
  try {
    const existing = window.sessionStorage.getItem(TAB_ID_KEY);
    if (existing) return existing;
    const value = crypto.randomUUID();
    window.sessionStorage.setItem(TAB_ID_KEY, value);
    return value;
  } catch {
    return crypto.randomUUID();
  }
}

function shouldLockPage(session: PageSessionView | null): boolean {
  return (
    session?.agent_id != null &&
    (session.status === "attached" || session.status === "running" || session.status === "paused")
  );
}

export interface PageSessionState {
  session: PageSessionView | null;
  authorizationCode: string | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  locked: boolean;
  control: (action: PageControlAction) => Promise<void>;
  authorize: () => Promise<void>;
  restart: () => Promise<void>;
}

export function usePageSession(): PageSessionState {
  const { user } = useAuthStore();
  const dirtyRegistry = useDirtyRegistry();
  const dirtyVersion = useDirtyRegistryVersion();
  const { t } = useI18n();
  const translatorRef = useRef(t);
  translatorRef.current = t;
  const location = useLocation();
  const navigate = useNavigate();
  const standaloneRoute = /^\/(?:agents|admin\/test-center)(?:\/|$)/.test(location.pathname);
  const enabled =
    !isStandaloneWindow() &&
    !standaloneRoute &&
    user != null &&
    user.page_cli_enabled &&
    shouldUseHttpBackend();
  const [session, setSession] = useState<PageSessionView | null>(null);
  const [authorizationCode, setAuthorizationCode] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restartKey, setRestartKey] = useState(0);
  const contextRevisionRef = useRef(0);
  const contextRef = useRef<PageContext>(
    buildPageContext({ pathname: location.pathname, search: location.search, hash: location.hash }, 0),
  );
  const currentRouteRef = useRef(`${location.pathname}${location.search}${location.hash}`);
  currentRouteRef.current = `${location.pathname}${location.search}${location.hash}`;
  const sessionIdRef = useRef<string | null>(null);
  const tokenRef = useRef<string | null>(null);
  const createSequenceRef = useRef(0);
  const tabIdRef = useRef<string>();

  const applySession = useCallback((next: PageSessionView) => {
    const currentContext = contextRef.current;
    const staleContext = next.context_revision < currentContext.context_revision;
    const applied = pageSessionWithCurrentContext(next, currentContext);
    setSession(applied);
    contextRevisionRef.current = Math.max(contextRevisionRef.current, next.context_revision, currentContext.context_revision);
    if (!staleContext) {
      contextRef.current = {
        ...currentContext,
        selected_ids: next.selected_ids,
        dirty_state: next.dirty_state,
        local_state: next.local_state,
        context_revision: Math.max(currentContext.context_revision, next.context_revision),
      };
    }
    if (next.status === "attached" || next.agent_id != null) setAuthorizationCode(null);
  }, []);

  const detachLocal = useCallback(() => {
    sessionIdRef.current = null;
    tokenRef.current = null;
    setSession(null);
    setAuthorizationCode(null);
    setBusy(false);
  }, []);

  const authorize = useCallback(async () => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    if (!sessionId || !token) return;
    setBusy(true);
    try {
      const result = await authorizePageSession(sessionId, token);
      if (sessionIdRef.current !== sessionId || tokenRef.current !== token) return;
      setAuthorizationCode(result.authorization_code);
      setError(null);
    } catch (err) {
      if (sessionIdRef.current === sessionId && tokenRef.current === token) {
        setError(extractErrorMessage(err, translatorRef.current("pageSession.authorizationFailed")));
      }
      throw err;
    } finally {
      if (sessionIdRef.current === sessionId && tokenRef.current === token) setBusy(false);
    }
  }, []);

  const control = useCallback(async (action: PageControlAction) => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    if (!sessionId || !token) return;
    setBusy(true);
    try {
      const next = await controlPageSession(sessionId, token, action);
      if (sessionIdRef.current !== sessionId || tokenRef.current !== token) return;
      if (action === "detach") {
        detachLocal();
      } else {
        applySession(next);
        if (action === "takeover") setAuthorizationCode(null);
      }
      setError(null);
    } catch (err) {
      if (sessionIdRef.current === sessionId && tokenRef.current === token) {
        setError(extractErrorMessage(err, translatorRef.current("pageSession.controlFailed")));
      }
      throw err;
    } finally {
      if (sessionIdRef.current === sessionId && tokenRef.current === token) setBusy(false);
    }
  }, [applySession, detachLocal]);

  const restart = useCallback(async () => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    try {
      if (sessionId && token) {
        await controlPageSession(sessionId, token, "detach");
      }
    } catch {
      // The old session may already be offline; creating a fresh session is the recovery path.
    }
    if (sessionIdRef.current !== sessionId || tokenRef.current !== token) return;
    detachLocal();
    setError(null);
    setRestartKey((value) => value + 1);
  }, [detachLocal]);

  useEffect(() => {
    if (!enabled) {
      if (sessionIdRef.current && tokenRef.current) {
        void controlPageSession(sessionIdRef.current, tokenRef.current, "detach").catch(() => undefined);
      }
      detachLocal();
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;
    const sequence = createSequenceRef.current + 1;
    createSequenceRef.current = sequence;
    tabIdRef.current ??= tabId();
    setLoading(true);
    setError(null);
    void (async () => {
      try {
        const created = await createPageSession(PAGE_CLIENT, tabIdRef.current!);
        if (cancelled || createSequenceRef.current !== sequence) return;
        sessionIdRef.current = created.session.ui_session_id;
        tokenRef.current = created.ui_session_token;
        contextRevisionRef.current = created.session.context_revision;
        applySession(created.session);
        if (created.session.status === "awaiting_attach" || created.session.status === "idle") {
          await authorizePageSession(created.session.ui_session_id, created.ui_session_token)
            .then((result) => {
              if (cancelled || createSequenceRef.current !== sequence) return;
              setAuthorizationCode(result.authorization_code);
            });
        }
      } catch (err) {
        if (!cancelled && createSequenceRef.current === sequence) {
          setError(extractErrorMessage(err, translatorRef.current("pageSession.createFailed")));
        }
      } finally {
        if (!cancelled && createSequenceRef.current === sequence) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [applySession, detachLocal, enabled, restartKey]);

  useEffect(() => {
    if (!enabled || !sessionIdRef.current || !tokenRef.current) return;
    const nextRevision = Math.max(
      contextRevisionRef.current,
      session?.context_revision ?? 0,
    ) + 1;
    contextRevisionRef.current = nextRevision;
    contextRef.current = buildPageContext(
      { pathname: location.pathname, search: location.search, hash: location.hash },
      nextRevision,
      { ...contextRef.current, dirty_state: dirtyRegistry.hasDirty() },
    );
  }, [dirtyRegistry, dirtyVersion, enabled, location.hash, location.pathname, location.search, session?.ui_session_id]);

  useEffect(() => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    if (!enabled || !sessionId || !token) return;
    const context = { ...contextRef.current };
    let cancelled = false;
    void updatePageContext(sessionId, token, context)
      .then((next) => {
        if (!cancelled && sessionIdRef.current === sessionId && tokenRef.current === token) applySession(next);
      })
      .catch((err) => {
        if (!cancelled && sessionIdRef.current === sessionId && tokenRef.current === token
          && context.context_revision >= contextRef.current.context_revision) {
          setError(extractErrorMessage(err, translatorRef.current("pageSession.contextSyncFailed")));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [
    applySession,
    enabled,
    location.hash,
    location.pathname,
    location.search,
    dirtyVersion,
    session?.ui_session_id,
  ]);

  useEffect(() => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    if (!enabled || !sessionId || !token) return;
    let requestInFlight = false;
    const timer = window.setInterval(() => {
      if (requestInFlight) return;
      requestInFlight = true;
      const context = { ...contextRef.current };
      void heartbeatPageSession(sessionId, token, context)
        .then((next) => {
          if (sessionIdRef.current === sessionId && tokenRef.current === token) applySession(next);
        })
        .catch((err) => {
          if (sessionIdRef.current === sessionId && tokenRef.current === token
            && context.context_revision >= contextRef.current.context_revision) {
            setError(extractErrorMessage(err, translatorRef.current("pageSession.heartbeatFailed")));
          }
        })
        .finally(() => { requestInFlight = false; });
    }, HEARTBEAT_MS);
    return () => window.clearInterval(timer);
  }, [applySession, enabled, session?.ui_session_id]);

  useEffect(() => {
    const sessionId = sessionIdRef.current;
    const token = tokenRef.current;
    if (!enabled || !sessionId || !token) return;
    const controller = new AbortController();
    void subscribePageEvents(
      token,
      (event: PageSessionEvent) => {
        if (event.session.ui_session_id !== sessionIdRef.current) return;
        applySession(event.session);
        if (event.kind === "navigate" && event.route && event.route !== currentRouteRef.current) {
          navigate(event.route);
        }
      },
      controller.signal,
    ).catch((err) => {
      if (!controller.signal.aborted) setError(extractErrorMessage(err, translatorRef.current("pageSession.eventStreamDisconnected")));
    });
    return () => controller.abort();
  }, [applySession, enabled, navigate, session?.ui_session_id]);

  const locked = shouldLockPage(session);
  return { session, authorizationCode, loading, busy, error, locked, control, authorize, restart };
}
