import React, { useEffect, useRef, useState } from "react";
import { History, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  promptVersionDiff,
  promptVersionHistory,
  type PromptEntry,
  type PromptVersion,
} from "../../../api/prompt";

interface PromptHistoryPanelProps {
  prompt: PromptEntry;
}

export const PromptHistoryPanel: React.FC<PromptHistoryPanelProps> = ({ prompt }) => {
  const { addToast } = useToast();
  const [open, setOpen] = useState(false);
  const [history, setHistory] = useState<PromptVersion[]>([]);
  const [loading, setLoading] = useState(false);
  const [diffPair, setDiffPair] = useState<string[]>([]);
  const [diffResult, setDiffResult] = useState<string | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const scopeRef = useRef({ name: prompt.name, token: 0 });
  const mountedRef = useRef(false);
  const diffSeqRef = useRef(0);
  const diffLoadingRef = useRef(false);

  const isCurrentScope = (target: { name: string; token: number }) =>
    mountedRef.current &&
    scopeRef.current.name === target.name &&
    scopeRef.current.token === target.token;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      diffSeqRef.current += 1;
      diffLoadingRef.current = false;
      scopeRef.current = {
        ...scopeRef.current,
        token: scopeRef.current.token + 1,
      };
    };
  }, []);

  const loadHistory = async () => {
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    setLoading(true);
    setHistory([]);
    setDiffPair([]);
    setDiffResult(null);
    diffLoadingRef.current = false;
    setDiffLoading(false);
    try {
      const data = await promptVersionHistory(targetScope.name);
      if (!isCurrentScope(targetScope)) return;
      setHistory(data);
    } catch (err) {
      if (isCurrentScope(targetScope)) {
        addToast(extractErrorMessage(err, "加载历史失败"), "error");
      }
    } finally {
      if (isCurrentScope(targetScope)) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    const token = scopeRef.current.token + 1;
    scopeRef.current = { name: prompt.name, token };
    diffSeqRef.current += 1;
    diffLoadingRef.current = false;
    setHistory([]);
    setDiffPair([]);
    setDiffResult(null);
    setDiffLoading(false);
    setLoading(false);
    if (open) void loadHistory();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, prompt.name]);

  const toggleDiffPick = (version: string) => {
    diffSeqRef.current += 1;
    diffLoadingRef.current = false;
    setDiffPair((prev) => {
      const has = prev.includes(version);
      if (has) return prev.filter((item) => item !== version);
      const next = [...prev, version];
      return next.length > 2 ? next.slice(-2) : next;
    });
    setDiffResult(null);
    setDiffLoading(false);
  };

  const handleDiff = async () => {
    if (diffLoadingRef.current) return;
    if (diffPair.length !== 2) return;
    const targetScope = { name: prompt.name, token: scopeRef.current.token };
    const requestPair = [...diffPair] as [string, string];
    const requestSeq = diffSeqRef.current + 1;
    diffSeqRef.current = requestSeq;
    diffLoadingRef.current = true;
    setDiffLoading(true);
    try {
      const text = await promptVersionDiff({
        name: targetScope.name,
        version_a: requestPair[0],
        version_b: requestPair[1],
      });
      if (!isCurrentScope(targetScope) || diffSeqRef.current !== requestSeq) return;
      setDiffResult(text);
    } catch (err) {
      if (isCurrentScope(targetScope) && diffSeqRef.current === requestSeq) {
        addToast(extractErrorMessage(err, "对比失败"), "error");
      }
    } finally {
      if (diffSeqRef.current === requestSeq) diffLoadingRef.current = false;
      if (isCurrentScope(targetScope) && diffSeqRef.current === requestSeq) {
        setDiffLoading(false);
      }
    }
  };

  return (
    <details
      open={open}
      onToggle={(event) => setOpen((event.target as HTMLDetailsElement).open)}
      className="pyqt-card p-2.5"
    >
      <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
        <History size={11} /> 版本历史（git2）
      </summary>
      <div className="mt-2">
        {loading ? (
          <div className="text-[11px] text-book-text-muted inline-flex items-center gap-1.5">
            <Loader2 size={10} className="animate-spin" /> 加载历史…
          </div>
        ) : history.length === 0 ? (
          <div className="text-[11px] text-book-text-muted italic">
            尚无历史记录（首次保存后会有）
          </div>
        ) : (
          <>
            <div className="flex items-center gap-2 mb-1.5 text-[10px]">
              <span className="text-book-text-muted">勾选 2 个版本对比</span>
              {diffPair.length === 2 && (
                <BookButton variant="secondary" onClick={handleDiff} disabled={diffLoading}>
                  {diffLoading ? (
                    <>
                      <Loader2 size={10} className="animate-spin" />
                      对比中…
                    </>
                  ) : (
                    <>对比 {diffPair[0].slice(0, 8)} ↔ {diffPair[1].slice(0, 8)}</>
                  )}
                </BookButton>
              )}
            </div>
            <ul className="space-y-0.5 max-h-44 overflow-y-auto custom-scrollbar">
              {history.map((version) => {
                const isPicked = diffPair.includes(version.version);
                return (
                  <li
                    key={version.version}
                    className={`flex items-center gap-2 px-2 py-1 rounded text-[11px] ${
                      isPicked ? "bg-book-primary/10 border border-book-primary/40" : ""
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={isPicked}
                      onChange={() => toggleDiffPick(version.version)}
                      className="accent-book-primary"
                    />
                    <span className="font-mono text-book-text-main shrink-0">
                      {version.version.slice(0, 8)}
                    </span>
                    <span className="text-book-text-muted shrink-0 text-[10px]">
                      {new Date(version.timestamp).toLocaleString()}
                    </span>
                    <span className="text-book-text-muted shrink-0">{version.author}</span>
                    <span className="text-book-text-sub truncate flex-1">{version.message}</span>
                  </li>
                );
              })}
            </ul>
            {diffResult != null && (
              <pre className="mt-2 rounded border border-book-border/40 bg-book-bg-tertiary/40 p-2 text-[10px] font-mono whitespace-pre-wrap break-words max-h-44 overflow-y-auto custom-scrollbar">
                {diffResult}
              </pre>
            )}
          </>
        )}
      </div>
    </details>
  );
};
