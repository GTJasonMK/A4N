import React from "react";
import type {
  ProfileChapterState,
  Snapshot,
  SnapshotDelta,
} from "../../api/character";
import {
  attributeKeyLabel,
  behaviorTypeLabel,
  categoryLabel,
  compactJson,
  decisionLabel,
  isPlainRecord,
  listPreview,
  nestedRecord,
  operationLabel,
  recordArray,
  recordNumber,
  recordString,
} from "./manageViewUtils";
import { formatUiDateTime } from "../../utils/dateTime";

const snapshotPayload = (
  snapshot: Snapshot | null,
): Record<string, unknown> | null => {
  if (!snapshot || !isPlainRecord(snapshot.data)) return null;
  return snapshot.data;
};

const snapshotCountRecordText = (
  counts: Record<string, unknown> | null,
): string | null => {
  const entries = [
    ["显性", recordNumber(counts, "explicit")],
    ["隐性", recordNumber(counts, "implicit")],
    ["社会", recordNumber(counts, "social")],
  ]
    .filter((entry): entry is [string, number] => entry[1] != null)
    .map(([label, count]) => `${label} ${count}`);

  return entries.length > 0 ? entries.join(" / ") : null;
};

const snapshotAttributeCountText = (
  payload: Record<string, unknown> | null,
): string | null => {
  const counts = nestedRecord(payload, "attribute_counts");
  return snapshotCountRecordText(counts);
};

const SnapshotItemList: React.FC<{
  title: string;
  items: unknown[];
  renderItem: (item: unknown, index: number) => React.ReactNode;
}> = ({ title, items, renderItem }) => {
  if (items.length === 0) return null;
  const visibleItems = items.slice(0, 4);
  return (
    <section className="border-t border-book-border/35 py-2.5 first:border-t-0">
      <div className="mb-1.5 text-[10px] font-semibold text-book-text-main">
        {title} · {items.length} 条
      </div>
      <div className="divide-y divide-book-border/30">
        {visibleItems.map((item, index) => (
          <div
            key={index}
            className="py-1.5 text-[10px] leading-relaxed text-book-text-muted"
          >
            {renderItem(item, index)}
          </div>
        ))}
      </div>
      {items.length > visibleItems.length && (
        <div className="mt-1 text-[10px] text-book-text-muted">
          另有 {items.length - visibleItems.length} 条，本页仅展示最近内容。
        </div>
      )}
    </section>
  );
};

const SnapshotDiagnosticValue: React.FC<{ value: unknown }> = ({ value }) => (
  <details className="mt-1 text-book-text-muted">
    <summary className="w-fit cursor-pointer rounded-lg px-1.5 py-0.5 transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main">
      诊断信息
    </summary>
    <div className="mt-1 break-words rounded-lg bg-book-bg-tertiary/45 px-2 py-1.5 font-mono">
      {compactJson(value)}
    </div>
  </details>
);

const SnapshotValue: React.FC<{ value: unknown }> = ({ value }) => {
  if (value == null)
    return <span className="text-book-text-subtle">未记录</span>;
  if (
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return <>{String(value)}</>;
  }
  if (Array.isArray(value)) {
    const preview = listPreview(value);
    if (preview) return <>{preview}</>;
  }
  return <SnapshotDiagnosticValue value={value} />;
};

const renderSnapshotChange = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return <SnapshotDiagnosticValue value={item} />;
  const category = recordString(item, "category");
  const key = recordString(item, "key");
  const operation = recordString(item, "operation");
  if (!category || !key || !operation)
    return <SnapshotDiagnosticValue value={item} />;

  const description = recordString(item, "change_description");
  const cause =
    recordString(item, "event_cause") || recordString(item, "cause");
  const evidence = recordString(item, "evidence");
  return (
    <>
      <div
        className="font-medium text-book-text-main"
        title={`${category}.${key} · ${operation}`}
      >
        {categoryLabel(category)} · {attributeKeyLabel(key)} ·{" "}
        {operationLabel(operation)}
      </div>
      {description && (
        <div className="mt-0.5 text-book-text-sub">{description}</div>
      )}
      {cause && <div className="mt-0.5">触发：{cause}</div>}
      {evidence && <div className="mt-0.5">证据：{evidence}</div>}
    </>
  );
};

const renderSnapshotBehavior = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return <SnapshotDiagnosticValue value={item} />;
  const behaviorType = recordString(item, "behavior_type");
  const description = recordString(item, "behavior_description");
  if (!behaviorType && !description)
    return <SnapshotDiagnosticValue value={item} />;

  const originalText = recordString(item, "original_text");
  const tags = listPreview(item.behavior_tags);
  return (
    <>
      {behaviorType && (
        <div className="font-medium text-book-text-main" title={behaviorType}>
          {behaviorTypeLabel(behaviorType)}
        </div>
      )}
      {description && (
        <div className="mt-0.5 text-book-text-sub">{description}</div>
      )}
      {tags && <div className="mt-0.5">标签：{tags}</div>}
      {originalText && <div className="mt-0.5">原文：{originalText}</div>}
    </>
  );
};

const renderSnapshotDeletionCandidate = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return <SnapshotDiagnosticValue value={item} />;
  const category = recordString(item, "category");
  const key = recordString(item, "key");
  if (!category || !key) return <SnapshotDiagnosticValue value={item} />;

  const reason =
    recordString(item, "reason") || recordString(item, "mark_reason");
  const evidence = recordString(item, "evidence");
  return (
    <>
      <div
        className="font-medium text-book-text-main"
        title={`${category}.${key}`}
      >
        {categoryLabel(category)} · {attributeKeyLabel(key)}
      </div>
      {reason && <div className="mt-0.5">原因：{reason}</div>}
      {evidence && <div className="mt-0.5">证据：{evidence}</div>}
    </>
  );
};

const renderSnapshotGenericItem = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return <SnapshotDiagnosticValue value={item} />;
  const attributeKey =
    recordString(item, "attribute_key") || recordString(item, "key");
  const decision = recordString(item, "decision");
  const reason =
    recordString(item, "reasoning") ||
    recordString(item, "reason") ||
    recordString(item, "evidence_summary");
  if (!attributeKey && !decision && !reason)
    return <SnapshotDiagnosticValue value={item} />;

  return (
    <>
      {(attributeKey || decision) && (
        <div
          className="font-medium text-book-text-main"
          title={[attributeKey, decision].filter(Boolean).join(" · ")}
        >
          {attributeKey ? attributeKeyLabel(attributeKey) : null}
          {attributeKey && decision ? " · " : null}
          {decision ? decisionLabel(decision) : null}
        </div>
      )}
      {reason && <div className="mt-0.5 text-book-text-sub">{reason}</div>}
    </>
  );
};

const SnapshotAttributeGroup: React.FC<{ label: string; value: unknown }> = ({
  label,
  value,
}) => {
  const entries = isPlainRecord(value) ? Object.entries(value) : [];
  if (entries.length === 0) return null;
  return (
    <section className="px-2.5 py-2.5">
      <div className="mb-1 text-[10px] font-semibold text-book-text-main">
        {label}
      </div>
      <dl className="divide-y divide-book-border/35">
        {entries.map(([key, entryValue]) => (
          <div
            key={key}
            className="grid gap-0.5 py-1 text-[10px] sm:grid-cols-[112px_minmax(0,1fr)] sm:gap-2"
          >
            <dt
              className="break-words font-medium text-book-text-muted"
              title={key}
            >
              {attributeKeyLabel(key)}
            </dt>
            <dd className="min-w-0 break-words text-book-text-sub">
              <SnapshotValue value={entryValue} />
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
};

export const ProfileChapterStateDetail: React.FC<{
  state: ProfileChapterState;
}> = ({ state }) => {
  const totalAttributes =
    state.attribute_counts.explicit +
    state.attribute_counts.implicit +
    state.attribute_counts.social;
  const metadata = [
    totalAttributes > 0
      ? `显性 ${state.attribute_counts.explicit} · 隐性 ${state.attribute_counts.implicit} · 社会 ${state.attribute_counts.social}`
      : null,
    state.changes_in_chapter > 0 ? `变化 ${state.changes_in_chapter}` : null,
    state.behaviors_in_chapter > 0
      ? `行为 ${state.behaviors_in_chapter}`
      : null,
    state.current_location ? `位置 ${state.current_location}` : null,
  ].filter(Boolean);
  return (
    <section className="a4n-flat-surface overflow-hidden text-[11px]">
      <div className="px-3 py-3">
        <div className="font-semibold text-book-text-main">
          第 {state.chapter_number} 章
          {state.created_at ? ` · ${formatUiDateTime(state.created_at)}` : ""}
        </div>
        {metadata.length > 0 ? (
          <div className="mt-1 text-[10px] leading-relaxed text-book-text-muted">
            {metadata.join(" · ")}
          </div>
        ) : null}
        {state.state_summary ? (
          <p className="mt-2 leading-relaxed text-book-text-sub">
            {state.state_summary}
          </p>
        ) : null}
      </div>
      <details className="border-t border-book-border/35">
        <summary className="cursor-pointer px-3 py-2.5 text-[10px] font-medium text-book-primary hover:bg-book-bg-tertiary/45">
          查看属性详情
        </summary>
        <div className="divide-y divide-book-border/35 border-t border-book-border/30 bg-book-bg/35 md:grid md:grid-cols-3 md:divide-x md:divide-y-0">
          <SnapshotAttributeGroup
            label="显性属性"
            value={state.explicit_attributes}
          />
          <SnapshotAttributeGroup
            label="隐性属性"
            value={state.implicit_attributes}
          />
          <SnapshotAttributeGroup
            label="社会属性"
            value={state.social_attributes}
          />
        </div>
      </details>
    </section>
  );
};

export const SnapshotDetail: React.FC<{ snapshot: Snapshot }> = ({
  snapshot,
}) => {
  const payload = snapshotPayload(snapshot);
  const summary = recordString(payload, "state_summary");
  const location = recordString(payload, "current_location");
  const goal = recordString(payload, "current_goal");
  const emotion = recordString(payload, "emotional_state");
  const chapter = recordNumber(payload, "synced_chapter");
  const changes = recordArray(payload, "changes");
  const behaviors = recordArray(payload, "behaviors");
  const deletionCandidates = recordArray(payload, "deletion_candidates");
  const classificationSuggestions = recordArray(
    payload,
    "classification_suggestions",
  );
  const implicitReviewDecisions = recordArray(
    payload,
    "implicit_review_decisions",
  );
  const attributeCountText = snapshotAttributeCountText(payload);
  const changesInChapter = recordNumber(payload, "changes_in_chapter");
  const behaviorsInChapter = recordNumber(payload, "behaviors_in_chapter");
  const metadata = [
    attributeCountText,
    changesInChapter != null && changesInChapter > 0
      ? `变化 ${changesInChapter}`
      : null,
    behaviorsInChapter != null && behaviorsInChapter > 0
      ? `行为 ${behaviorsInChapter}`
      : null,
    location ? `位置 ${location}` : null,
  ].filter(Boolean);

  return (
    <section className="a4n-flat-surface overflow-hidden text-[11px]">
      <div className="px-3 py-3">
        {(chapter != null || snapshot.created_at) && (
          <div className="font-semibold text-book-text-main">
            {chapter != null ? `第 ${chapter} 章` : "角色快照"}
            {snapshot.created_at
              ? ` · ${formatUiDateTime(snapshot.created_at)}`
              : ""}
          </div>
        )}
        {metadata.length > 0 ? (
          <div className="mt-1 text-[10px] leading-relaxed text-book-text-muted">
            {metadata.join(" · ")}
          </div>
        ) : null}
        {summary ? (
          <p className="mt-2 text-[10px] leading-relaxed text-book-text-sub">
            {summary}
          </p>
        ) : null}
        {(goal || emotion) && (
          <div className="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-[10px] text-book-text-muted">
            {goal && <span>目标：{goal}</span>}
            {emotion && <span>情绪：{emotion}</span>}
          </div>
        )}
      </div>
      {changes.length > 0 ||
      behaviors.length > 0 ||
      deletionCandidates.length > 0 ||
      classificationSuggestions.length > 0 ||
      implicitReviewDecisions.length > 0 ? (
        <div className="border-t border-book-border/35 px-3">
          <SnapshotItemList
            title="属性变化"
            items={changes}
            renderItem={renderSnapshotChange}
          />
          <SnapshotItemList
            title="行为证据"
            items={behaviors}
            renderItem={renderSnapshotBehavior}
          />
          <SnapshotItemList
            title="删除候选"
            items={deletionCandidates}
            renderItem={renderSnapshotDeletionCandidate}
          />
          <SnapshotItemList
            title="隐性复审"
            items={[...classificationSuggestions, ...implicitReviewDecisions]}
            renderItem={renderSnapshotGenericItem}
          />
        </div>
      ) : null}
    </section>
  );
};

const deltaEntries = (
  categoryDelta: Record<string, unknown> | null,
  operation: string,
): Array<[string, unknown]> => {
  const value = categoryDelta?.[operation];
  return isPlainRecord(value) ? Object.entries(value) : [];
};

const structuredDeltaHasChanges = (
  attributes: Record<string, unknown> | null,
): boolean =>
  ["explicit", "implicit", "social"].some((category) => {
    const categoryDelta = nestedRecord(attributes, category);
    return ["added", "modified", "deleted"].some(
      (operation) => deltaEntries(categoryDelta, operation).length > 0,
    );
  });

export const SnapshotDeltaView: React.FC<{ delta: SnapshotDelta }> = ({
  delta,
}) => {
  const payload = isPlainRecord(delta.delta) ? delta.delta : null;
  const attributes = nestedRecord(payload, "attributes");
  const summary = nestedRecord(payload, "summary");
  const hasStructuredChanges = structuredDeltaHasChanges(attributes);
  const fromChapter = recordNumber(summary, "from_chapter");
  const toChapter = recordNumber(summary, "to_chapter");
  const fromSummary = recordString(summary, "from_state_summary");
  const toSummary = recordString(summary, "to_state_summary");

  return (
    <section className="overflow-hidden rounded-2xl border border-warning/25 bg-warning-bg/25 text-[11px]">
      <div className="px-3 py-3">
        <div className="font-semibold text-book-text-main">快照变化</div>
        {fromChapter != null && toChapter != null ? (
          <div className="mt-1 text-[10px] text-book-text-muted">
            第 {fromChapter} 章 → 第 {toChapter} 章
          </div>
        ) : null}
      </div>
      {(fromSummary || toSummary) && (
        <div className="grid border-t border-warning/20 text-[10px] md:grid-cols-2 md:divide-x md:divide-book-border/30">
          {fromSummary && (
            <div className="px-3 py-2.5 text-book-text-muted">
              <span className="font-medium text-book-text-main">原状态</span>
              <p className="mt-1 leading-relaxed">{fromSummary}</p>
            </div>
          )}
          {toSummary && (
            <div className="border-t border-book-border/30 px-3 py-2.5 text-book-text-muted md:border-t-0">
              <span className="font-medium text-book-text-main">新状态</span>
              <p className="mt-1 leading-relaxed">{toSummary}</p>
            </div>
          )}
        </div>
      )}
      {hasStructuredChanges ? (
        <div className="divide-y divide-book-border/35 border-t border-warning/20 px-3">
          {["explicit", "implicit", "social"].map((category) => (
            <SnapshotDeltaCategory
              key={category}
              label={categoryLabel(category)}
              delta={nestedRecord(attributes, category)}
            />
          ))}
        </div>
      ) : (
        <div className="border-t border-warning/20 px-3 py-3 text-[10px] text-book-text-muted">
          未检测到属性变化
        </div>
      )}
    </section>
  );
};

const SnapshotDeltaCategory: React.FC<{
  label: string;
  delta: Record<string, unknown> | null;
}> = ({ label, delta }) => {
  const operations = ["added", "modified", "deleted"]
    .map((operation) => ({
      operation,
      entries: deltaEntries(delta, operation),
    }))
    .filter((group) => group.entries.length > 0);
  if (operations.length === 0) return null;

  return (
    <section className="py-2.5">
      <div className="mb-1.5 text-[10px] font-semibold text-book-text-main">
        {label}
      </div>
      <div className="space-y-2">
        {operations.map((group) => (
          <div key={group.operation} className="space-y-1">
            <div className="text-[10px] font-medium text-book-text-muted">
              {operationLabel(group.operation)}
            </div>
            <div className="divide-y divide-book-border/30 rounded-xl bg-book-bg-paper/45 px-2.5">
              {group.entries.map(([key, value]) => (
                <SnapshotDeltaEntry
                  key={`${group.operation}-${key}`}
                  operation={group.operation}
                  attrKey={key}
                  value={value}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
};

const SnapshotDeltaEntry: React.FC<{
  operation: string;
  attrKey: string;
  value: unknown;
}> = ({ operation, attrKey, value }) => {
  const changed =
    operation === "modified" && isPlainRecord(value) ? value : null;
  return (
    <div className="py-2 text-[10px]">
      <div className="font-medium text-book-text-main" title={attrKey}>
        {attributeKeyLabel(attrKey)}
      </div>
      {changed ? (
        <div className="mt-0.5 grid gap-1 text-book-text-muted">
          <span className="flex gap-1">
            旧：
            <SnapshotValue value={changed.from} />
          </span>
          <span className="flex gap-1">
            新：
            <SnapshotValue value={changed.to} />
          </span>
        </div>
      ) : (
        <div className="mt-0.5 text-book-text-muted">
          <SnapshotValue value={value} />
        </div>
      )}
    </div>
  );
};
