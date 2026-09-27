import { memo, type ReactNode } from "react";

interface MarkdownTextProps {
  text: string;
  className?: string;
}

type MarkdownBlock =
  | { type: "paragraph"; text: string }
  | { type: "ul" | "ol"; items: string[] }
  | { type: "heading"; level: number; text: string }
  | { type: "blockquote"; text: string }
  | { type: "code"; language: string | null; text: string }
  | { type: "table"; headers: string[]; rows: string[][] }
  | { type: "hr" };

export const MarkdownText = memo(function MarkdownText({ text, className }: MarkdownTextProps) {
  const blocks = markdownBlocks(text);
  if (blocks.length === 0) return null;
  return (
    <div className={`min-w-0 whitespace-pre-wrap ${className ?? ""}`}>
      {blocks.map((block, index) => {
        if (block.type === "ul" || block.type === "ol") {
          const List = block.type === "ul" ? "ul" : "ol";
          return (
            <List
              key={index}
              className={`my-1 min-w-0 space-y-0.5 pl-4 ${
                block.type === "ul" ? "list-disc" : "list-decimal"
              }`}
            >
              {block.items.map((item, itemIndex) => (
                <li key={itemIndex}>{renderInlineMarkdown(item)}</li>
              ))}
            </List>
          );
        }
        if (block.type === "heading") {
          if (block.level === 1) {
            return (
              <h3 key={index} className="my-2 text-sm font-semibold leading-snug text-book-text-main">
                {renderInlineMarkdown(block.text)}
              </h3>
            );
          }
          if (block.level <= 3) {
            return (
              <h4 key={index} className="my-1.5 text-xs font-semibold leading-snug text-book-text-main">
                {renderInlineMarkdown(block.text)}
              </h4>
            );
          }
          return (
            <h5 key={index} className="my-1.5 text-xs font-medium leading-snug text-book-text-main">
              {renderInlineMarkdown(block.text)}
            </h5>
          );
        }
        if (block.type === "blockquote") {
          return (
            <blockquote
              key={index}
              className="my-2 min-w-0 border-l-2 border-book-accent/55 pl-3 text-book-text-sub"
            >
              <MarkdownText text={block.text} />
            </blockquote>
          );
        }
        if (block.type === "code") {
          return (
            <div
              key={index}
              className="my-2 min-w-0 max-w-full overflow-hidden rounded-lg border border-book-border/55 bg-book-bg-tertiary/55"
            >
              {block.language ? (
                <div className="border-b border-book-border/45 px-2 py-1 font-mono text-[10px] text-book-text-muted">
                  {block.language}
                </div>
              ) : null}
              <pre className="max-h-64 max-w-full overflow-auto px-2 py-2 font-mono text-[10px] leading-relaxed text-book-text-main custom-scrollbar">
                <code>{block.text}</code>
              </pre>
            </div>
          );
        }
        if (block.type === "table") {
          return (
            <div
              key={index}
              className="my-2 max-w-full overflow-x-auto rounded-lg border border-book-border/55"
            >
              <table className="min-w-full border-collapse text-[10px] leading-relaxed">
                <thead className="bg-book-bg-tertiary/55 text-left text-book-text-sub">
                  <tr>
                    {block.headers.map((cell, cellIndex) => (
                      <th key={cellIndex} className="border-b border-book-border/55 px-2 py-1.5 font-semibold">
                        {renderInlineMarkdown(cell)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {block.rows.map((row, rowIndex) => (
                    <tr key={rowIndex} className="border-b border-book-border/35 last:border-b-0">
                      {block.headers.map((_, cellIndex) => (
                        <td key={cellIndex} className="px-2 py-1.5 align-top text-book-text-sub">
                          {renderInlineMarkdown(row[cellIndex] ?? "")}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
        }
        if (block.type === "hr") {
          return <hr key={index} className="my-2 border-book-border/55" />;
        }
        if (block.type !== "paragraph") return null;
        return (
          <p key={index} className="my-1 min-w-0 first:mt-0 last:mb-0">
            {renderInlineMarkdown(block.text)}
          </p>
        );
      })}
    </div>
  );
});

function markdownBlocks(text: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  let paragraph: string[] = [];
  let list: { type: "ul" | "ol"; items: string[] } | null = null;

  const flushParagraph = () => {
    if (paragraph.length === 0) return;
    blocks.push({ type: "paragraph", text: paragraph.join("\n") });
    paragraph = [];
  };
  const flushList = () => {
    if (!list) return;
    blocks.push(list);
    list = null;
  };
  const flushText = () => {
    flushParagraph();
    flushList();
  };

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    if (!trimmed) {
      flushText();
      continue;
    }

    const fence = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);
    if (fence) {
      flushText();
      const marker = fence[1][0];
      const markerLength = fence[1].length;
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !isClosingFence(lines[index], marker, markerLength)) {
        code.push(lines[index]);
        index += 1;
      }
      blocks.push({
        type: "code",
        language: fence[2].trim() || null,
        text: code.join("\n"),
      });
      continue;
    }

    const heading = /^(#{1,6})\s+(.+?)\s*#*$/.exec(trimmed);
    if (heading) {
      flushText();
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2] });
      continue;
    }
    if (/^(?:\*\s*){3,}$/.test(trimmed) || /^(?:-\s*){3,}$/.test(trimmed) || /^(?:_\s*){3,}$/.test(trimmed)) {
      flushText();
      blocks.push({ type: "hr" });
      continue;
    }

    if (index + 1 < lines.length && isTableSeparator(lines[index + 1])) {
      const headers = splitTableRow(line);
      if (headers.length > 1) {
        flushText();
        const rows: string[][] = [];
        index += 2;
        while (index < lines.length) {
          const row = splitTableRow(lines[index]);
          if (row.length <= 1) {
            index -= 1;
            break;
          }
          rows.push(row);
          index += 1;
        }
        blocks.push({ type: "table", headers, rows });
        continue;
      }
    }

    const quote = /^ {0,3}>\s?(.*)$/.exec(line);
    if (quote) {
      flushText();
      const quoteLines: string[] = [];
      while (index < lines.length) {
        const nextQuote = /^ {0,3}>\s?(.*)$/.exec(lines[index]);
        if (nextQuote) {
          quoteLines.push(nextQuote[1]);
          index += 1;
          continue;
        }
        if (!lines[index].trim()) {
          quoteLines.push("");
          index += 1;
          continue;
        }
        break;
      }
      blocks.push({ type: "blockquote", text: quoteLines.join("\n").trim() });
      index -= 1;
      continue;
    }

    const unordered = /^ {0,3}[-+*]\s+(.+)$/.exec(line);
    if (unordered) {
      flushParagraph();
      if (!list || list.type !== "ul") {
        flushList();
        list = { type: "ul", items: [] };
      }
      list.items.push(unordered[1]);
      continue;
    }
    const ordered = /^ {0,3}\d+[.)]\s+(.+)$/.exec(line);
    if (ordered) {
      flushParagraph();
      if (!list || list.type !== "ol") {
        flushList();
        list = { type: "ol", items: [] };
      }
      list.items.push(ordered[1]);
      continue;
    }

    flushList();
    paragraph.push(line);
  }

  flushText();
  return blocks;
}

function isClosingFence(line: string, marker: string, length: number): boolean {
  const trimmed = line.trim();
  const fence = marker.repeat(length);
  return trimmed.startsWith(fence) && trimmed.slice(fence.length).trim() === "";
}

function splitTableRow(line: string): string[] {
  const trimmed = line.trim();
  if (!trimmed.includes("|")) return [];
  return trimmed
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim().replace(/\\\|/g, "|"));
}

function isTableSeparator(line: string): boolean {
  const cells = splitTableRow(line);
  return cells.length > 1 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function safeHref(value: string): string | null {
  const href = value.trim();
  return /^(?:https?:\/\/|mailto:|\/|#|\.\.?\/)/i.test(href) ? href : null;
}

function renderInlineMarkdown(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(\[[^\]\n]+\]\((?:[^)\s]+)(?:\s+["'][^)]*["'])?\)|\*\*[^*\n]+\*\*|__[^_\n]+__|~~[^~\n]+~~|`[^`\n]+`|\*[^*\n]+\*|_[^_\n]+_)/g;
  let lastIndex = 0;
  for (const match of text.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > lastIndex) nodes.push(text.slice(lastIndex, index));
    const value = match[0];
    const link = /^\[([^\]]+)\]\(([^)\s]+)(?:\s+["'][^)]*["'])?\)$/.exec(value);
    const href = link ? safeHref(link[2]) : null;
    if (link && href) {
      nodes.push(
        <a key={nodes.length} href={href} className="text-book-primary underline decoration-book-primary/35 underline-offset-2 hover:decoration-book-primary">
          {renderInlineMarkdown(link[1])}
        </a>,
      );
    } else if (value.startsWith("**") || value.startsWith("__")) {
      nodes.push(<strong key={nodes.length}>{value.slice(2, -2)}</strong>);
    } else if (value.startsWith("~~")) {
      nodes.push(<del key={nodes.length}>{value.slice(2, -2)}</del>);
    } else if (value.startsWith("`")) {
      nodes.push(
        <code key={nodes.length} className="break-all rounded bg-book-bg-tertiary px-1 py-0.5 font-mono text-[0.92em]">
          {value.slice(1, -1)}
        </code>,
      );
    } else if (value.startsWith("*") || value.startsWith("_")) {
      nodes.push(<em key={nodes.length}>{value.slice(1, -1)}</em>);
    } else {
      nodes.push(value);
    }
    lastIndex = index + value.length;
  }
  if (lastIndex < text.length) nodes.push(text.slice(lastIndex));
  return nodes;
}
