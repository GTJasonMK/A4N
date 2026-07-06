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
        addToast(extractErrorMessage(err, "RAG 查询失败"), "error");
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
      <div className="text-[10px] text-book-text-muted">
        {chapterNumber != null ? `当前章节：第 ${chapterNumber} 章` : "全项目检索"}
      </div>
      <textarea
        value={query}
        onChange={(e) => handleQueryChange(e.target.value)}
        placeholder="输入查询文本，测试 RAG 检索效果…"
        rows={3}
        className="pyqt-input w-full text-xs resize-y"
      />
      <BookButton variant="primary" onClick={handleSearch} disabled={busy}>
        {busy ? (
          <span className="inline-flex items-center gap-1">
            <Loader2 size={11} className="animate-spin" /> 查询中…
        </span>
      ) : (
          "增强检索"
      )}
      </BookButton>
      {results != null && (
        <div className="border-t border-book-border/40 pt-2 mt-2 space-y-1.5">
          <div className="text-[10px] text-book-text-muted">
            找到 {results.length} 条 · 候选 {candidateCount} 条
          </div>
          {queries.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {queries.map((item) => (
                <span
                  key={`${item.kind}-${item.query}`}
                  className="pyqt-pill max-w-full !text-[9px]"
                  title={item.query}
                >
                  {formatQueryKind(item.kind)} · {item.weight.toFixed(2)}
                </span>
              ))}
            </div>
          )}
          {results.length === 0 ? (
            <div className="text-[11px] text-book-text-muted italic">无匹配结果</div>
          ) : (
            results.map((r) => (
              <div key={r.id} className="a4n-stat-tile p-1.5 text-[11px]">
                <div className="flex items-center gap-1.5 mb-0.5">
                  <span className="font-mono text-[10px] text-book-text-muted">{r.id}</span>
                  <span className="pyqt-pill !text-[9px]">综合 {r.score.toFixed(3)}</span>
                </div>
                <p className="text-book-text-sub line-clamp-3">{r.content}</p>
              </div>
            ))
          )}
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
      return kind;
  }
}
