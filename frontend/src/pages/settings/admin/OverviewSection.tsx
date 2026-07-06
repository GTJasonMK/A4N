import React, { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";
import { getAdminOverview, healthCheck } from "../../../api/admin";
import { BookIconButton } from "../../../components/ui/BookButton";
import { JsonCard } from "./JsonCard";
import { useAdminSectionLoader } from "./useAdminSectionLoader";

export const OverviewSection: React.FC<{ callerUserId: number }> = ({ callerUserId }) => {
  const [overview, setOverview] = useState<unknown>(null);
  const [health, setHealth] = useState<unknown>(null);
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

  const loadOverview = useCallback(async () => {
    const requestCallerUserId = callerUserId;
    const requestSeq = loadSeqRef.current + 1;
    loadSeqRef.current = requestSeq;
    try {
      const [o, h] = await Promise.all([
        getAdminOverview(requestCallerUserId),
        healthCheck(requestCallerUserId),
      ]);
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      setOverview(o);
      setHealth(h);
    } catch (err) {
      if (!isCurrentLoad(requestCallerUserId, requestSeq)) return;
      throw err;
    }
  }, [callerUserId, isCurrentLoad]);
  const { loading, load } = useAdminSectionLoader(loadOverview, "加载失败");

  useEffect(() => {
    setOverview(null);
    setHealth(null);
  }, [callerUserId]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-end">
        <BookIconButton label="刷新" onClick={load} variant="secondary" size="sm" disabled={loading}>
          <RefreshCw size={14} className={loading ? "animate-spin" : undefined} />
        </BookIconButton>
      </div>
      <JsonCard title="后台概览" data={overview} loading={loading} />
      <JsonCard title="系统健康" data={health} loading={loading} />
    </div>
  );
};
