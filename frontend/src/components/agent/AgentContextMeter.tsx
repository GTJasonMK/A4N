import type { AgentRunView } from "../../api/agent";
import type { Locale } from "../../i18n";

type AgentContextRun = Pick<
  AgentRunView,
  "context_input_tokens" | "context_capacity_tokens"
>;

export function AgentContextMeter({
  run,
  locale,
}: {
  run: AgentContextRun | null;
  locale: Locale;
}) {
  const used = run?.context_input_tokens;
  const capacity = run?.context_capacity_tokens;
  if (
    used == null ||
    capacity == null ||
    !Number.isFinite(used) ||
    !Number.isFinite(capacity) ||
    capacity <= 0
  ) {
    return null;
  }

  const percentage = Math.round(Math.min(100, Math.max(0, (used / capacity) * 100)));
  const level = percentage >= 95 ? "critical" : percentage >= 80 ? "warning" : "normal";
  const label = locale === "zh-CN" ? "上下文" : "Context";
  const detail = locale === "zh-CN"
    ? `已用 ${formatTokens(used, locale)} / ${formatTokens(capacity, locale)} 个可用输入 token`
    : `${formatTokens(used, locale)} / ${formatTokens(capacity, locale)} available input tokens used`;

  return (
    <div
      className="agent-context-meter"
      data-agent-context-level={level}
      role="status"
      aria-label={`${label} ${percentage}%. ${detail}`}
      title={detail}
    >
      <span className="agent-context-meter__label">{label}</span>
      <span className="agent-context-meter__value">{percentage}%</span>
      <span className="agent-context-meter__track" aria-hidden="true">
        <span className="agent-context-meter__fill" style={{ width: `${percentage}%` }} />
      </span>
    </div>
  );
}

function formatTokens(value: number, locale: Locale): string {
  return Math.max(0, Math.round(value)).toLocaleString(locale === "zh-CN" ? "zh-CN" : "en-US");
}
