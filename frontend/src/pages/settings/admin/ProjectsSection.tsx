import React, { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { listAllProjects, type AdminProject } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const ProjectsSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const [projects, setProjects] = useState<AdminProject[]>([]);
  const activeCallerUserIdRef = useRef(callerUserId);
  const mountedRef = useRef(true);
  const loadSeqRef = useRef(0);

  activeCallerUserIdRef.current = callerUserId;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const isCurrentLoad = useCallback((requestCallerUserId: number, seq: number) => {
    return (
      mountedRef.current &&
      activeCallerUserIdRef.current === requestCallerUserId &&
      loadSeqRef.current === seq
    );
  }, []);

  const loadProjects = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const nextProjects = await listAllProjects(requestCallerUserId);
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      setProjects(nextProjects);
    } catch (err) {
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadProjects, "加载失败");

  useEffect(() => {
    setProjects([]);
  }, [callerUserId]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-serif text-book-text-main">全部项目（{projects.length}）</h4>
        <BookIconButton label="刷新" onClick={load} variant="secondary" size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      {loading && projects.length === 0 ? (
        <div className="inline-flex items-center justify-center gap-1.5 py-4 text-xs text-book-text-muted pyqt-card">
          <Loader2 size={12} className="animate-spin" />
          正在加载项目…
        </div>
      ) : projects.length === 0 ? (
        <div className="text-xs text-book-text-muted py-4 text-center pyqt-card">
          暂无项目
        </div>
      ) : (
        <ul className="pyqt-card divide-y divide-book-border/30">
          {projects.map((p) => (
            <li key={p.id} className="p-2.5 flex items-center gap-2 text-xs">
              <span className="font-mono text-book-text-muted shrink-0 w-10">#{p.id}</span>
              <span className="font-semibold text-book-text-main truncate flex-1">{p.title}</span>
              <span className="pyqt-pill !text-[10px] shrink-0">{p.project_type}</span>
              <span className="pyqt-pill !text-[10px] shrink-0">{p.status}</span>
              <span className="text-book-text-muted shrink-0">
                {p.current_chapter} / {p.total_chapters ?? p.target_chapters}
              </span>
              <span className="text-book-text-muted shrink-0">user #{p.user_id}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
