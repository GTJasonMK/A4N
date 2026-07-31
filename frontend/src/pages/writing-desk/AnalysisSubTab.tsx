import React, { useEffect, useMemo, useState } from "react";
import { Eye, Sparkles } from "lucide-react";
import { LoadingSpinner } from "../../components/ui/LoadingSpinner";
import { listObjectProfiles, type ObjectProfile } from "../../api/objectProfiles";
import { BookButton } from "../../components/ui/BookButton";
import { Dropdown } from "../../components/ui/Dropdown";
import {
  analysisTextList,
  firstAnalysisText as firstText,
} from "../../utils/chapterAnalysisText";
import { asArray, asJsonRecord as asRecord, type JsonRecord } from "../../utils/jsonRecord";
import {
  getObjectProfileImportanceLabel,
  getObjectProfileTypeLabel,
} from "../../utils/objectProfilePresentation";
import {
  chapterEventImportanceLabel,
  chapterEventTypeLabel,
} from "../../utils/chapterAnalysisPresentation";

const FORESHADOWING_CATEGORY_LABELS: Record<string, string> = {
  character_secret: "角色秘密",
  plot_twist: "剧情转折",
  item_mystery: "物品谜团",
  world_rule: "世界规则",
};

const knownLabel = (
  labels: Record<string, string>,
  value: string | null,
): string | null => (value ? labels[value] ?? null : null);

const ANALYSIS_CARD_CLASS = "a4n-flat-surface mb-2 break-inside-avoid p-3 sm:p-4";

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
  const [objectProfilesLoading, setObjectProfilesLoading] = useState(Boolean(analysis));

  useEffect(() => {
    let cancelled = false;
    if (!analysis) {
      setObjectProfiles([]);
      setObjectProfilesError(null);
      setObjectProfilesLoading(false);
      return;
    }
    setObjectProfilesLoading(true);
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
      })
      .finally(() => {
        if (cancelled) return;
        setObjectProfilesLoading(false);
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
    const emptyStateTitle = generating
      ? `正在分析第 ${chapterNumber} 章…`
      : analysisBlockedReason
        ? `第 ${chapterNumber} 章暂时无法生成分析`
        : `第 ${chapterNumber} 章尚未生成分析`;

    return (
      <div className="flex h-full min-h-0 items-center justify-center overflow-y-auto p-4 text-xs">
        <div className="flex w-full max-w-md flex-col items-center text-center">
          {generating ? (
            <LoadingSpinner size="lg" className="mb-3" />
          ) : (
            <Sparkles size={20} className="mb-3 text-book-text-muted" />
          )}
          <h3 className="text-sm font-semibold text-book-text-main">{emptyStateTitle}</h3>
          <p
            className={`mt-1.5 max-w-sm leading-relaxed ${
              analysisBlockedReason ? "text-error" : "text-book-text-muted"
            }`}
          >
            {analysisBlockedReason ?? "生成后可查看摘要、章节要素、关键事件与角色状态。"}
          </p>
          {!generating && canGenerate ? (
            <div className="mt-4 flex w-full max-w-xs items-center gap-2">
              <Dropdown
                items={[
                  {
                    label: "预览模型请求",
                    icon: <Eye size={12} />,
                    onClick: () => void onPreviewGenerate(),
                  },
                ]}
              />
              <BookButton
                variant="primary"
                size="sm"
                onClick={() => void onGenerate()}
                className="flex-1"
              >
                <Sparkles size={13} />
                生成分析
              </BookButton>
            </div>
          ) : null}
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
  const metadataRows = [
    { label: "角色", value: analysisTextList(metadata?.characters, { limit: 12 }).join("、") },
    { label: "地点", value: analysisTextList(metadata?.locations, { limit: 8 }).join("、") },
    { label: "物品", value: analysisTextList(metadata?.items, { limit: 8 }).join("、") },
    { label: "标签", value: analysisTextList(metadata?.tags, { limit: 8 }).join("、") },
    { label: "基调", value: firstText(metadata, ["tone"]) },
    { label: "时间", value: firstText(metadata, ["timeline_marker"]) },
  ].filter((row): row is { label: string; value: string } => Boolean(row.value));
  const stateRows = characterStates
    ? Object.entries(characterStates)
        .map(([name, raw]) => ({ name, data: asRecord(raw) }))
        .filter((row): row is { name: string; data: JsonRecord } => Boolean(row.data))
    : [];
  const planted = asArray(foreshadowing?.planted).map(asRecord).filter(Boolean) as JsonRecord[];
  const resolved = asArray(foreshadowing?.resolved).map(asRecord).filter(Boolean) as JsonRecord[];
  const tensions = analysisTextList(foreshadowing?.tensions, { limit: 10 });
  const hasForeshadowing = planted.length > 0 || resolved.length > 0 || tensions.length > 0;
  const secondaryActions: React.ComponentProps<typeof Dropdown>["items"] = [
    ...(canResumeSync
      ? [
          {
            label: "预览同步请求",
            icon: <Eye size={12} />,
            onClick: () => void onPreviewResumeSync(),
          },
        ]
      : []),
    ...(canGenerate
      ? [
          {
            label: canResumeSync ? "预览重新分析请求" : "预览分析请求",
            icon: <Eye size={12} />,
            onClick: () => void onPreviewGenerate(),
          },
        ]
      : []),
    ...(canResumeSync && canGenerate
      ? [
          { type: "divider" as const },
          {
            label: "重新分析",
            icon: <Sparkles size={12} />,
            onClick: () => void onGenerate(),
          },
        ]
      : []),
  ];

  return (
    <div className="h-full overflow-y-auto custom-scrollbar p-2 text-xs text-book-text-sub">
      <div className="mx-auto mb-2 w-full max-w-5xl space-y-2 sm:flex sm:items-center sm:justify-end sm:space-y-0 sm:gap-2">
        {analysisBlockedReason ? (
          <p className="min-w-0 text-small leading-relaxed text-error sm:mr-auto sm:flex-1">
            {analysisBlockedReason}
          </p>
        ) : null}
        {generating || canResumeSync || canGenerate ? (
          <div className="flex items-center gap-2 sm:contents">
            {!generating && secondaryActions.length > 0 ? (
              <Dropdown items={secondaryActions} />
            ) : null}
            <BookButton
              variant="primary"
              size="sm"
              className="flex-1 sm:flex-none"
              onClick={() => {
                if (canResumeSync) {
                  void onResumeSync();
                  return;
                }
                void onGenerate();
              }}
              disabled={generating}
            >
              <span className="inline-flex items-center gap-1.5">
                {generating ? (
                  <LoadingSpinner size="sm" />
                ) : (
                  <Sparkles size={13} />
                )}
                {generating ? "处理中…" : canResumeSync ? "继续同步" : "重新分析"}
              </span>
            </BookButton>
          </div>
        ) : null}
      </div>
      <div className="mx-auto w-full max-w-5xl columns-1 gap-2 sm:columns-2">
        {summary || keywords.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">摘要</h3>
            {summary ? <p className="whitespace-pre-wrap leading-relaxed">{summary}</p> : null}
            {keywords.length > 0 ? (
              <p className={`${summary ? "mt-2" : ""} text-small text-book-text-muted`}>
                关键词：{keywords.join("、")}
              </p>
            ) : null}
          </section>
        ) : null}

        {metadataRows.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">章节要素</h3>
            <dl className="space-y-2">
              {metadataRows.map(({ label, value }) => (
                <div key={label} className="grid grid-cols-[42px_minmax(0,1fr)] gap-2 leading-relaxed">
                  <dt className="text-book-text-muted">{label}</dt>
                  <dd className="min-w-0 text-book-text-sub">{value}</dd>
                </div>
              ))}
            </dl>
          </section>
        ) : null}

        {keyEvents.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">关键事件</h3>
            <ul className="divide-y divide-book-border/35">
              {keyEvents.map((event, idx) => {
                const desc = firstText(event, ["description", "summary", "event"]);
                if (!desc) return null;
                const involved = analysisTextList(event.involved_characters, { limit: 8 });
                const eventType = chapterEventTypeLabel(firstText(event, ["type"]));
                const importance = chapterEventImportanceLabel(
                  firstText(event, ["importance"]),
                );
                const detail = [eventType, importance, ...involved].filter(Boolean).join(" · ");
                return (
                  <li key={`${desc}-${idx}`} className="py-2 first:pt-0 last:pb-0">
                    <div className="font-medium leading-relaxed text-book-text-main">{desc}</div>
                    {detail ? (
                      <div className="mt-0.5 text-small text-book-text-muted">{detail}</div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}

        <section className={ANALYSIS_CARD_CLASS}>
          <h3 className="mb-2 text-small font-semibold text-book-text-main">识别物体</h3>
          {objectProfilesLoading ? (
            <div className="flex items-center gap-2 text-small text-book-text-muted">
              <LoadingSpinner size="sm" />
              正在同步物体档案…
            </div>
          ) : objectProfilesError ? (
            <div>
              <p className="text-small text-error">物体档案暂时无法加载。</p>
              <details className="mt-2 text-caption text-book-text-muted">
                <summary className="w-fit cursor-pointer hover:text-book-text-main">诊断信息</summary>
                <pre className="mt-1.5 whitespace-pre-wrap break-all rounded-lg bg-book-bg-tertiary/70 p-2 font-mono leading-relaxed">
                  {objectProfilesError}
                </pre>
              </details>
            </div>
          ) : chapterObjects.length > 0 ? (
            <ul className="divide-y divide-book-border/35">
              {chapterObjects.slice(0, 8).map((item) => (
                <li key={item.id} className="py-2 first:pt-0 last:pb-0">
                  <div className="font-medium text-book-text-main">{item.name}</div>
                  <div className="mt-0.5 text-small text-book-text-muted">
                    {getObjectProfileImportanceLabel(item.importance)} ·{" "}
                    {getObjectProfileTypeLabel(item.object_type)}
                  </div>
                  {item.story_usage ? (
                    <div className="mt-1 line-clamp-2 leading-relaxed text-book-text-sub">
                      {item.story_usage}
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-small text-book-text-muted">本章尚未同步出物体档案。</p>
          )}
        </section>

        {stateRows.length > 0 ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">角色状态</h3>
            <ul className="divide-y divide-book-border/35">
              {stateRows.map(({ name, data }) => {
                const changes = analysisTextList(data.changes, { limit: 6 });
                const status = firstText(data, ["status"]);
                const context = [
                  firstText(data, ["location", "current_location"]),
                  firstText(data, ["emotional_state", "emotion"]),
                ]
                  .filter(Boolean)
                  .join(" · ");
                return (
                  <li
                    key={name}
                    className="grid grid-cols-[72px_minmax(0,1fr)] gap-2 py-2 first:pt-0 last:pb-0"
                  >
                    <span className="truncate font-medium text-book-text-main" title={name}>
                      {name}
                    </span>
                    <div className="min-w-0">
                      {status && <div className="leading-relaxed">{status}</div>}
                      {context ? (
                        <div className="mt-0.5 text-small text-book-text-muted">{context}</div>
                      ) : null}
                      {changes.length > 0 && (
                        <div className="mt-1 text-small leading-relaxed text-book-text-sub">
                          变化：{changes.join("；")}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          </section>
        ) : null}

        {hasForeshadowing ? (
          <section className={ANALYSIS_CARD_CLASS}>
            <h3 className="mb-2 text-small font-semibold text-book-text-main">伏笔</h3>
            <div className="space-y-3">
              {planted.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">埋设</div>
                  <ul className="divide-y divide-book-border/35">
                    {planted.map((item, idx) => {
                      const desc = firstText(item, ["description", "content"]);
                      if (!desc) return null;
                      const tags = [
                        knownLabel(
                          FORESHADOWING_CATEGORY_LABELS,
                          firstText(item, ["category", "type"]),
                        ),
                        chapterEventImportanceLabel(
                          firstText(item, ["priority", "importance"]),
                        ),
                      ].filter((tag): tag is string => tag != null);
                      return (
                        <li key={`${desc}-${idx}`} className="py-2 first:pt-0 last:pb-0">
                          <div className="leading-relaxed text-book-text-main">{desc}</div>
                          {tags.length > 0 ? (
                            <div className="mt-0.5 text-small text-book-text-muted">
                              {tags.join(" · ")}
                            </div>
                          ) : null}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {resolved.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">回收</div>
                  <ul className="divide-y divide-book-border/35">
                    {resolved.map((item, idx) => {
                      const desc = firstText(item, ["resolution", "description", "content"]);
                      if (!desc) return null;
                      return (
                        <li key={idx} className="py-2 leading-relaxed first:pt-0 last:pb-0">
                          {desc}
                        </li>
                      );
                    })}
                  </ul>
                </div>
              )}
              {tensions.length > 0 && (
                <div>
                  <div className="mb-1 text-small text-book-text-muted">悬念</div>
                  <ul className="divide-y divide-book-border/35">
                    {tensions.map((item) => (
                      <li key={item} className="py-2 leading-relaxed first:pt-0 last:pb-0">
                        {item}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </section>
        ) : null}
      </div>
    </div>
  );
};
