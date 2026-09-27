import React from "react";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { usePacedStreamText, type PacedStreamText } from "../../src/hooks/usePacedStreamText";
import { applyActiveTheme, setThemeAppearance } from "../../src/theme/activeTheme";
import { formatNumber, formatRelativeTime } from "../../src/i18n/format";
import { MarkdownText } from "../../src/components/ui/MarkdownText";
import type { ThemeStreamDisplaySettings } from "../../src/api/theme";

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function setDisplay(settings: ThemeStreamDisplaySettings) {
  setThemeAppearance("light");
  applyActiveTheme({
    id: 1, user_id: 1, name: "Performance fixture", mode: "light",
    is_active: true, is_system_default: false, resources: [],
    document: { version: 1, tokens: {}, backgrounds: {}, stream_display: settings },
  });
}

// Only the timer clock is controlled. The production hook, React and DOM all run in Chromium.
function streamSession() {
  const nativeTimeout = globalThis.setTimeout;
  const nativeClear = globalThis.clearTimeout;
  const nativeNow = Date.now;
  let now = nativeNow();
  let nextId = 0;
  const timers = new Map<number, { run: () => void; delay: number }>();
  globalThis.setTimeout = ((run: () => void, delay = 0) => {
    timers.set(++nextId, { run, delay });
    return nextId;
  }) as typeof setTimeout;
  globalThis.clearTimeout = (id) => { timers.delete(Number(id)); };
  Date.now = () => now;
  let latest: PacedStreamText;
  let root: Root | null = createRoot(document.getElementById("root")!);
  function Probe(props: { text: string; active: boolean; drain: boolean; streamed: boolean }) {
    latest = usePacedStreamText(props.text, props.active, props.drain, props.streamed);
    return <pre style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>{latest.displayedText}</pre>;
  }
  return {
    render(text: string, active = true, drain = true, streamed = false) {
      flushSync(() => root!.render(<Probe text={text} active={active} drain={drain} streamed={streamed} />));
      return latest!;
    },
    async settle() {
      await new Promise<void>((resolve) => nativeTimeout(resolve, 0));
      return latest!;
    },
    tick() {
      const entry = timers.entries().next().value;
      check(entry, "stream timer is missing");
      const [id, timer] = entry;
      timers.delete(id);
      now += timer.delay;
      flushSync(timer.run);
      return { ...latest!, delay: timer.delay };
    },
    close() {
      flushSync(() => root!.unmount());
      root = null;
      const remaining = timers.size;
      globalThis.setTimeout = nativeTimeout;
      globalThis.clearTimeout = nativeClear;
      Date.now = nativeNow;
      check(remaining === 0, "unmount must release stream timers");
    },
  };
}

async function verifyStream() {
  const trace = [];
  for (const mode of ["fixed", "adaptive"] as const) {
    setDisplay({ enabled: true, rate: 30, mode });
    const session = streamSession();
    const update = async (...args: Parameters<typeof session.render>) => {
      session.render(...args);
      trace.push(await session.settle());
    };
    try {
      await update("猫🙂甲");
      check(session.tick().displayedText === "猫", "first character");
      check(session.tick().displayedText === "猫🙂", "Unicode code point must remain whole");
      await update("猫🙂甲乙");
      trace.push(session.tick());
      await update("猫🙂新");
      check(document.querySelector("pre")!.textContent === "猫🙂", "replacement keeps common prefix");
      trace.push(session.tick());
      await update("猫", false);
      check(document.querySelector("pre")!.textContent === "猫", "truncation reconciles");
      await update("猫🙂尾", false, false);
      check(document.querySelector("pre")!.textContent === "猫🙂尾", "inactive text is immediately visible");
      await update("猫🙂尾末", true);
      await update("猫🙂尾末", false, true);
      trace.push(session.tick());
      check(document.querySelector("pre")!.textContent === "猫🙂尾末", "terminal stream drains");
      await update("猫🙂尾末", false, true, true);
      trace.push(session.tick());
      check(document.querySelector("pre")!.textContent === "猫", "newly observed stream replays after completion");
      flushSync(() => setDisplay({ enabled: false, rate: 30, mode }));
      trace.push(await session.settle());
      check(document.querySelector("pre")!.textContent === "猫🙂尾末", "disabling pacing reveals the source");
      flushSync(() => setDisplay({ enabled: true, rate: 60, mode }));
      trace.push(await session.settle());
      trace.push(session.tick());
      check(document.querySelector("pre")!.textContent === "猫", "changed theme resumes Unicode pacing");
    } finally {
      session.close();
    }
  }
  setDisplay({ enabled: false, rate: 30, mode: "fixed" });
  const disabled = streamSession();
  try {
    trace.push(disabled.render("未启用逐字显示🙂"));
    check(document.querySelector("pre")!.textContent === "未启用逐字显示🙂", "disabled mode shows source");
  } finally {
    disabled.close();
  }
  return trace;
}

async function measureStream(characters: number) {
  setDisplay({ enabled: true, rate: 120, mode: "fixed" });
  const points = Array.from("风🙂A".repeat(Math.ceil(characters / 3))).slice(0, characters);
  const text = points.join("");
  const ticks = Math.min(1000, characters - 5);
  const samples = [];
  const warmupUntil = performance.now() + 250;
  const session = streamSession();
  try {
    for (let sample = 0; samples.length < 9; sample++) {
      session.render("", false, false);
      await session.settle();
      session.render(text);
      await session.settle();
      for (let i = 0; i < 5; i++) session.tick();
      const started = performance.now();
      for (let i = 0; i < ticks; i++) session.tick();
      const elapsed = performance.now() - started;
      if (sample >= 3 && started >= warmupUntil) samples.push(elapsed);
      check(document.querySelector("pre")!.textContent === points.slice(0, ticks + 5).join(""), "paced output changed");
    }
  } finally {
    session.close();
  }
  const sorted = [...samples].sort((a, b) => a - b);
  return { characters, ticks, samplesMs: samples, medianMs: sorted[4] };
}

function measureFormatting() {
  const results = [];
  for (const locale of ["zh-CN", "en-US"] as const) {
    for (const value of [0, -1234.5, 999999, NaN, Infinity, 1234567890123456789n]) {
      check(formatNumber(value, locale) === new Intl.NumberFormat(locale).format(value), "number formatting changed");
    }
    check(formatNumber(3.14159, locale, { maximumFractionDigits: 2 }) === new Intl.NumberFormat(locale, { maximumFractionDigits: 2 }).format(3.14159), "custom number options changed");
    const ago = Date.now() - 120_000;
    check(formatRelativeTime(ago, locale) === new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(-2, "minute"), "relative time formatting changed");
    for (const kind of ["number", "relative"] as const) {
      const samples = [];
      let output = "";
      const warmupUntil = performance.now() + 250;
      for (let sample = 0; samples.length < 9; sample++) {
        const started = performance.now();
        for (let index = 0; index < 2000; index++) {
          output = kind === "number" ? formatNumber(index * 1000.5, locale) : formatRelativeTime(ago, locale);
        }
        const elapsed = performance.now() - started;
        if (sample >= 3 && started >= warmupUntil) samples.push(elapsed);
      }
      check(output.length > 0, "missing formatted output");
      const sorted = [...samples].sort((a, b) => a - b);
      results.push({ locale, kind, calls: 2000, samplesMs: samples, medianMs: sorted[4] });
    }
  }
  return results;
}

async function measureMarkdown() {
  const history = Array.from({ length: 50 }, (_, index) =>
    `## 历史回复 ${index}\n\n${"正文包含**重点**、`代码`与[安全链接](https://example.test)。\n\n".repeat(20)}`);
  const element = document.getElementById("root")!;
  const root = createRoot(element);
  function Conversation({ update }: { update: number }) {
    return <>
      {history.map((text, index) => <MarkdownText key={index} text={text} className="history" />)}
      <section id="live"><MarkdownText text={`新回复 **${update}**`} /></section>
    </>;
  }
  const samples = [];
  try {
    flushSync(() => root.render(<Conversation update={0} />));
    const warmupUntil = performance.now() + 250;
    for (let sample = 0; samples.length < 9; sample++) {
      const started = performance.now();
      for (let update = 0; update < 40; update++) {
        flushSync(() => root.render(<Conversation update={update} />));
      }
      const elapsed = performance.now() - started;
      if (sample >= 3 && started >= warmupUntil) samples.push(elapsed);
    }
    check(element.querySelectorAll(".history").length === 50, "history messages disappeared");
    check(element.querySelector("#live strong")?.textContent === "39", "updated markdown did not render");
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(element.innerHTML));
    const htmlSha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
    const sorted = [...samples].sort((a, b) => a - b);
    return { historyMessages: history.length, charactersPerMessage: history[0].length, updates: 40, samplesMs: samples, medianMs: sorted[4], htmlSha256 };
  } finally {
    flushSync(() => root.unmount());
  }
}

Object.assign(window, { verifyStream, measureStream, measureFormatting, measureMarkdown, performanceReady: true });
