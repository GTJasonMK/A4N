import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ChapterSceneSummary } from "../../api/chapter";
import { I18nProvider } from "../../i18n";
import { SceneList } from "./SceneList";

const scene: ChapterSceneSummary = {
  id: 7,
  chapter_id: 3,
  scene_number: 2,
  title: "码头会面",
  content: "正文",
  word_count: 2,
  summary: "摘要",
  scene_type: "对话",
  characters_present: null,
  location: "旧港",
  timeline_position: null,
  transition_from_previous: null,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

function renderSceneList(scenes: ChapterSceneSummary[]) {
  return renderToStaticMarkup(
    <I18nProvider initialLocale="zh-CN">
      <SceneList
        loading={false}
        loadError="读取场景失败"
        scenes={scenes}
        expanded={new Set()}
        editingId={null}
        editForm={{}}
        saving={false}
        mutationBusy={false}
        reorderDisabled
        actingKeys={new Set()}
        onToggleExpand={() => undefined}
        onStartEdit={() => undefined}
        onCancelEdit={() => undefined}
        onEditFormChange={() => undefined}
        onSave={() => undefined}
        onMove={() => undefined}
        onDelete={() => undefined}
      />
    </I18nProvider>,
  );
}

describe("SceneList read errors", () => {
  it("keeps the last successful scenes visible when a refresh fails", () => {
    const html = renderSceneList([scene]);

    expect(html).toContain("读取场景失败");
    expect(html).toContain("码头会面");
  });

  it("does not report an unknown list as empty after the first read fails", () => {
    const html = renderSceneList([]);

    expect(html).toContain("读取场景失败");
    expect(html).not.toContain("本章尚未拆分场景");
  });
});
