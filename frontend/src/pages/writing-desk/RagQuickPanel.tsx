import React, { useEffect, useRef, useState } from "react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { extractErrorMessage } from "../../api";
import {
  ragEnhancedSearch,
  type RagEnhancedSearchOutput,
  type RagSearchResult,
} from "../../api/rag";
import { useToast } from "../../components/feedback/Toast";
import { BookButton } from "../../components/ui/BookButton";
import { formatNumber, useI18n } from "../../i18n";

interface RagQuickPanelProps {
  projectId: number;
  chapterNumber: number | null;
}

export const RagQuickPanel: React.FC<RagQuickPanelProps> = ({ projectId, chapterNumber }) => {
  const { addToast } = useToast();
  const { locale, t } = useI18n();
  const [query, setQuery] = useState("");
  const [searchResult, setSearchResult] = useState<RagEnhancedSearchOutput | null>(null);
  const [busy, setBusy] = useState(false);
  const results = searchResult?.results ?? null;
  const queries = searchResult?.queries ?? [];
  const candidateCount = searchResult?.candidate_count ?? 0;
  const queryReady = query.trim().length > 0;
  const searchSeqRef = useRef(0);
  const searchBusyRef = useRef(false);

  const clearResults = () => setSearchResult(null);

  useEffect(
    () => () => {
      searchSeqRef.current += 1;
    },
    [],
  );

  useEffect(() => {
    searchSeqRef.current += 1;
    searchBusyRef.current = false;
    setBusy(false);
    setSearchResult(null);
  }, [projectId, chapterNumber]);

  const isCurrentSearch = (requestSeq: number) => searchSeqRef.current === requestSeq;

  const handleQueryChange = (value: string) => {
    setQuery(value);
    searchSeqRef.current += 1;
    searchBusyRef.current = false;
    setBusy(false);
    clearResults();
  };

  const handleSearch = async () => {
    if (searchBusyRef.current) return;
    const requestQuery = query.trim();
    const requestSeq = searchSeqRef.current + 1;
    searchSeqRef.current = requestSeq;
    searchBusyRef.current = true;
    setBusy(true);
    try {
      const data = await ragEnhancedSearch(projectId, {
        query: requestQuery,
        top_k: 10,
        chapter_number: chapterNumber,
      });
      if (!isCurrentSearch(requestSeq)) {
        return;
      }
      setSearchResult(data);
    } catch (err) {
      if (isCurrentSearch(requestSeq)) {
        addToast(extractErrorMessage(err, t("writing.searchFailed")), "error");
      }
    } finally {
      if (isCurrentSearch(requestSeq)) {
        setBusy(false);
        searchBusyRef.current = false;
      }
    }
  };

  return (
    <div className="space-y-2">
      <label className="block text-small text-book-text-muted">
        {t("writing.searchQuery")}
        <textarea
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          placeholder={t("writing.searchPlaceholder")}
          rows={2}
          className="pyqt-input mt-1 w-full resize-y text-xs"
        />
      </label>
      <div className="flex items-center gap-2">
        {results != null ? (
          <BookButton variant="ghost" size="sm" onClick={clearResults} disabled={busy}>
            {t("writing.clearResults")}
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
              <LoadingSpinner size="sm" /> {t("writing.searching")}
            </span>
          ) : (
            t("writing.search")
          )}
        </BookButton>
      </div>
      {results != null && (
        <div className="mt-2 space-y-2 border-t border-book-border/40 pt-2">
          <div className="text-caption text-book-text-muted">{t("writing.resultCount", { count: results.length })}</div>
          {results.length === 0 ? (
            <div className="py-4 text-center text-small text-book-text-muted">{t("writing.noResults")}</div>
          ) : (
            <ul className="grid grid-cols-1 gap-2">
              {results.map((result, index) => (
                <li key={result.id} className="a4n-flat-surface p-2.5 text-small">
                  <div className="mb-1 flex items-center justify-between gap-2">
                    <span className="min-w-0 truncate font-medium text-book-text-main">
                      {searchResultTitle(result, index, t)}
                    </span>
                    <span className="shrink-0 text-caption text-book-text-muted">
                      {t("writing.relevance", { value: formatNumber(result.score, locale, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) })}
                    </span>
                  </div>
                  <p className="line-clamp-4 leading-relaxed text-book-text-sub">{result.content}</p>
                </li>
              ))}
            </ul>
          )}
          {queries.length > 0 || candidateCount > 0 ? (
            <details className="text-caption text-book-text-muted">
              <summary className="w-fit cursor-pointer hover:text-book-text-main">{t("writing.searchScope")}</summary>
              <div className="mt-1.5 space-y-1 border-l-2 border-book-border/60 pl-2">
                {candidateCount > 0 ? <div>{t("writing.candidateCount", { count: candidateCount })}</div> : null}
                {queries.map((item) => (
                  <div key={`${item.label}-${item.query}`} className="leading-relaxed">
                    <span className="font-medium text-book-text-sub">{item.label}：</span>
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

function searchResultTitle(result: RagSearchResult, index: number, t: ReturnType<typeof useI18n>["t"]): string {
  if (result.metadata && typeof result.metadata === "object" && !Array.isArray(result.metadata)) {
    const metadata = result.metadata as Record<string, unknown>;
    for (const key of ["title", "label", "name"]) {
      const value = metadata[key];
      if (typeof value === "string" && value.trim()) return value.trim();
    }
    const chapterNumber = metadata.chapter_number;
    if (typeof chapterNumber === "number" && Number.isFinite(chapterNumber)) {
      return t("writing.searchChapterResult", { chapter: chapterNumber });
    }
  }
  return t("writing.searchResult", { index: index + 1 });
}
