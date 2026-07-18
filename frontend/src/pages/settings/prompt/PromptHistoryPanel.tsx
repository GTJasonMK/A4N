import React, { useEffect, useRef, useState } from "react";
import { Check, History, Loader2 } from "lucide-react";
import { BookButton } from "../../../components/ui/BookButton";
import { useToast } from "../../../components/feedback/Toast";
import { extractErrorMessage } from "../../../api";
import {
  promptVersionDiff,
  promptVersionHistory,
  type PromptEntry,
  type PromptVersion,
} from "../../../api/prompt";
import { formatUiDateTime } from "../../../utils/dateTime";

const promptVersionLabel = (index: number): string =>
  index === 0 ? "最新版本" : `历史版本 ${index}`;

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
      className="py-2.5"
    >
      <summary className="cursor-pointer text-xs text-book-text-sub inline-flex items-center gap-1.5">
        <History size={11} /> 版本历史
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
            <div className="mb-1.5 flex items-center gap-2 text-[11px]">
              <span className="text-book-text-muted">选择两个版本查看内容差异</span>
              {diffPair.length === 2 && (
                <BookButton variant="secondary" size="sm" onClick={handleDiff} disabled={diffLoading}>
                  {diffLoading ? (
                    <>
                      <Loader2 size={10} className="animate-spin" />
                      对比中…
                    </>
                  ) : (
                    <>对比已选版本</>
                  )}
                </BookButton>
              )}
            </div>
            <ul className="divide-y divide-book-border/35 border-y border-book-border/35">
              {history.map((version, index) => {
                const isPicked = diffPair.includes(version.version);
                return (
                  <li
                    key={version.version}
                    className={isPicked ? "bg-book-primary/10" : ""}
                  >
                    <button
                      type="button"
                      aria-pressed={isPicked}
                      onClick={() => toggleDiffPick(version.version)}
                      className="flex w-full cursor-pointer items-start gap-2.5 px-3 py-2.5 text-left"
                    >
                      <span className={`mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${isPicked ? "border-book-primary bg-book-primary text-white" : "border-book-border text-transparent"}`}>
                        <Check size={10} />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                          <span className="font-medium text-book-text-main">
                            {promptVersionLabel(index)}
                          </span>
                          <time className="text-[10px] tabular-nums text-book-text-muted">
                            {formatUiDateTime(version.timestamp)}
                          </time>
                          {version.author ? (
                            <span className="text-[10px] text-book-text-muted">
                              {version.author}
                            </span>
                          ) : null}
                        </span>
                        <span className="mt-0.5 block truncate text-[11px] text-book-text-sub">
                          {version.message || "保存提示词"}
                        </span>
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
            {diffResult != null && (
              <pre className="mt-2 whitespace-pre-wrap break-words border-l border-book-border/45 pl-2.5 font-mono text-[11px] leading-relaxed text-book-text-sub">
                {diffResult}
              </pre>
            )}
          </>
        )}
      </div>
    </details>
  );
};
