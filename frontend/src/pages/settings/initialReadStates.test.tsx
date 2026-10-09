import { renderToStaticMarkup } from "react-dom/server";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n";
import { SessionManager } from "./about/AccountPanels";
import { QueueTab } from "./QueueTab";
import { LlmTab } from "./LlmTab";
import { LogsTab } from "./logs/LogsTab";
import { PromptTab } from "./PromptTab";
import { PromptAssemblyTab } from "./PromptAssemblyTab";
import { ThemeTab } from "./ThemeTab";
import { ProjectsSection } from "./admin/ProjectsSection";

vi.mock("../../store/auth", () => ({
  useAuthStore: (selector?: (state: { user: { id: number; role: string } }) => unknown) => {
    const state = { user: { id: 1, role: "admin" } };
    return selector ? selector(state) : state;
  },
}));

function render(element: ReactNode): string {
  return renderToStaticMarkup(
    <I18nProvider initialLocale="zh-CN">{element}</I18nProvider>,
  );
}

describe("initial reads", () => {
  it("does not render an empty session list before the first response", () => {
    const html = render(<SessionManager />);

    expect(html).toContain("正在检查会话");
    expect(html).toContain("加载…");
    expect(html).not.toContain("没有其他活动会话");
  });

  it("does not render empty admin project or queue states before loading", () => {
    const projects = render(<ProjectsSection />);
    const queue = render(<QueueTab />);

    expect(projects).toContain("正在加载项目…");
    expect(projects).not.toContain("暂无项目");
    expect(queue).toContain("正在加载最近任务…");
    expect(queue).not.toContain("当前没有运行中的工作流");
    expect(queue).not.toContain("暂无任务");
  });

  it("keeps prompt, model, theme, and log reads unknown until their first response", () => {
    const prompts = render(<PromptTab />);
    const assembly = render(<PromptAssemblyTab />);
    const llm = render(<LlmTab />);
    const themes = render(<ThemeTab />);
    const logs = render(<LogsTab />);

    expect(prompts).toContain("加载…");
    expect(prompts).not.toContain("该分类下没有匹配的提示词");
    expect(assembly).toContain("加载…");
    expect(assembly).not.toContain("该分类下没有匹配的提示词");
    expect(llm).toContain("加载中…");
    expect(llm).not.toContain("选择“新增”添加配置。");
    expect(themes).toContain("正在加载主题…");
    expect(themes).not.toContain("尚未配置任何主题。");
    expect(logs).toContain("加载中…");
    expect(logs).not.toContain("暂无调用记录");
    expect(logs).not.toContain("暂无语言模型调用");
  });
});
