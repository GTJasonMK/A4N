import React from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";
import { I18nProvider } from "../../src/i18n";
import { StoryEventsTab } from "../../src/pages/novel-detail/StoryEventsTab";
import "../../src/index.css";

const root = createRoot(document.getElementById("root")!);

function waitFor(predicate: () => boolean): Promise<void> {
  return new Promise((resolve, reject) => {
    const deadline = performance.now() + 20_000;
    const check = () => predicate() ? resolve() : requestAnimationFrame(guarded);
    const guarded = () => performance.now() > deadline ? reject(new Error(`wait timeout: ${document.body.textContent?.slice(0, 300)}`)) : check();
    guarded();
  });
}

async function measureStoryEventsLoader() {
  const calls = (window as typeof window & { storyEventsLoaderCalls?: unknown[] }).storyEventsLoaderCalls ?? [];
  calls.length = 0;
  const started = performance.now();
  root.render(
    <MemoryRouter>
      <I18nProvider initialLocale="zh-CN">
        <StoryEventsTab projectId={1} canOpenWritingDesk={false} />
      </I18nProvider>
    </MemoryRouter>,
  );
  await waitFor(() => document.querySelectorAll(".a4n-story-event-section").length === 1000);
  const initialReadyMs = performance.now() - started;
  const initialCalls = calls.map((call) => ({ ...(call as Record<string, unknown>) }));
  const initialBytes = initialCalls.reduce((total, call) => total + Number(call.bytes ?? 0), 0);
  const details = document.querySelector("details");
  if (!(details instanceof HTMLDetailsElement)) throw new Error("Foreshadowing details missing");
  const expandStarted = performance.now();
  (details.querySelector("summary") as HTMLElement).click();
  await waitFor(() => calls.some((call) => (call as Record<string, unknown>).command === "list_foreshadowing"));
  await waitFor(() => details.querySelectorAll("article").length === 1000);
  const expandedReadyMs = performance.now() - expandStarted;
  return {
    initialReadyMs,
    expandedReadyMs,
    initialCalls,
    initialBytes,
    allCalls: calls,
    detailsOpen: details.open,
    foreshadowingRows: details.querySelectorAll("article").length,
  };
}

Object.assign(window, { measureStoryEventsLoader, performanceReady: true });
