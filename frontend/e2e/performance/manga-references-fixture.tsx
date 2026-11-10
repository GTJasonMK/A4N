import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import type { CharacterProfile } from "../../src/api/character";
import type { GeneratedImage } from "../../src/api/image";
import type { Portrait } from "../../src/api/portrait";
import { I18nProvider } from "../../src/i18n";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { ReferenceBindingsPanel } from "../../src/pages/novel-detail/manga/ReferenceBindingsPanel";
import type { MangaPagePrompt } from "../../src/pages/novel-detail/manga/mangaPagePromptModel";
import "../../src/index.css";

const params = new URLSearchParams(location.search);
const imageCount = Number(params.get("images") ?? 5000);
const bindingCount = Number(params.get("bindings") ?? 3);
const mode = params.get("mode") ?? "measure";
const profileCount = Number(params.get("profiles") ?? 50);
const targetQuery = params.get("q") ?? "asset-" + imageCount;
const createdAt = "2026-01-01T00:00:00.000Z";
const previewImage = "data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%2232%22%20height%3D%2232%22%3E%3Crect%20width%3D%2232%22%20height%3D%2232%22%20fill%3D%22%238a4c2f%22%2F%3E%3C%2Fsvg%3E";
const textFor = (index: number) =>
  "An atmospheric manga illustration with a quiet city street, warm window light, careful ink detail, and Character " +
  (index % bindingCount + 1) + " in the foreground, scene " + index;

const profiles = Array.from({ length: profileCount }, (_, index) => ({
  id: index + 1,
  project_id: 1,
  name: "Character " + (index + 1),
})) as unknown as CharacterProfile[];
const portraits = Array.from({ length: profileCount * 4 }, (_, index) => ({
  created_at: createdAt,
  expression: index % 2 === 0 ? "neutral" : "thoughtful",
  id: index + 1,
  image_path: previewImage,
  is_primary: index % 4 === 0,
  portrait_type: "half_body",
  pose: "standing",
  profile_id: Math.floor(index / 4) + 1,
  project_id: 1,
  prompt: "asset-" + (index + 1) + " " + textFor(index),
})) as unknown as Portrait[];
const projectImages = Array.from({ length: imageCount }, (_, index) => ({
  ai_score: null,
  chapter_id: 1,
  created_at: createdAt,
  file_path: previewImage,
  file_size: 900000,
  height: 1024,
  id: index + 1,
  image_type: "manga_page",
  is_favorite: index % 100 === 0,
  is_system_managed: false,
  metadata_json: {
    generation_task_id: "task-" + (index + 1),
    provider_request_id: "request-" + (index + 1),
    reference_image_ids: [index % profileCount + 1],
    stage: "image",
  },
  model: "fixture-image-model-v1",
  panel_id: "page-" + (index % 12 + 1),
  project_id: 1,
  prompt: "asset-" + (index + 1) + " " + textFor(index),
  width: 1024,
})) as unknown as GeneratedImage[];
const page = {
  prompt: { id: 1 },
  referenceBindings: Array.from({ length: bindingCount }, (_, index) => ({
    characterName: "Character " + (index + 1),
    systemAsset: null,
    userAddedAssets: [],
  })),
  referenceDiscovery: null,
} as unknown as MangaPagePrompt;

function hashText(value: string): string {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash = Math.imul(hash ^ value.charCodeAt(index), 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

const root = createRoot(document.getElementById("root")!);
const ReferenceBindingsHarness: React.FC = () => {
  const [selectedCandidateByCharacter, setSelectedCandidateByCharacter] =
    useState<Record<string, string>>({});
  return (
    <I18nProvider initialLocale="zh-CN">
      <ImageLightboxProvider>
        <ReferenceBindingsPanel
          page={page}
          profiles={profiles}
          portraits={portraits}
          projectImages={projectImages}
          chapterLocked={false}
          selectedCandidateByCharacter={selectedCandidateByCharacter}
          onSelectCandidate={(bindingKey, value) =>
            setSelectedCandidateByCharacter((current) => ({
              ...current,
              [bindingKey]: value,
            }))
          }
          onBindReferenceImage={() => undefined}
        />
      </ImageLightboxProvider>
    </I18nProvider>
  );
};
const startedAt = performance.now();
flushSync(() => root.render(<ReferenceBindingsHarness />));
const mountMs = performance.now() - startedAt;
const preciseMemory = performance as Performance & {
  memory?: { usedJSHeapSize: number };
};
(window as Window & { gc?: () => void }).gc?.();
const heapAfterMountBytes = preciseMemory.memory?.usedJSHeapSize ?? null;
const searchInput = document.querySelector<HTMLInputElement>("input[placeholder]");
let searchMs = 0;
if (searchInput && mode !== "default") {
  const valueSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  const searchStartedAt = performance.now();
  valueSetter?.call(searchInput, targetQuery);
  flushSync(() => searchInput.dispatchEvent(new Event("input", { bubbles: true })));
  searchMs = performance.now() - searchStartedAt;
}
const firstSelect = document.querySelector<HTMLButtonElement>('button[role="combobox"]');
const openStartedAt = performance.now();
if (firstSelect && (mode === "measure" || mode === "search" || mode === "selected")) {
  flushSync(() => firstSelect.click());
} else if (firstSelect && mode === "keyboard" && searchInput) {
  firstSelect.focus();
  flushSync(() => firstSelect.dispatchEvent(new KeyboardEvent("keydown", {
    key: "ArrowDown",
    bubbles: true,
    cancelable: true,
  })));
}
const openMs = performance.now() - openStartedAt;
const optionNodes = [...document.querySelectorAll<HTMLElement>("#a4n-portal-root [role='option']")];
const matchingOption = searchInput
  ? optionNodes.find((option) => option.textContent?.includes(targetQuery))
  : undefined;
const optionTextHash = hashText(optionNodes.map((option) => option.textContent ?? "").join("\n"));
(window as Window & { gc?: () => void }).gc?.();
const heapAfterOpenBytes = preciseMemory.memory?.usedJSHeapSize ?? null;
if (matchingOption && (mode === "measure" || mode === "selected")) {
  flushSync(() => matchingOption.click());
}
if (firstSelect && mode === "keyboard" && searchInput) {
  flushSync(() => firstSelect.dispatchEvent(new KeyboardEvent("keydown", {
    key: "Enter",
    bubbles: true,
    cancelable: true,
  })));
}
const firstArticle = document.querySelector("article");
const addButton = firstArticle
  ? [...firstArticle.querySelectorAll<HTMLButtonElement>("button")]
      .find((button) => button.textContent?.trim() === "添加")
  : undefined;
const selectedPreviewVisible =
  firstArticle?.querySelector('img[alt*="图片 #"], img[alt*="立绘 #"]') != null;
(window as Window & { gc?: () => void }).gc?.();
const report = {
  success: mode === "default"
    ? optionNodes.length === 0
    : searchInput
      ? optionNodes.length >= 2 &&
        optionNodes.length <= 101 &&
        matchingOption != null &&
        (mode === "search" || (addButton?.disabled === false && selectedPreviewVisible))
      : optionNodes.length === imageCount + portraits.length + 1,
  imageCount,
  bindingCount,
  portraitCount: portraits.length,
  mode,
  searchMode: searchInput != null,
  searchQuery: searchInput ? targetQuery : null,
  searchMs: Number(searchMs.toFixed(2)),
  comboboxCount: document.querySelectorAll('button[role="combobox"]').length,
  expanded: firstSelect?.getAttribute("aria-expanded"),
  selectedAddEnabled: addButton?.disabled === false,
  selectedPreviewVisible,
  listboxCount: document.querySelectorAll("#a4n-portal-root [role='listbox']").length,
  mountMs: Number(mountMs.toFixed(2)),
  heapAfterMountBytes,
  openMs: Number(openMs.toFixed(2)),
  heapAfterOpenBytes,
  optionCount: optionNodes.length,
  optionTextHash,
  searchStatus: document.querySelector('[role="status"]')?.textContent ?? null,
  overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
};
document.getElementById("manga-reference-report")!.textContent = JSON.stringify(report);
