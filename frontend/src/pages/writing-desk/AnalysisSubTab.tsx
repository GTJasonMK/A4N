import React, { useEffect, useMemo, useState } from "react";
import { Eye, Loader2, Sparkles } from "lucide-react";
import { listObjectProfiles, type ObjectProfile } from "../../api/objectProfiles";
import { BookButton } from "../../components/ui/BookButton";
import {
  analysisTextList,
  firstAnalysisText as firstText,
} from "../../utils/chapterAnalysisText";
import { asArray, asJsonRecord as asRecord, type JsonRecord } from "../../utils/jsonRecord";

interface AnalysisSubTabProps {
  projectId: number;
  chapterNumber: number;
  analysis: unknown;
  analysisBlockedReason: string | null;
  generating: boolean;
  canGenerate: boolean;
  onGenerate: () => void | Promise<void>;
  onPreviewGenerate: () => void | Promise<void>;
  canResumeSync: boolean;
  onResumeSync: () => void | Promise<void>;
  onPreviewResumeSync: () => void | Promise<void>;
}

export const AnalysisSubTab: React.FC<AnalysisSubTabProps> = ({
  projectId,
  chapterNumber,
  analysis,
  analysisBlockedReason,
  generating,
  canGenerate,
  onGenerate,
  onPreviewGenerate,
  canResumeSync,
  onResumeSync,
  onPreviewResumeSync,
}) => {
  const [objectProfiles, setObjectProfiles] = useState<ObjectProfile[]>([]);
  const [objectProfilesError, setObjectProfilesError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!analysis) {
      setObjectProfiles([]);
      setObjectProfilesError(null);
      return;
    }
    void listObjectProfiles(projectId)
      .then((rows) => {
        if (cancelled) return;
        setObjectProfiles(rows);
        setObjectProfilesError(null);
      })
      .catch((err) => {
        if (cancelled) return;
        setObjectProfiles([]);
        setObjectProfilesError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [analysis, projectId]);

  const chapterObjects = useMemo(
    () =>
      objectProfiles.filter(
        (profile) =>
          profile.first_seen_chapter <= chapterNumber &&
          profile.last_seen_chapter >= chapterNumber,
      ),
    [chapterNumber, objectProfiles],
  );

  const root = asRecord(analysis);
  if (!root) {
    return (
      <div className="h-full overflow-y-auto custom-scrollbar p-2 text-xs text-book-text-muted">
        <div className="flex h-full items-center justify-center">
          <div className="flex flex-col items-center gap-2">
            {analysisBlockedReason ? (
              <p className="max-w-md text-center text-[11px] leading-relaxed text-error">
                {analysisBlockedReason}
              </p>
            ) : null}
            <div className="flex items-center gap-2">
              <BookButton
                variant="secondary"
                onClick={() => void onPreviewGenerate()}
                disabled={!canGenerate || generating}
              >
                <span className="inline-flex items-center gap-1.5">
                  <Eye size={13} />
                  预览提示词
                </span>
              </BookButton>
              <BookButton
                variant="secondary"
                onClick={() => void onGenerate()}
                disabled={!canGenerate || generating}
              >
                <span className="inline-flex items-center gap-1.5">
                  {generating ? (
                    <Loader2 size={13} className="animate-spin" />
                  ) : (
                    <Sparkles size={13} />
                  )}
                  {generating ? "分析中…" : "生成分析"}
                </span>
              </BookButton>
            </div>
          </div>
        </div>
      </div>
    );
  }

  const metadata = asRecord(root.metadata);
  const summaries = asRecord(root.summaries);
  const characterStates = asRecord(root.character_states);
  const foreshadowing = asRecord(root.foreshadowing);
  const keyEvents = asArray(root.key_events).map(asRecord).filter(Boolean) as JsonRecord[];

  const summary =
    firstText(summaries, ["compressed", "summary", "short"]) ||
    firstText(summaries, ["one_line"]);
  const keywords = analysisTextList(summaries?.keywords, { limit: 8 });
  const metadataGroups = [
    ["角色", analysisTextList(metadata?.characters, { limit: 12 })],
    ["地点", analysisTextList(metadata?.locations, { limit: 8 })],
    ["物品", analysisTextList(metadata?.items, { limit: 8 })],
    ["标签", analysisTextList(metadata?.tags, { limit: 8 })],
  ] as const;
  const stateRows = characterStates
    ? Object.entries(characterStates)
        .map(([name, raw]) => ({ name, data: asRecord(raw) }))
        .filter((row): row is { name: string; data: JsonRecord } => Boolean(row.data))
    : [];
  const planted = asArray(foreshadowing?.planted).map(asRecord).filter(Boolean) as JsonRecord[];
  const resolved = asArray(foreshadowing?.resolved).map(asRecord).filter(Boolean) as JsonRecord[];
  const tensions = analysisTextList(foreshadowing?.tensions, { limit: 10 });

  return (
    <div className="h-full overflow-y-auto custom-scrollbar p-2 text-xs text-book-text-sub">
      <div className="mb-2 flex items-center justify-end gap-2">
        {analysisBlockedReason ? (
          <span className="mr-auto max-w-[60%] text-[10px] leading-relaxed text-error">
            {analysisBlockedReason}
          </span>
        ) : null}
        <BookButton
          variant="secondary"
          onClick={() => void onPreviewResumeSync()}
          disabled={!canResumeSync || generating}
        >
          <span className="inline-flex items-center gap-1.5">
            <Eye size={13} />
            预览同步
          </span>
        </BookButton>
        <BookButton
          variant="secondary"
          onClick={() => void onResumeSync()}
          disabled={!canResumeSync || generating}
        >
          <span className="inline-flex items-center gap-1.5">
            {generating ? (
              <Loader2 size={13} className="animate-spin" />
            ) : (
              <Sparkles size={13} />
            )}
            继续同步
          </span>
        </BookButton>
        <BookButton
          variant="secondary"
          onClick={() => void onPreviewGenerate()}
          disabled={!canGenerate || generating}
        >
          <span className="inline-flex items-center gap-1.5">
            <Eye size={13} />
            预览分析
          </span>
        </BookButton>
        <BookButton
          variant="secondary"
          onClick={() => void onGenerate()}
          disabled={!canGenerate || generating}
        >
          <span className="inline-flex items-center gap-1.5">
            {generating ? (
              <Loader2 size={13} className="animate-spin" />
            ) : (
              <Sparkles size={13} />
            )}
            {generating ? "分析中…" : "重新分析"}
          </span>
        </BookButton>
      </div>
      <div className="grid grid-cols-1 xl:grid-cols-2 gap-2">
        <section className="a4n-page-surface p-2 xl:col-span-2">
          <div className="flex items-center justify-between gap-2 mb-1">
            <h3 className="text-[11px] font-semibold text-book-text-main">摘要</h3>
            {keywords.length > 0 && (
              <div className="flex flex-wrap justify-end gap-1">
                {keywords.map((word) => (
                  <span key={word} className="pyqt-pill !text-[9px]">
                    {word}
                  </span>
                ))}
              </div>
            )}
          </div>
          {summary && <p className="leading-relaxed whitespace-pre-wrap">{summary}</p>}
        </section>

        <section className="a4n-page-surface p-2">
          <h3 className="text-[11px] font-semibold text-book-text-main mb-2">元数据</h3>
          <div className="space-y-2">
            {metadataGroups
              .filter(([, values]) => values.length > 0)
              .map(([label, values]) => (
                <div key={label} className="grid grid-cols-[42px_1fr] gap-2">
                  <span className="text-book-text-muted">{label}</span>
                  <div className="flex flex-wrap gap-1">
                    {values.map((value) => (
                      <span key={value} className="pyqt-pill !text-[9px]">
                        {value}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            {firstText(metadata, ["tone"]) && (
              <div className="grid grid-cols-[42px_1fr] gap-2">
                <span className="text-book-text-muted">基调</span>
                <span>{firstText(metadata, ["tone"])}</span>
              </div>
            )}
            {firstText(metadata, ["timeline_marker"]) && (
              <div className="grid grid-cols-[42px_1fr] gap-2">
                <span className="text-book-text-muted">时间</span>
                <span>{firstText(metadata, ["timeline_marker"])}</span>
              </div>
            )}
          </div>
        </section>

        <section className="a4n-page-surface p-2">
          <h3 className="text-[11px] font-semibold text-book-text-main mb-2">关键事件</h3>
          {keyEvents.length > 0 && (
            <ul className="space-y-1.5">
              {keyEvents.map((event, idx) => {
                const desc = firstText(event, ["description", "summary", "event"]);
                if (!desc) return null;
                const involved = analysisTextList(event.involved_characters, { limit: 8 });
                return (
                  <li key={`${desc}-${idx}`} className="border-l border-book-border/70 pl-2">
                    <div className="flex flex-wrap items-center gap-1 mb-0.5">
                      <span className="font-medium text-book-text-main">{desc}</span>
                      {firstText(event, ["type"]) && (
                        <span className="pyqt-pill !text-[9px]">{firstText(event, ["type"])}</span>
                      )}
                      {firstText(event, ["importance"]) && (
                        <span className="pyqt-pill !text-[9px]">
                          {firstText(event, ["importance"])}
                        </span>
                      )}
                    </div>
                    {involved.length > 0 && (
                      <div className="text-[10px] text-book-text-muted">
                        {involved.join("、")}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </section>

        <section className="a4n-page-surface p-2">
          <h3 className="text-[11px] font-semibold text-book-text-main mb-2">识别物体</h3>
          {objectProfilesError ? (
            <p className="text-[10px] text-error">{objectProfilesError}</p>
          ) : chapterObjects.length > 0 ? (
            <ul className="space-y-1.5">
              {chapterObjects.slice(0, 8).map((item) => (
                <li key={item.id} className="border-l border-book-border/70 pl-2">
                  <div className="flex flex-wrap items-center gap-1">
                    <span className="font-medium text-book-text-main">{item.name}</span>
                    <span className="pyqt-pill !text-[9px]">{item.importance}</span>
                    <span className="pyqt-pill !text-[9px]">{item.object_type}</span>
                  </div>
                  <div className="mt-0.5 line-clamp-2 text-[10px] text-book-text-muted">
                    {item.story_usage}
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-[10px] text-book-text-muted">
              本章尚未同步出物体档案。
            </p>
          )}
        </section>

        <section className="a4n-page-surface p-2">
          <h3 className="text-[11px] font-semibold text-book-text-main mb-2">角色状态</h3>
          {stateRows.length > 0 && (
            <ul className="space-y-1.5">
              {stateRows.map(({ name, data }) => {
                const changes = analysisTextList(data.changes, { limit: 6 });
                const status = firstText(data, ["status"]);
                return (
                  <li key={name} className="grid grid-cols-[72px_1fr] gap-2">
                    <span className="font-medium text-book-text-main truncate" title={name}>
                      {name}
                    </span>
                    <div className="min-w-0">
                      {status && <div className="leading-relaxed">{status}</div>}
                      <div className="text-[10px] text-book-text-muted">
                        {[
                          firstText(data, ["location", "current_location"]),
                          firstText(data, ["emotional_state", "emotion"]),
                        ]
                          .filter(Boolean)
                          .join(" / ")}
                      </div>
                      {changes.length > 0 && (
                        <div className="text-[10px] text-book-text-sub mt-0.5">
                          {changes.join("；")}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </section>

        <section className="a4n-page-surface p-2">
          <h3 className="text-[11px] font-semibold text-book-text-main mb-2">伏笔</h3>
          {(planted.length > 0 || resolved.length > 0 || tensions.length > 0) && (
            <div className="space-y-2">
              {planted.length > 0 && (
                <div>
                  <div className="text-book-text-muted mb-1">埋设</div>
                  <ul className="space-y-1">
                    {planted.map((item, idx) => {
                      const desc = firstText(item, ["description", "content"]);
                      if (!desc) return null;
                      return (
                        <li key={`${desc}-${idx}`} className="border-l border-book-border/70 pl-2">
                          <div>{desc}</div>
                          <div className="flex flex-wrap gap-1 mt-0.5">
                            {[
                              firstText(item, ["category", "type"]),
                              firstText(item, ["priority", "importance"]),
                            ]
                              .filter(Boolean)
                              .map((tag) => (
                                <span key={tag} className="pyqt-pill !text-[9px]">
                                  {tag}
                                </span>
                              ))}
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {resolved.length > 0 && (
                <div>
                  <div className="text-book-text-muted mb-1">回收</div>
                  <ul className="space-y-1">
                    {resolved.map((item, idx) => {
                      const desc = firstText(item, ["resolution", "description", "content"]);
                      if (!desc) return null;
                      return (
                        <li key={idx} className="border-l border-book-border/70 pl-2">
                          {desc}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {tensions.length > 0 && (
                <div>
                  <div className="text-book-text-muted mb-1">悬念</div>
                  <div className="flex flex-wrap gap-1">
                    {tensions.map((item) => (
                      <span key={item} className="pyqt-pill !text-[9px]">
                        {item}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
