import React from "react";
import { FileText } from "lucide-react";
import { Modal } from "../../../components/ui/Modal";
import { useI18n } from "../../../i18n";
import type { GenerationInputSource } from "../../../api/project";

const INLINE_TEXT_MAX_CHARS = 120;
const AUTO_OPEN_TEXT_MAX_CHARS = 360;
const AUTO_OPEN_ARRAY_MAX_ITEMS = 4;
const AUTO_OPEN_OBJECT_MAX_FIELDS = 8;

export interface InputViewerState {
  recordLabel: string;
  inputs: GenerationInputSource[];
}

export const PromptViewerModal: React.FC<{
  viewer: InputViewerState | null;
  onClose: () => void;
}> = ({ viewer, onClose }) => {
  const { t } = useI18n();
  if (!viewer) return null;
  return (
    <Modal
      isOpen
      onClose={onClose}
      title={t("novel.assetGraph.generationInputTitle")}
      subtitle={viewer.recordLabel}
      icon={<FileText size={15} />}
      maxWidthClassName="max-w-5xl"
      bodyClassName="min-h-0 flex-1 overflow-y-auto p-3 sm:p-4 custom-scrollbar"
      zIndexClassName="z-[70]"
    >
      <div className="space-y-3">
        {groupGenerationInputs(viewer.inputs).map(([group, inputs]) => (
          <section key={group} className="a4n-flat-surface overflow-hidden">
            <div className="border-b border-book-border/35 px-3 py-2.5">
              <div className="text-xs font-semibold text-book-text-main">
                {inputGroupLabel(group, t)}
              </div>
              <div className="mt-0.5 text-caption tabular-nums text-book-text-subtle">
                {t("novel.assetGraph.inputItems", { count: inputs.length })}
              </div>
            </div>
            <div className="divide-y divide-book-border/35">
              {inputs.map((input, index) => (
                <GenerationInputCard
                  key={`${input.kind}:${index}`}
                  input={input}
                />
              ))}
            </div>
          </section>
        ))}
      </div>
    </Modal>
  );
};

const GenerationInputCard: React.FC<{ input: GenerationInputSource }> = ({
  input,
}) => {
  return (
    <article className="px-3 py-3">
      <h4
        className="truncate text-small font-semibold text-book-text-main"
        title={input.label}
      >
        {input.label}
      </h4>
      <div className="mt-2">
        <GenerationInputBody input={input} />
      </div>
    </article>
  );
};

const GenerationInputBody: React.FC<{ input: GenerationInputSource }> = ({
  input,
}) => {
  if (input.kind === "rendered_messages") {
    return <MessageList value={input.value} />;
  }
  if (typeof input.value === "string") {
    return <TextBlock text={input.value} />;
  }
  return <StructuredValueView value={input.value} />;
};

const MessageList: React.FC<{ value: unknown }> = ({ value }) => {
  const { t } = useI18n();
  if (!Array.isArray(value) || !value.every(isChatMessageLike)) {
    return <StructuredValueView value={value} />;
  }
  if (!value.length) {
    return (
      <span className="text-body-sm text-book-text-subtle">
        {t("novel.assetGraph.emptyMessages")}
      </span>
    );
  }
  return (
    <div className="divide-y divide-book-border/30 border-y border-book-border/30">
      {value.map((message, index) => (
        <MessageBlock
          key={`${String(message.role ?? "message")}-${index}`}
          message={message}
          index={index}
        />
      ))}
    </div>
  );
};

const MessageBlock: React.FC<{
  message: Record<string, unknown>;
  index: number;
}> = ({ message, index }) => {
  const { t } = useI18n();
  const role =
    typeof message.role === "string" && message.role.trim()
      ? message.role
      : "message";
  const content = message.content;
  const extraEntries = Object.entries(message).filter(
    ([key]) => key !== "role" && key !== "content",
  );
  return (
    <article className="py-2.5">
      <div className="flex items-center justify-between gap-3">
        <span className="text-small font-semibold text-book-text-main">
          {messageRoleLabel(role, t)}
        </span>
        <span className="text-caption tabular-nums text-book-text-subtle">
          {t("novel.assetGraph.messageNumber", { number: index + 1 })}
        </span>
      </div>
      <div className="mt-2">
        {typeof content === "string" ? (
          <TextBlock text={content} />
        ) : (
          <StructuredValueView value={content} />
        )}
        {extraEntries.length ? (
          <div className="mt-2 border-t border-book-border/70 pt-1.5">
            <StructuredEntriesView entries={extraEntries} depth={1} />
          </div>
        ) : null}
      </div>
    </article>
  );
};

const StructuredValueView: React.FC<{ value: unknown; depth?: number }> = ({
  value,
  depth = 0,
}) => {
  const { t } = useI18n();
  if (value == null) {
    return <span className="text-body-sm text-book-text-subtle">null</span>;
  }
  if (typeof value === "string") {
    return <TextBlock text={value} />;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return (
      <span className="font-mono text-body-sm text-book-text-main">
        {String(value)}
      </span>
    );
  }
  if (Array.isArray(value)) {
    if (!value.length) {
      return (
        <span className="text-body-sm text-book-text-subtle">
          {t("novel.assetGraph.emptyArray")}
        </span>
      );
    }
    if (value.every(isChatMessageLike)) {
      return <MessageList value={value} />;
    }
    return (
      <div className="divide-y divide-book-border/25 border-y border-book-border/30">
        {value.map((item, index) => (
          <StructuredValueItem
            key={index}
            title={`#${index + 1}`}
            value={item}
            depth={depth}
          />
        ))}
      </div>
    );
  }
  if (isRecord(value)) {
    return (
      <StructuredEntriesView entries={Object.entries(value)} depth={depth} />
    );
  }
  return <TextBlock text={String(value)} />;
};

const StructuredEntriesView: React.FC<{
  entries: Array<[string, unknown]>;
  depth: number;
}> = ({ entries, depth }) => {
  const { t } = useI18n();
  if (!entries.length) {
    return (
      <span className="text-body-sm text-book-text-subtle">
        {t("novel.assetGraph.emptyObject")}
      </span>
    );
  }
  return (
    <div className="divide-y divide-book-border/25 border-y border-book-border/30">
      {entries.map(([key, value]) => (
        <StructuredValueItem
          key={key}
          title={key.replace(/_/g, " ")}
          value={value}
          depth={depth}
        />
      ))}
    </div>
  );
};

const StructuredValueItem: React.FC<{
  title: string;
  value: unknown;
  depth: number;
}> = ({ title, value, depth }) => {
  const { t } = useI18n();
  if (typeof value === "string") {
    const text = value;
    const compact =
      !text.includes("\n") && text.length <= INLINE_TEXT_MAX_CHARS;
    return compact ? (
      <div className="grid grid-cols-[minmax(6rem,9rem)_minmax(0,1fr)] gap-2 py-2 text-small">
        <div
          className="truncate font-semibold text-book-text-muted"
          title={title}
        >
          {title}
        </div>
        <div className="min-w-0 break-words text-book-text-main">{text}</div>
      </div>
    ) : (
      <details open={text.length <= AUTO_OPEN_TEXT_MAX_CHARS && depth === 0}>
        <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
          <span>{title}</span>
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {textSummary(text, t)}
          </span>
        </summary>
        <div className="border-l border-book-border/35 pb-2 pl-3">
          <TextBlock text={text} />
        </div>
      </details>
    );
  }
  if (
    value == null ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return (
      <div className="grid grid-cols-[minmax(6rem,9rem)_minmax(0,1fr)] gap-2 py-2 text-small">
        <div
          className="truncate font-semibold text-book-text-muted"
          title={title}
        >
          {title}
        </div>
        <div className="min-w-0 break-words font-mono text-book-text-main">
          {String(value)}
        </div>
      </div>
    );
  }
  if (
    title === "messages" &&
    Array.isArray(value) &&
    value.every(isChatMessageLike)
  ) {
    return (
      <details open={value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS && depth === 0}>
        <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
          <span>{title}</span>
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {t("novel.assetGraph.messagesSummary", { count: value.length })}
          </span>
        </summary>
        <div className="border-l border-book-border/35 pb-2 pl-3">
          <MessageList value={value} />
        </div>
      </details>
    );
  }
  const summary = structuredValueSummary(value, t);
  return (
    <details open={shouldAutoOpenStructuredValue(value, depth)}>
      <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-muted hover:text-book-text-main">
        <span>{title}</span>
        {summary ? (
          <span className="ml-2 font-mono text-caption font-normal text-book-text-subtle">
            {summary}
          </span>
        ) : null}
      </summary>
      <div className="border-l border-book-border/35 pb-2 pl-3">
        <StructuredValueView value={value} depth={depth + 1} />
      </div>
    </details>
  );
};

function shouldAutoOpenStructuredValue(value: unknown, depth: number): boolean {
  if (depth > 0) return false;
  if (Array.isArray(value)) return value.length <= AUTO_OPEN_ARRAY_MAX_ITEMS;
  if (isRecord(value))
    return Object.keys(value).length <= AUTO_OPEN_OBJECT_MAX_FIELDS;
  return true;
}

function structuredValueSummary(
  value: unknown,
  t: ReturnType<typeof useI18n>["t"],
): string | null {
  if (Array.isArray(value))
    return t("novel.assetGraph.arraySummary", { count: value.length });
  if (isRecord(value))
    return t("novel.assetGraph.objectSummary", {
      count: Object.keys(value).length,
    });
  return null;
}

function textSummary(text: string, t: ReturnType<typeof useI18n>["t"]): string {
  const lines = text.split("\n").length;
  return lines > 1
    ? t("novel.assetGraph.textSummaryLines", { chars: text.length, lines })
    : t("novel.assetGraph.textSummaryChars", { chars: text.length });
}

const TextBlock: React.FC<{ text: string }> = ({ text }) => (
  <pre className="m-0 whitespace-pre-wrap break-words text-body-sm leading-relaxed text-book-text-main">
    {text}
  </pre>
);

export const JsonBlock: React.FC<{ title: string; value: unknown }> = ({
  title,
  value,
}) => (
  <details className="border-t border-book-border/30 py-1">
    <summary className="cursor-pointer py-2 text-small font-semibold text-book-text-main">
      {title}
    </summary>
    <div className="border-l border-book-border/35 pb-2 pl-3">
      <StructuredValueView value={value} />
    </div>
  </details>
);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function originLabel(
  origin: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    derived_text: "novel.assetGraph.origin.derivedText",
    llm_generated: "novel.assetGraph.origin.llmGenerated",
    image_generated: "novel.assetGraph.origin.imageGenerated",
    video_generated: "novel.assetGraph.origin.videoGenerated",
    uploaded: "novel.assetGraph.origin.uploaded",
    imported: "novel.assetGraph.origin.imported",
    manual: "novel.assetGraph.origin.manual",
    extracted_frame: "novel.assetGraph.origin.extractedFrame",
    unknown: "novel.assetGraph.origin.unknown",
  };
  return t(labels[origin] ?? "novel.assetGraph.origin.other");
}

export function inputStatusLabel(
  status: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    complete: "novel.assetGraph.inputStatus.complete",
    missing: "novel.assetGraph.inputStatus.missing",
    run_without_inputs: "novel.assetGraph.inputStatus.runWithoutInputs",
    structural: "novel.assetGraph.inputStatus.structural",
    not_applicable: "novel.assetGraph.inputStatus.notApplicable",
  };
  return t(labels[status] ?? "novel.assetGraph.inputStatus.unknown");
}

export function generationInputEmptyText(
  status: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    structural: "novel.assetGraph.inputEmpty.structural",
    not_applicable: "novel.assetGraph.inputEmpty.notApplicable",
    run_without_inputs: "novel.assetGraph.inputEmpty.runWithoutInputs",
    missing: "novel.assetGraph.inputEmpty.missing",
  };
  return t(labels[status] ?? "novel.assetGraph.inputEmpty.missing");
}

function groupGenerationInputs(
  inputs: GenerationInputSource[],
): Array<[string, GenerationInputSource[]]> {
  const order = [
    "prompt",
    "variables",
    "source_text",
    "media",
    "request",
    "response",
  ];
  const grouped = new Map<string, GenerationInputSource[]>();
  inputs.forEach((input) => {
    const list = grouped.get(input.group) ?? [];
    list.push(input);
    grouped.set(input.group, list);
  });
  return [...grouped.entries()].sort(([a], [b]) => {
    const ai = order.indexOf(a);
    const bi = order.indexOf(b);
    return (ai < 0 ? order.length : ai) - (bi < 0 ? order.length : bi);
  });
}

function inputGroupLabel(
  group: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    prompt: "novel.assetGraph.inputGroup.prompt",
    variables: "novel.assetGraph.inputGroup.variables",
    source_text: "novel.assetGraph.inputGroup.sourceText",
    media: "novel.assetGraph.inputGroup.media",
    request: "novel.assetGraph.inputGroup.request",
    response: "novel.assetGraph.inputGroup.response",
  };
  return t(labels[group] ?? "novel.assetGraph.inputGroup.other");
}

function isChatMessageLike(value: unknown): value is Record<string, unknown> {
  if (!isRecord(value)) return false;
  return "content" in value || "role" in value;
}

function messageRoleLabel(
  role: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  const labels: Record<string, Parameters<typeof t>[0]> = {
    system: "novel.assetGraph.messageRole.system",
    user: "novel.assetGraph.messageRole.user",
    assistant: "novel.assetGraph.messageRole.assistant",
    tool: "novel.assetGraph.messageRole.tool",
    message: "novel.assetGraph.messageRole.message",
  };
  return t(labels[role] ?? "novel.assetGraph.messageRole.message");
}
