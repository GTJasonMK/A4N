import React from "react";
import type { ProfileChapterState, Snapshot, SnapshotDelta } from "../../api/character";
import {
  CATEGORY_LABELS,
  OPERATION_LABELS,
  compactJson,
  isPlainRecord,
  listPreview,
  nestedRecord,
  recordArray,
  recordNumber,
  recordString,
} from "./manageViewUtils";

const snapshotPayload = (snapshot: Snapshot | null): Record<string, unknown> | null => {
  if (!snapshot || !isPlainRecord(snapshot.data)) return null;
  return snapshot.data;
};

const snapshotCountRecordText = (counts: Record<string, unknown> | null): string | null => {
  const entries = [
    ["显性", recordNumber(counts, "explicit")],
    ["隐性", recordNumber(counts, "implicit")],
    ["社会", recordNumber(counts, "social")],
  ]
    .filter((entry): entry is [string, number] => entry[1] != null)
    .map(([label, count]) => `${label} ${count}`);

  return entries.length > 0 ? entries.join(" / ") : null;
};

const snapshotAttributeCountText = (payload: Record<string, unknown> | null): string | null => {
  const counts = nestedRecord(payload, "attribute_counts");
  return snapshotCountRecordText(counts);
};

const SnapshotMetric: React.FC<{ label: string; value: string }> = ({ label, value }) => (
  <div className="rounded bg-book-bg-tertiary/40 px-2 py-1">
    <div className="text-[9px] text-book-text-muted">{label}</div>
    <div className="truncate text-book-text-sub">{value}</div>
  </div>
);

const SnapshotItemList: React.FC<{
  title: string;
  items: unknown[];
  renderItem: (item: unknown, index: number) => React.ReactNode;
}> = ({ title, items, renderItem }) => {
  if (items.length === 0) return null;
  const visibleItems = items.slice(0, 4);
  return (
    <div className="rounded border border-book-border/35 bg-book-bg-paper/40 px-2 py-1.5">
      <div className="mb-1 flex items-center gap-1.5">
        <span className="text-[10px] font-semibold text-book-text-main">{title}</span>
        <span className="pyqt-pill !text-[10px]">{items.length} 条</span>
      </div>
      <div className="space-y-1">
        {visibleItems.map((item, index) => (
          <div
            key={index}
            className="rounded bg-book-bg-tertiary/35 px-2 py-1 text-[10px] leading-relaxed text-book-text-muted"
          >
            {renderItem(item, index)}
          </div>
        ))}
      </div>
      {items.length > visibleItems.length && (
        <div className="mt-1 text-[10px] text-book-text-muted">
          另有 {items.length - visibleItems.length} 条，展开原始快照数据查看。
        </div>
      )}
    </div>
  );
};

const renderSnapshotChange = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return compactJson(item);
  const category = recordString(item, "category");
  const key = recordString(item, "key");
  const operation = recordString(item, "operation");
  if (!category || !key || !operation) return compactJson(item);

  const description = recordString(item, "change_description");
  const cause = recordString(item, "event_cause") || recordString(item, "cause");
  const evidence = recordString(item, "evidence");
  return (
    <>
      <div className="font-mono text-book-text-main">
        {CATEGORY_LABELS[category] ?? category}.{key} · {OPERATION_LABELS[operation] ?? operation}
      </div>
      {description && <div className="mt-0.5 text-book-text-sub">{description}</div>}
      {cause && <div className="mt-0.5">触发：{cause}</div>}
      {evidence && <div className="mt-0.5">证据：{evidence}</div>}
    </>
  );
};

const renderSnapshotBehavior = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return compactJson(item);
  const behaviorType = recordString(item, "behavior_type");
  const description = recordString(item, "behavior_description");
  if (!behaviorType && !description) return compactJson(item);

  const originalText = recordString(item, "original_text");
  const tags = listPreview(item.behavior_tags);
  return (
    <>
      {behaviorType && <div className="font-medium text-book-text-main">{behaviorType}</div>}
      {description && <div className="mt-0.5 text-book-text-sub">{description}</div>}
      {tags && <div className="mt-0.5">标签：{tags}</div>}
      {originalText && <div className="mt-0.5">原文：{originalText}</div>}
    </>
  );
};

const renderSnapshotDeletionCandidate = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return compactJson(item);
  const category = recordString(item, "category");
  const key = recordString(item, "key");
  if (!category || !key) return compactJson(item);

  const reason = recordString(item, "reason") || recordString(item, "mark_reason");
  const evidence = recordString(item, "evidence");
  return (
    <>
      <div className="font-mono text-book-text-main">
        {CATEGORY_LABELS[category] ?? category}.{key}
      </div>
      {reason && <div className="mt-0.5">原因：{reason}</div>}
      {evidence && <div className="mt-0.5">证据：{evidence}</div>}
    </>
  );
};

const renderSnapshotGenericItem = (item: unknown): React.ReactNode => {
  if (!isPlainRecord(item)) return compactJson(item);
  const title =
    recordString(item, "attribute_key") ||
    recordString(item, "key") ||
    recordString(item, "decision");
  const reason =
    recordString(item, "reasoning") ||
    recordString(item, "reason") ||
    recordString(item, "evidence_summary");
  if (!title && !reason) return compactJson(item);

  return (
    <>
      {title && <div className="font-mono text-book-text-main">{title}</div>}
      {reason && <div className="mt-0.5 text-book-text-sub">{reason}</div>}
    </>
  );
};

export const ProfileChapterStateDetail: React.FC<{ state: ProfileChapterState }> = ({
  state,
}) => {
  const attributes = {
    explicit_attributes: state.explicit_attributes,
    implicit_attributes: state.implicit_attributes,
    social_attributes: state.social_attributes,
  };
  return (
    <div className="a4n-page-surface p-2 text-[11px]">
      <div className="flex flex-wrap items-center gap-2 mb-2">
        <span className="font-semibold text-book-text-main">第{state.chapter_number}章状态</span>
        <span className="font-mono text-book-text-muted text-[10px]">
          {state.snapshot_hash.slice(0, 12)}
        </span>
        {state.created_at && (
          <span className="text-book-text-muted text-[10px]">{state.created_at}</span>
        )}
      </div>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-1.5 mb-2">
        <SnapshotMetric
          label="属性"
          value={`显性 ${state.attribute_counts.explicit} / 隐性 ${state.attribute_counts.implicit} / 社会 ${state.attribute_counts.social}`}
        />
        <SnapshotMetric label="变化" value={String(state.changes_in_chapter)} />
        <SnapshotMetric label="行为" value={String(state.behaviors_in_chapter)} />
        {state.current_location && (
          <SnapshotMetric label="位置" value={state.current_location} />
        )}
      </div>
      {state.state_summary && (
        <div className="mb-2 text-book-text-sub leading-relaxed">{state.state_summary}</div>
      )}
      <pre className="max-h-52 overflow-auto custom-scrollbar rounded bg-book-bg-tertiary/45 p-2 text-[10px] leading-relaxed">
        {JSON.stringify(attributes, null, 2)}
      </pre>
    </div>
  );
};

export const SnapshotDetail: React.FC<{ snapshot: Snapshot }> = ({ snapshot }) => {
  const payload = snapshotPayload(snapshot);
  const summary = recordString(payload, "state_summary");
  const location = recordString(payload, "current_location");
  const goal = recordString(payload, "current_goal");
  const emotion = recordString(payload, "emotional_state");
  const chapter = recordNumber(payload, "synced_chapter");
  const changes = recordArray(payload, "changes");
  const behaviors = recordArray(payload, "behaviors");
  const deletionCandidates = recordArray(payload, "deletion_candidates");
  const classificationSuggestions = recordArray(payload, "classification_suggestions");
  const implicitReviewDecisions = recordArray(payload, "implicit_review_decisions");
  const attributeCountText = snapshotAttributeCountText(payload);
  const changesInChapter = recordNumber(payload, "changes_in_chapter");
  const behaviorsInChapter = recordNumber(payload, "behaviors_in_chapter");

  return (
    <div className="a4n-page-surface p-2 text-[11px]">
      <div className="mb-1 flex flex-wrap items-center gap-1.5">
        <span className="pyqt-pill !text-[10px]">快照详情</span>
        {chapter != null && <span className="pyqt-pill !text-[10px]">第{chapter}章</span>}
        <span className="font-mono text-book-text-muted text-[10px]">
          {snapshot.content_hash.slice(0, 12)}
        </span>
        {snapshot.created_at && (
          <span className="ml-auto text-[10px] text-book-text-muted">
            {snapshot.created_at}
          </span>
        )}
      </div>
      <div className="grid gap-1 text-[10px] md:grid-cols-4">
        {attributeCountText && <SnapshotMetric label="属性" value={attributeCountText} />}
        {changesInChapter != null && (
          <SnapshotMetric label="变化" value={String(changesInChapter)} />
        )}
        {behaviorsInChapter != null && (
          <SnapshotMetric label="行为" value={String(behaviorsInChapter)} />
        )}
        {location && <SnapshotMetric label="当前位置" value={location} />}
      </div>
      {summary && (
        <div className="mt-2 rounded bg-book-bg-tertiary/40 px-2 py-1 text-[10px] leading-relaxed text-book-text-sub">
          {summary}
        </div>
      )}
      {(goal || emotion) && (
        <div className="mt-1 flex flex-wrap gap-2 text-[10px] text-book-text-muted">
          {goal && <span>目标：{goal}</span>}
          {emotion && <span>情绪：{emotion}</span>}
        </div>
      )}
      <div className="mt-2 grid gap-2 md:grid-cols-2">
        <SnapshotItemList title="属性变化" items={changes} renderItem={renderSnapshotChange} />
        <SnapshotItemList title="行为证据" items={behaviors} renderItem={renderSnapshotBehavior} />
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
      <details className="mt-2">
        <summary className="cursor-pointer text-[10px] text-book-primary">原始快照数据</summary>
        <pre className="mt-1 max-h-40 overflow-y-auto whitespace-pre-wrap break-words rounded bg-book-bg-tertiary/40 p-2 font-mono text-[10px] text-book-text-sub custom-scrollbar">
          {JSON.stringify(snapshot, null, 2)}
        </pre>
      </details>
    </div>
  );
};

const DELTA_CATEGORY_LABELS: Record<string, string> = {
  explicit: "显性",
  implicit: "隐性",
  social: "社会",
};

const DELTA_OPERATION_LABELS: Record<string, string> = {
  added: "新增",
  modified: "修改",
  deleted: "删除",
};

const deltaEntries = (
  categoryDelta: Record<string, unknown> | null,
  operation: string,
): Array<[string, unknown]> => {
  const value = categoryDelta?.[operation];
  return isPlainRecord(value) ? Object.entries(value) : [];
};

const structuredDeltaHasChanges = (attributes: Record<string, unknown> | null): boolean =>
  ["explicit", "implicit", "social"].some((category) => {
    const categoryDelta = nestedRecord(attributes, category);
    return ["added", "modified", "deleted"].some(
      (operation) => deltaEntries(categoryDelta, operation).length > 0,
    );
  });

export const SnapshotDeltaView: React.FC<{ delta: SnapshotDelta }> = ({ delta }) => {
  const payload = isPlainRecord(delta.delta) ? delta.delta : null;
  const attributes = nestedRecord(payload, "attributes");
  const summary = nestedRecord(payload, "summary");
  const hasStructuredChanges = structuredDeltaHasChanges(attributes);
  const fromChapter = recordNumber(summary, "from_chapter");
  const toChapter = recordNumber(summary, "to_chapter");
  const fromSummary = recordString(summary, "from_state_summary");
  const toSummary = recordString(summary, "to_state_summary");

  return (
    <div className="rounded border border-warning/30 bg-warning-bg/40 p-2 text-[11px]">
      <div className="mb-1 flex flex-wrap items-center gap-1.5">
        <span className="pyqt-pill !border-warning/40 !text-[10px] !text-warning">
          快照差异
        </span>
        <span className="font-mono text-[10px] text-book-text-muted">
          {delta.from_hash.slice(0, 8)} → {delta.to_hash.slice(0, 8)}
        </span>
        {fromChapter != null && toChapter != null && (
          <span className="pyqt-pill !text-[10px]">
            第{fromChapter}章 → 第{toChapter}章
          </span>
        )}
      </div>
      {(fromSummary || toSummary) && (
        <div className="mb-2 grid gap-1 text-[10px] md:grid-cols-2">
          {fromSummary && (
            <div className="rounded bg-book-bg-tertiary/40 px-2 py-1 text-book-text-muted">
              原状态：{fromSummary}
            </div>
          )}
          {toSummary && (
            <div className="rounded bg-book-bg-tertiary/40 px-2 py-1 text-book-text-muted">
              新状态：{toSummary}
            </div>
          )}
        </div>
      )}
      {hasStructuredChanges ? (
        <div className="space-y-2">
          {["explicit", "implicit", "social"].map((category) => (
            <SnapshotDeltaCategory
              key={category}
              label={DELTA_CATEGORY_LABELS[category]}
              delta={nestedRecord(attributes, category)}
            />
          ))}
        </div>
      ) : (
        <div className="rounded bg-book-bg-tertiary/40 px-2 py-1 text-[10px] text-book-text-muted">
          未检测到属性变化
        </div>
      )}
      <details className="mt-2">
        <summary className="cursor-pointer text-[10px] text-book-primary">原始差异数据</summary>
        <pre className="mt-1 max-h-40 overflow-y-auto whitespace-pre-wrap break-words rounded bg-book-bg-tertiary/40 p-2 font-mono text-[10px] text-book-text-sub custom-scrollbar">
          {JSON.stringify(delta.delta, null, 2)}
        </pre>
      </details>
    </div>
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
    <div className="rounded border border-book-border/35 bg-book-bg-paper/45 px-2 py-1.5">
      <div className="mb-1 text-[10px] font-semibold text-book-text-main">{label}</div>
      <div className="space-y-1">
        {operations.map((group) => (
          <div key={group.operation} className="space-y-1">
            <span className="pyqt-pill !text-[10px]">
              {DELTA_OPERATION_LABELS[group.operation] ?? group.operation}
            </span>
            <div className="grid gap-1 md:grid-cols-2">
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
    </div>
  );
};

const SnapshotDeltaEntry: React.FC<{
  operation: string;
  attrKey: string;
  value: unknown;
}> = ({ operation, attrKey, value }) => {
  const changed = operation === "modified" && isPlainRecord(value) ? value : null;
  return (
    <div className="rounded bg-book-bg-tertiary/40 px-2 py-1 text-[10px]">
      <div className="font-mono text-book-text-main">{attrKey}</div>
      {changed ? (
        <div className="mt-0.5 grid gap-1 text-book-text-muted">
          <span>旧：{compactJson(changed.from)}</span>
          <span>新：{compactJson(changed.to)}</span>
        </div>
      ) : (
        <div className="mt-0.5 font-mono text-book-text-muted">{compactJson(value)}</div>
      )}
    </div>
  );
};
