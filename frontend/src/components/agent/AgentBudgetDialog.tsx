import { useEffect, useState } from "react";
import { controlAgentRun, getAgentRun, type AgentBudget, type AgentRunView } from "../../api/agent";
import { extractErrorMessage } from "../../api";
import { useI18n } from "../../i18n";
import { BookButton } from "../ui/BookButton";
import { BookInput } from "../ui/BookInput";
import { LoadingSpinner } from "../ui/LoadingSpinner";
import { Modal } from "../ui/Modal";

const FIELDS = ["max_steps", "max_model_calls", "max_tool_calls", "max_tokens", "max_duration_seconds"] as const;
type BudgetField = typeof FIELDS[number];

const maximumFor = (field: BudgetField) =>
  field === "max_steps" || field === "max_model_calls" || field === "max_tool_calls" ? 0xffffffff : Number.MAX_SAFE_INTEGER;

function usageFor(run: AgentRunView, field: BudgetField): number {
  switch (field) {
    case "max_steps": return run.steps_used;
    case "max_model_calls": return run.model_calls_used;
    case "max_tool_calls": return run.tool_calls_used;
    case "max_tokens": return run.tokens_used + (run.tokens_reserved ?? 0);
    case "max_duration_seconds": {
      if (!run.started_at) return 0;
      const stamp = run.started_at.replace(" ", "T");
      const time = Date.parse(/(?:Z|[+-]\d\d:\d\d)$/.test(stamp) ? stamp : `${stamp}Z`);
      if (!Number.isFinite(time)) throw new Error("Agent start timestamp is invalid");
      return Math.max(0, Math.ceil((Date.now() - time) / 1000));
    }
  }
}

export function AgentBudgetDialog({ run, onClose, onResumed }: {
  run: AgentRunView;
  onClose: () => void;
  onResumed: () => Promise<unknown>;
}) {
  const { t } = useI18n();
  const [owner, setOwner] = useState<AgentRunView | null>(null);
  const [values, setValues] = useState<Partial<Record<BudgetField, string>>>({});
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    let stopped = false;
    setOwner(null);
    setError(null);
    void getAgentRun({ run_id: run.budget_limiting_run_id ?? run.id }).then((next) => {
      if (stopped) return;
      const draft: Partial<Record<BudgetField, string>> = {};
      for (const field of FIELDS) {
        const limit = next.budget[field];
        if (limit == null) continue;
        if (!Number.isSafeInteger(limit) || limit <= 0 || limit > maximumFor(field)) throw new Error(t("agent.budget.invalid"));
        const used = usageFor(next, field);
        const increase = used >= limit || (field === "max_tokens" && run.waiting_reason === "budget");
        draft[field] = String(increase ? Math.min(maximumFor(field), Math.max(limit * 2, used + limit)) : limit);
      }
      setValues(draft);
      setOwner(next);
    }).catch((failure) => { if (!stopped) setError(extractErrorMessage(failure, t("agent.loadFailed"))); });
    return () => { stopped = true; };
  }, [run.id, run.budget_limiting_run_id, run.waiting_reason, t]);

  const invalidFields = new Set(FIELDS.filter((field) => values[field] != null && (
    !Number.isSafeInteger(Number(values[field])) || Number(values[field]) <= 0
    || Number(values[field]) < (owner?.budget[field] ?? 0) || Number(values[field]) > maximumFor(field)
  )));
  const invalid = invalidFields.size > 0;
  const close = () => { if (!saving) onClose(); };
  const resume = async () => {
    if (!owner || saving || invalid) return;
    setSaving(true);
    setError(null);
    try {
      const budget: AgentBudget = {};
      for (const field of FIELDS) {
        if (values[field] != null && Number(values[field]) !== owner.budget[field]) budget[field] = Number(values[field]);
      }
      await controlAgentRun({ thread_id: owner.thread_id, run_id: owner.id, action: "resume", ...(Object.keys(budget).length ? { budget } : {}) });
      await onResumed();
      onClose();
    } catch (failure) {
      setError(extractErrorMessage(failure, t("agent.domain.controlFailed")));
    } finally {
      setSaving(false);
    }
  };
  return <Modal isOpen onClose={close} closeOnEscape={!saving} closeOnBackdrop={!saving} showCloseButton={!saving}
    title={t("agent.budget.title")} maxWidthClassName="max-w-lg"
    footer={<div className="flex flex-wrap justify-end gap-2">
      <BookButton variant="ghost" onClick={close} disabled={saving}>{t("common.close")}</BookButton>
      <BookButton onClick={() => void resume()} disabled={!owner || saving || invalid}>
        {saving ? <LoadingSpinner size="sm" /> : null}{t("agent.budget.apply")}
      </BookButton>
    </div>}>
    <div className="space-y-4 text-sm" aria-busy={!owner && !error || saving}>
      <p className="text-book-text-sub">{t("agent.budget.explanation")}</p>
      {!owner && !error ? <LoadingSpinner size="sm" /> : null}
      {owner && <>
        {owner.id !== run.id ? <p className="text-warning">{t("agent.budget.parent")}</p> : null}
        <p className="text-book-text-muted tabular-nums">{t("agent.budget.usage", { tokens: owner.tokens_used })}
          {!owner.usage_complete ? ` · ${t("agent.budget.reserved", { tokens: owner.tokens_reserved ?? 0 })}` : ""}
        </p>
        <div className="grid min-w-0 gap-3 sm:grid-cols-2">
          {FIELDS.filter((field) => values[field] != null).map((field) => <BookInput key={field} type="number" step={1} required
            min={owner.budget[field] ?? 1} max={maximumFor(field)} label={t(`agent.budget.${field}`)}
            error={invalidFields.has(field) ? t("agent.budget.invalid") : undefined}
            value={values[field] ?? ""} onChange={(event) => setValues((current) => ({ ...current, [field]: event.target.value }))}
            disabled={saving} />)}
        </div>
      </>}
      {error ? <p role="alert" className="break-words text-error">{error}</p> : null}
    </div>
  </Modal>;
}
