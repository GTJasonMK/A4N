import React, { useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { extractErrorMessage } from "../../api";
import {
  ragEnhancedSearch,
  type RagEnhancedQuerySummary,
  type RagSearchResult,
} from "../../api/rag";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";

interface RagQuickPanelProps {
  projectId: number;
  chapterNumber: number | null;
}

export const RagQuickPanel: React.FC<RagQuickPanelProps> = ({ projectId, chapterNumber }) => {
  const { addToast } = useToast();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<RagSearchResult[] | null>(null);
  const [queries, setQueries] = useState<RagEnhancedQuerySummary[]>([]);
  const [candidateCount, setCandidateCount] = useState(0);
  const [busy, setBusy] = useState(false);
  const queryReady = query.trim().length > 0;
  const mountedRef = useRef(false);
  const activeScopeRef = useRef({ projectId, chapterNumber });
  const queryRef = useRef(query);
  const searchSeqRef = useRef(0);
  const searchBusyRef = useRef(false);

  const clearResults = () => {
    setResults(null);
    setQueries([]);
    setCandidateCount(0);
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      searchSeqRef.current += 1;
      searchBusyRef.current = false;
    };
  }, []);

  useEffect(() => {
    activeScopeRef.current = { projectId, chapterNumber };
    searchSeqRef.current += 1;
    searchBusyRef.current = false;
    setBusy(false);
    clearResults();
  }, [projectId, chapterNumber]);

  const isCurrentSearch = (
    requestProjectId: number,
    requestChapterNumber: number | null,
    requestQuery: string,
    requestSeq: number,
  ) =>
    mountedRef.current &&
    searchSeqRef.current === requestSeq &&
    activeScopeRef.current.projectId === requestProjectId &&
    activeScopeRef.current.chapterNumber === requestChapterNumber &&
    queryRef.current.trim() === requestQuery;

  const handleQueryChange = (value: string) => {
    queryRef.current = value;
    setQuery(value);
    searchSeqRef.current += 1;
    searchBusyRef.current = false;
    setBusy(false);
    clearResults();
  };

  const handleSearch = async () => {
    if (searchBusyRef.current) return;
    const requestQuery = queryRef.current.trim();
    if (!requestQuery) {
      addToast("请输入查询文本", "error");
      return;
    }
    const requestProjectId = projectId;
    const requestChapterNumber = chapterNumber;
    activeScopeRef.current = { projectId: requestProjectId, chapterNumber: requestChapterNumber };
    const requestSeq = searchSeqRef.current + 1;
    searchSeqRef.current = requestSeq;
    searchBusyRef.current = true;
    setBusy(true);
    try {
      const data = await ragEnhancedSearch(requestProjectId, {
        query: requestQuery,
        top_k: 10,
        chapter_number: requestChapterNumber,
      });
      if (
        !isCurrentSearch(
          requestProjectId,
          requestChapterNumber,
          requestQuery,
          requestSeq,
        )
      ) {
        return;
      }
      setResults(data.results);
      setQueries(data.queries);
      setCandidateCount(data.candidate_count);
    } catch (err) {
      if (
        isCurrentSearch(
          requestProjectId,
          requestChapterNumber,
          requestQuery,
          requestSeq,
        )
      ) {
        addToast(extractErrorMessage(err, "资料检索失败"), "error");
      }
    } finally {
      if (
        isCurrentSearch(
          requestProjectId,
          requestChapterNumber,
          requestQuery,
          requestSeq,
        )
      ) {
        setBusy(false);
        searchBusyRef.current = false;
      }
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-[11px] text-book-text-muted">
        查询
        <textarea
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          placeholder="输入人物、事件或设定关键词…"
          rows={2}
          className="pyqt-input mt-1 w-full resize-y text-xs"
        />
      </label>
      <div className="flex items-center gap-2">
        {results != null ? (
          <BookButton variant="ghost" size="sm" onClick={clearResults} disabled={busy}>
            清除结果
          </BookButton>
        ) : null}
        <BookButton
          variant="primary"
          size="sm"
          className="ml-auto flex-1 sm:flex-none"
          onClick={handleSearch}
          disabled={busy || !queryReady}
        >
          {busy ? (
            <span className="inline-flex items-center gap-1">
              <Loader2 size={11} className="animate-spin" /> 查询中…
            </span>
          ) : (
            "检索"
          )}
        </BookButton>
      </div>
      {results != null && (
        <div className="mt-2 space-y-2 border-t border-book-border/40 pt-2">
          <div className="text-[10px] text-book-text-muted">{results.length} 条结果</div>
          {results.length === 0 ? (
            <div className="py-4 text-center text-[11px] text-book-text-muted">没有找到相关资料。</div>
          ) : (
            <ul className="grid grid-cols-1 gap-2">
              {results.map((result, index) => (
                <li key={result.id} className="a4n-flat-surface p-2.5 text-[11px]">
                  <div className="mb-1 flex items-center justify-between gap-2">
                    <span className="min-w-0 truncate font-medium text-book-text-main">
                      {searchResultTitle(result, index)}
                    </span>
                    <span className="shrink-0 text-[10px] text-book-text-muted">
                      相关度 {result.score.toFixed(2)}
                    </span>
                  </div>
                  <p className="line-clamp-4 leading-relaxed text-book-text-sub">{result.content}</p>
                </li>
              ))}
            </ul>
          )}
          {queries.length > 0 || candidateCount > 0 ? (
            <details className="text-[10px] text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">检索范围</summary>
              <div className="mt-1.5 space-y-1 border-l-2 border-book-border/60 pl-2">
                {candidateCount > 0 ? <div>候选资料 {candidateCount} 条</div> : null}
                {queries.map((item) => (
                  <div key={`${item.kind}-${item.query}`} className="leading-relaxed">
                    <span className="font-medium text-book-text-sub">{formatQueryKind(item.kind)}：</span>
                    {item.query}
                  </div>
                ))}
              </div>
            </details>
          ) : null}
        </div>
      )}
    </div>
  );
};

function formatQueryKind(kind: string): string {
  switch (kind) {
    case "main":
      return "主查询";
    case "character":
      return "角色";
    case "foreshadowing":
      return "伏笔";
    case "location":
      return "场景";
    default:
      return "其他查询";
  }
}

function searchResultTitle(result: RagSearchResult, index: number): string {
  if (result.metadata && typeof result.metadata === "object" && !Array.isArray(result.metadata)) {
    const metadata = result.metadata as Record<string, unknown>;
    for (const key of ["title", "label", "name"]) {
      const value = metadata[key];
      if (typeof value === "string" && value.trim()) return value.trim();
    }
    const chapterNumber = metadata.chapter_number;
    if (typeof chapterNumber === "number" && Number.isFinite(chapterNumber)) {
      return `第 ${chapterNumber} 章`;
    }
  }
  return `结果 ${index + 1}`;
}
