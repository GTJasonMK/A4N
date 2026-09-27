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
import { formatDateTime, useI18n, type MessageTranslator } from "../../i18n";

const snapshotPayload = (
  snapshot: Snapshot | null,
): Record<string, unknown> | null => {
  if (!snapshot || !isPlainRecord(snapshot.data)) return null;
  return snapshot.data;
};

const snapshotCountRecordText = (
  counts: Record<string, unknown> | null,
  t: MessageTranslator,
): string | null => {
  const entries = [
    [t("characterDetail.labels.explicit"), recordNumber(counts, "explicit")],
    [t("characterDetail.labels.implicit"), recordNumber(counts, "implicit")],
    [t("characterDetail.labels.social"), recordNumber(counts, "social")],
  ]
    .filter((entry): entry is [string, number] => entry[1] != null)
    .map(([label, count]) => `${label} ${count}`);

  return entries.length > 0 ? entries.join(" / ") : null;
};

const snapshotAttributeCountText = (
  payload: Record<string, unknown> | null,
  t: MessageTranslator,
): string | null => {
  const counts = nestedRecord(payload, "attribute_counts");
  return snapshotCountRecordText(counts, t);
};

const SnapshotItemList: React.FC<{
  title: string;
  items: unknown[];
  renderItem: (item: unknown, index: number) => React.ReactNode;
}> = ({ title, items, renderItem }) => {
  const { t } = useI18n();
  if (items.length === 0) return null;
  const visibleItems = items.slice(0, 4);
  return (
    <section className="border-t border-book-border/35 py-2.5 first:border-t-0">
      <div className="mb-1.5 text-caption font-semibold text-book-text-main">
        {title} · {t("characterDetail.behavior.count", { count: items.length })}
      </div>
      <div className="divide-y divide-book-border/30">
        {visibleItems.map((item, index) => (
          <div
            key={index}
            className="py-1.5 text-caption leading-relaxed text-book-text-muted"
          >
            {renderItem(item, index)}
          </div>
        ))}
      </div>
      {items.length > visibleItems.length && (
        <div className="mt-1 text-caption text-book-text-muted">
          {t("characterDetail.snapshot.moreItems", { count: items.length - visibleItems.length })}
        </div>
      )}
    </section>
  );
};

const SnapshotDiagnosticValue: React.FC<{ value: unknown }> = ({ value }) => {
  const { t } = useI18n();
  return (
    <details className="mt-1 text-book-text-muted">
      <summary className="w-fit cursor-pointer rounded-lg px-1.5 py-0.5 transition-colors hover:bg-book-bg-tertiary/60 hover:text-book-text-main">
        {t("characterDetail.diagnostics")}
      </summary>
      <div className="mt-1 break-words rounded-lg bg-book-bg-tertiary/45 px-2 py-1.5 font-mono">
        {compactJson(value)}
      </div>
    </details>
  );
};

const SnapshotValue: React.FC<{ value: unknown }> = ({ value }) => {
  const { t } = useI18n();
  if (value == null)
    return <span className="text-book-text-subtle">{t("characterDetail.notRecorded")}</span>;
  if (
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return <>{String(value)}</>;
  }
  if (Array.isArray(value)) {
    const preview = listPreview(value, t);
    if (preview) return <>{preview}</>;
  }
  return <SnapshotDiagnosticValue value={value} />;
};

const renderSnapshotChange = (item: unknown, t: MessageTranslator): React.ReactNode => {
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
        {categoryLabel(category, t)} · {attributeKeyLabel(key, t)} ·{" "}
        {operationLabel(operation, t)}
      </div>
      {description && (
        <div className="mt-0.5 text-book-text-sub">{description}</div>
      )}
      {cause && <div className="mt-0.5">{t("characterDetail.snapshot.trigger")}{cause}</div>}
      {evidence && <div className="mt-0.5">{t("characterDetail.snapshot.evidence")}{evidence}</div>}
    </>
  );
};

const renderSnapshotBehavior = (item: unknown, t: MessageTranslator): React.ReactNode => {
  if (!isPlainRecord(item)) return <SnapshotDiagnosticValue value={item} />;
  const behaviorType = recordString(item, "behavior_type");
  const description = recordString(item, "behavior_description");
  if (!behaviorType && !description)
    return <SnapshotDiagnosticValue value={item} />;

  const originalText = recordString(item, "original_text");
  const tags = listPreview(item.behavior_tags, t);
  return (
    <>
      {behaviorType && (
        <div className="font-medium text-book-text-main" title={behaviorType}>
          {behaviorTypeLabel(behaviorType, t)}
        </div>
      )}
      {description && (
        <div className="mt-0.5 text-book-text-sub">{description}</div>
      )}
      {tags && <div className="mt-0.5">{t("characterDetail.snapshot.tags")}{tags}</div>}
      {originalText && <div className="mt-0.5">{t("characterDetail.snapshot.originalText")}{originalText}</div>}
    </>
  );
};

const renderSnapshotDeletionCandidate = (item: unknown, t: MessageTranslator): React.ReactNode => {
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
        {categoryLabel(category, t)} · {attributeKeyLabel(key, t)}
      </div>
      {reason && <div className="mt-0.5">{t("characterDetail.snapshot.reason")}{reason}</div>}
      {evidence && <div className="mt-0.5">{t("characterDetail.snapshot.evidence")}{evidence}</div>}
    </>
  );
};

const renderSnapshotGenericItem = (item: unknown, t: MessageTranslator): React.ReactNode => {
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
          {attributeKey ? attributeKeyLabel(attributeKey, t) : null}
          {attributeKey && decision ? " · " : null}
          {decision ? decisionLabel(decision, t) : null}
        </div>
      )}
      {reason && <div className="mt-0.5 text-book-text-sub">{reason}</div>}
    </>
  );
};

const SnapshotAttributeGroup: React.FC<{
  label: string;
  value: unknown;
  t: MessageTranslator;
}> = ({
  label,
  value,
  t,
}) => {
  const entries = isPlainRecord(value) ? Object.entries(value) : [];
  if (entries.length === 0) return null;
  return (
    <section className="px-2.5 py-2.5">
      <div className="mb-1 text-caption font-semibold text-book-text-main">
        {label}
      </div>
      <dl className="divide-y divide-book-border/35">
        {entries.map(([key, entryValue]) => (
          <div
            key={key}
            className="grid gap-0.5 py-1 text-caption sm:grid-cols-[112px_minmax(0,1fr)] sm:gap-2"
          >
            <dt
              className="break-words font-medium text-book-text-muted"
              title={key}
            >
              {attributeKeyLabel(key, t)}
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
  const { locale, t } = useI18n();
  const totalAttributes =
    state.attribute_counts.explicit +
    state.attribute_counts.implicit +
    state.attribute_counts.social;
  const metadata = [
    totalAttributes > 0
      ? t("characterDetail.snapshot.attributeCounts", {
          explicit: t("characterDetail.labels.explicit"),
          explicitCount: state.attribute_counts.explicit,
          implicit: t("characterDetail.labels.implicit"),
          implicitCount: state.attribute_counts.implicit,
          social: t("characterDetail.labels.social"),
          socialCount: state.attribute_counts.social,
        })
      : null,
    state.changes_in_chapter > 0
      ? t("characterDetail.snapshot.changes", { count: state.changes_in_chapter })
      : null,
    state.behaviors_in_chapter > 0
      ? t("characterDetail.snapshot.behaviors", { count: state.behaviors_in_chapter })
      : null,
    state.current_location
      ? t("characterDetail.snapshot.location", { value: state.current_location })
      : null,
  ].filter(Boolean);
  return (
    <section className="a4n-flat-surface overflow-hidden text-small">
      <div className="px-3 py-3">
        <div className="font-semibold text-book-text-main">
          {t("characterDetail.chapter", { chapter: state.chapter_number })}
          {state.created_at ? ` · ${formatDateTime(state.created_at, locale)}` : ""}
        </div>
        {metadata.length > 0 ? (
          <div className="mt-1 text-caption leading-relaxed text-book-text-muted">
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
        <summary className="cursor-pointer px-3 py-2.5 text-caption font-medium text-book-primary hover:bg-book-bg-tertiary/45">
          {t("characterDetail.snapshot.viewAttributes")}
        </summary>
        <div className="divide-y divide-book-border/35 border-t border-book-border/30 bg-book-bg/35">
          <SnapshotAttributeGroup
            label={t("characterDetail.labels.explicit")}
            value={state.explicit_attributes}
            t={t}
          />
          <SnapshotAttributeGroup
            label={t("characterDetail.labels.implicit")}
            value={state.implicit_attributes}
            t={t}
          />
          <SnapshotAttributeGroup
            label={t("characterDetail.labels.social")}
            value={state.social_attributes}
            t={t}
          />
        </div>
      </details>
    </section>
  );
};

export const SnapshotDetail: React.FC<{ snapshot: Snapshot }> = ({
  snapshot,
}) => {
  const { locale, t } = useI18n();
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
  const attributeCountText = snapshotAttributeCountText(payload, t);
  const changesInChapter = recordNumber(payload, "changes_in_chapter");
  const behaviorsInChapter = recordNumber(payload, "behaviors_in_chapter");
  const metadata = [
    attributeCountText,
    changesInChapter != null && changesInChapter > 0
      ? t("characterDetail.snapshot.changes", { count: changesInChapter })
      : null,
    behaviorsInChapter != null && behaviorsInChapter > 0
      ? t("characterDetail.snapshot.behaviors", { count: behaviorsInChapter })
      : null,
    location ? t("characterDetail.snapshot.location", { value: location }) : null,
  ].filter(Boolean);

  return (
    <section className="a4n-flat-surface overflow-hidden text-small">
      <div className="px-3 py-3">
        {(chapter != null || snapshot.created_at) && (
          <div className="font-semibold text-book-text-main">
            {chapter != null ? t("characterDetail.chapter", { chapter }) : t("characterDetail.snapshot.stateDetail")}
            {snapshot.created_at
              ? ` · ${formatDateTime(snapshot.created_at, locale)}`
              : ""}
          </div>
        )}
        {metadata.length > 0 ? (
          <div className="mt-1 text-caption leading-relaxed text-book-text-muted">
            {metadata.join(" · ")}
          </div>
        ) : null}
        {summary ? (
          <p className="mt-2 text-caption leading-relaxed text-book-text-sub">
            {summary}
          </p>
        ) : null}
        {(goal || emotion) && (
          <div className="mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-caption text-book-text-muted">
            {goal && <span>{t("characterDetail.snapshot.goal", { value: goal })}</span>}
            {emotion && <span>{t("characterDetail.snapshot.emotion", { value: emotion })}</span>}
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
            title={t("characterDetail.snapshot.change")}
            items={changes}
            renderItem={(item) => renderSnapshotChange(item, t)}
          />
          <SnapshotItemList
            title={t("characterDetail.snapshot.behaviorEvidence")}
            items={behaviors}
            renderItem={(item) => renderSnapshotBehavior(item, t)}
          />
          <SnapshotItemList
            title={t("characterDetail.snapshot.deletionCandidates")}
            items={deletionCandidates}
            renderItem={(item) => renderSnapshotDeletionCandidate(item, t)}
          />
          <SnapshotItemList
            title={t("characterDetail.snapshot.implicitReview")}
            items={[...classificationSuggestions, ...implicitReviewDecisions]}
            renderItem={(item) => renderSnapshotGenericItem(item, t)}
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
  const { t } = useI18n();
  const payload = isPlainRecord(delta.delta) ? delta.delta : null;
  const attributes = nestedRecord(payload, "attributes");
  const summary = nestedRecord(payload, "summary");
  const hasStructuredChanges = structuredDeltaHasChanges(attributes);
  const fromChapter = recordNumber(summary, "from_chapter");
  const toChapter = recordNumber(summary, "to_chapter");
  const fromSummary = recordString(summary, "from_state_summary");
  const toSummary = recordString(summary, "to_state_summary");

  return (
    <section className="overflow-hidden rounded-2xl border border-warning/25 bg-warning-bg/25 text-small">
      <div className="px-3 py-3">
        <div className="font-semibold text-book-text-main">{t("characterDetail.snapshot.changeSummary")}</div>
        {fromChapter != null && toChapter != null ? (
          <div className="mt-1 text-caption text-book-text-muted">
            {t("characterDetail.chapter", { chapter: fromChapter })} → {t("characterDetail.chapter", { chapter: toChapter })}
          </div>
        ) : null}
      </div>
      {(fromSummary || toSummary) && (
        <div className="grid border-t border-warning/20 text-caption md:grid-cols-2 md:divide-x md:divide-book-border/30">
          {fromSummary && (
            <div className="px-3 py-2.5 text-book-text-muted">
              <span className="font-medium text-book-text-main">{t("characterDetail.snapshot.oldState")}</span>
              <p className="mt-1 leading-relaxed">{fromSummary}</p>
            </div>
          )}
          {toSummary && (
            <div className="border-t border-book-border/30 px-3 py-2.5 text-book-text-muted md:border-t-0">
              <span className="font-medium text-book-text-main">{t("characterDetail.snapshot.newState")}</span>
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
              label={categoryLabel(category, t)}
              delta={nestedRecord(attributes, category)}
              t={t}
            />
          ))}
        </div>
      ) : (
        <div className="border-t border-warning/20 px-3 py-3 text-caption text-book-text-muted">
          {t("characterDetail.snapshot.noAttributeChanges")}
        </div>
      )}
    </section>
  );
};

const SnapshotDeltaCategory: React.FC<{
  label: string;
  delta: Record<string, unknown> | null;
  t: MessageTranslator;
}> = ({ label, delta, t }) => {
  const operations = ["added", "modified", "deleted"]
    .map((operation) => ({
      operation,
      entries: deltaEntries(delta, operation),
    }))
    .filter((group) => group.entries.length > 0);
  if (operations.length === 0) return null;

  return (
    <section className="py-2.5">
      <div className="mb-1.5 text-caption font-semibold text-book-text-main">
        {label}
      </div>
      <div className="space-y-2">
        {operations.map((group) => (
          <div key={group.operation} className="space-y-1">
            <div className="text-caption font-medium text-book-text-muted">
              {operationLabel(group.operation, t)}
            </div>
            <div className="divide-y divide-book-border/30 rounded-xl bg-book-bg-paper/45 px-2.5">
              {group.entries.map(([key, value]) => (
                <SnapshotDeltaEntry
                  key={`${group.operation}-${key}`}
                  operation={group.operation}
                  attrKey={key}
                  value={value}
                  t={t}
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
  t: MessageTranslator;
}> = ({ operation, attrKey, value, t }) => {
  const changed =
    operation === "modified" && isPlainRecord(value) ? value : null;
  return (
    <div className="py-2 text-caption">
      <div className="font-medium text-book-text-main" title={attrKey}>
        {attributeKeyLabel(attrKey, t)}
      </div>
      {changed ? (
        <div className="mt-0.5 grid gap-1 text-book-text-muted">
          <span className="flex gap-1">
            {t("characterDetail.snapshot.oldValue")}
            <SnapshotValue value={changed.from} />
          </span>
          <span className="flex gap-1">
            {t("characterDetail.snapshot.newValue")}
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
