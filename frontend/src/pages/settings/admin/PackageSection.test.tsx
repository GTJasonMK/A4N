import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { readPackageRejection } from "../../../api/package";
import type { PackageOverview } from "../../../api/generated/page";
import { I18nProvider } from "../../../i18n";
import { isActiveLifecycleState, isTerminalLifecycleState, PackageCard } from "./PackageSection";

const packageOverview = (overrides: Partial<PackageOverview> = {}): PackageOverview => ({
  package_id: "manga",
  required: false,
  installation: "installed",
  host_state: "enabled",
  health: "healthy",
  status_version: 4,
  lifecycle: null,
  ...overrides,
});

const renderCard = (props: Partial<React.ComponentProps<typeof PackageCard>> = {}) =>
  renderToStaticMarkup(
    <I18nProvider initialLocale="zh-CN">
      <PackageCard
        item={packageOverview()}
        hidden={false}
        preferencesUnavailable={false}
        canManageLifecycle
        snapshotStale={false}
        busy={false}
        preferenceBusy={false}
        onToggle={() => undefined}
        onHiddenChange={() => undefined}
        {...props}
      />
    </I18nProvider>,
  );

describe("PackageSection", () => {
  it("keeps personal hiding and Host lifecycle controls as separate controls", () => {
    const personal = renderCard({ canManageLifecycle: false, hidden: true });
    expect(personal).toContain('data-package-preference="true"');
    expect(personal).toContain("checked");
    expect(personal).not.toContain("data-package-lifecycle");
    expect(personal).not.toContain("停用");

    const admin = renderCard({ canManageLifecycle: true });
    expect(admin).toContain('data-package-lifecycle="true"');
    expect(admin).toContain('aria-label="停用 manga"');
    expect(admin).toContain('data-package-preference="true"');
  });

  it("shows the server lifecycle reason and operation identity", () => {
    const html = renderCard({
      item: packageOverview({
        lifecycle: {
          operation_id: "operation-version-conflict",
          action: "disable",
          state: "failed",
          requested_version: null,
          status_version: 5,
          reason_code: "version_conflict",
          error: "状态版本已变化",
        },
      }),
    });

    expect(html).toContain('data-operation-id="operation-version-conflict"');
    expect(html).toContain("原因码：version_conflict");
    expect(html).toContain("错误：状态版本已变化");
  });

  it("keeps lifecycle controls disabled while the operation is still active", () => {
    const html = renderCard({
      item: packageOverview({
        lifecycle: {
          operation_id: "operation-waiting-external",
          action: "disable",
          state: "waiting_external",
          requested_version: null,
          status_version: 5,
          reason_code: null,
          error: null,
        },
      }),
    });

    expect(html).toContain('data-operation-id="operation-waiting-external"');
    expect(html).toContain('disabled=""');
  });

  it("marks a failed overview refresh as unknown and disables lifecycle writes", () => {
    const html = renderCard({ snapshotStale: true, preferencesUnavailable: false });

    expect(html).toContain("未知 · 未知 · 未知");
    expect(html).toContain('data-snapshot-stale="1"');
    expect(html).toContain('data-host-state="unknown"');
    expect(html).toContain('data-package-lifecycle="true"');
    expect(html).toContain('data-package-preference="true"');
    expect(html).toContain('disabled=""');
  });

  it("gates display preferences independently from package lifecycle state", () => {
    const staleOverview = renderCard({
      canManageLifecycle: false,
      snapshotStale: true,
      preferencesUnavailable: false,
    });
    expect(staleOverview).toContain('data-package-preference="true"');
    expect(staleOverview).not.toContain('disabled=""');

    const unavailablePreferences = renderCard({
      canManageLifecycle: false,
      snapshotStale: false,
      preferencesUnavailable: true,
    });
    expect(unavailablePreferences).toContain('data-package-preference="true"');
    expect(unavailablePreferences).toContain('disabled=""');
  });

  it("accepts only a structured package rejection envelope", () => {
    const rejection = readPackageRejection({
      code: "package_unavailable",
      message: "功能包 manga 当前不可用",
      retryable: false,
      trace_id: null,
      details: {
        package_id: "manga",
        reason_code: "disabled",
        status_version: 9,
      },
    });
    expect(rejection?.details).toMatchObject({
      package_id: "manga",
      reason_code: "disabled",
      status_version: 9,
    });
    expect(readPackageRejection(new Error("reason_code=disabled"))).toBeNull();
  });

  it("treats only pending lifecycle states as resumable", () => {
    expect(isActiveLifecycleState("waiting_external")).toBe(true);
    expect(isActiveLifecycleState("completed")).toBe(false);
    expect(isTerminalLifecycleState("failed")).toBe(true);
  });
});
