import assert from "node:assert/strict";

const timeout = 5000;
const panel = async () => {
  const view = $('[aria-label="应用更新"]');
  await view.waitForDisplayed({ timeout });
  return view;
};

describe("App update state projection", function () {
  it("UX-1748 distinguishes an update check failure from no available update", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=error&recovery=none");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("update feed unavailable"), { timeout });
    const text = await view.getText();
    assert.equal(text.includes("当前已是最新版本"), false);
    assert.ok(text.includes("更新失败：update feed unavailable"));
    assert.deepEqual(await browser.execute(() => [...window.__appUpdateAudit.calls].sort()), [
      "check_app_update", "get_update_recovery_state",
    ]);
  });

  it("UX-1749 attributes a recovery-state read failure to that result", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=none&recovery=error");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("recovery file unavailable"), { timeout });
    const text = await view.getText();
    assert.ok(text.includes("当前已是最新版本"));
    assert.ok(text.includes("无法读取更新恢复状态：recovery file unavailable"));
    assert.equal(text.includes("更新失败：recovery file unavailable"), false);
  });

  it("UX-1932 retries a failed recovery-state read in place", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=none&recovery=retry");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("recovery file unavailable"), { timeout });
    await (await view.$('.//button[normalize-space(.)="重试"]')).click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__appUpdateAudit.calls.filter((command) => command === "get_update_recovery_state").length === 2,
    ), { timeout });
    assert.equal((await view.getText()).includes("recovery file unavailable"), false);
    assert.ok(await view.$('[role="status"]').isDisplayed());
    await browser.execute(() => window.__appUpdateAudit.recoveryReadResolvers[0]());
    await browser.waitUntil(async () => !(await view.$('[role="status"]').isExisting()), { timeout });
    assert.equal((await view.getText()).includes("无法读取更新恢复状态"), false);
    assert.equal((await view.getText()).includes("当前已是最新版本"), true);
  });

  it("UX-1933 keeps recovery-clear failures on the clear action path", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=none&recovery=failed&clear=error-once");
    const view = await panel();
    const dismiss = await view.$('.//button[normalize-space(.)="关闭更新失败提示"]');
    await dismiss.waitForDisplayed({ timeout });
    await dismiss.click();
    await browser.waitUntil(async () => (await view.getText()).includes("recovery state file is read-only"), { timeout });
    const text = await view.getText();
    assert.ok(text.includes("无法关闭更新恢复状态：recovery state file is read-only"));
    assert.equal(text.includes("无法读取更新恢复状态：recovery state file is read-only"), false);
    assert.ok(await dismiss.isEnabled());
    await dismiss.click();
    await browser.waitUntil(async () => !(await view.getText()).includes("更新未完成"), { timeout });
    assert.equal(await browser.execute(() =>
      window.__appUpdateAudit.calls.filter((command) => command === "clear_update_recovery_state").length,
    ), 2);
  });

  it("UX-1936 blocks settings navigation until recovery-record clearing settles", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=none&recovery=failed&clear=deferred");
    const view = await panel();
    const dismiss = await view.$('.//button[normalize-space(.)="关闭更新失败提示"]');
    await dismiss.waitForDisplayed({ timeout });
    await dismiss.click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__appUpdateAudit.calls.filter((command) => command === "clear_update_recovery_state").length === 1,
    ), { timeout });
    await (await $('[aria-label="离开应用更新页"]')).click();
    assert.ok(await view.isExisting());
    assert.ok((await $("body").getText()).includes("操作仍在进行"));
    await browser.execute(() => window.__appUpdateAudit.clearResolvers[0]());
    await browser.waitUntil(async () => !(await dismiss.isExisting()), { timeout });
    await (await $('[aria-label="离开应用更新页"]')).click();
    await browser.waitUntil(async () => !(await view.isExisting()), { timeout });
  });

  it("UX-1939 scopes an installation error to the version that failed", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=version-change&recovery=none&download=error");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();
    await browser.waitUntil(async () => (await view.getText()).includes("update package unavailable"), { timeout });
    await (await view.$('.//button[normalize-space(.)="检查更新"]')).click();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.1.0"), { timeout });
    assert.equal((await view.getText()).includes("update package unavailable"), false);
  });

  it("UX-1940 refuses to prepare an update when recovery state cannot be read", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=available&recovery=error");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    await browser.waitUntil(async () => (await view.getText()).includes("无法读取更新恢复状态"), { timeout });
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();
    await browser.waitUntil(async () => (await view.getText()).includes("existing update recovery state is unreadable"), { timeout });
    assert.equal(await browser.execute(() => window.__appUpdateAudit.calls.includes("download_app_update")), false);
  });

  it("UX-1750 prevents duplicate recovery-record clears while the command is pending", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=none&recovery=failed&clear=deferred");
    const view = await panel();
    const dismiss = await view.$('.//button[normalize-space(.)="关闭更新失败提示"]');
    await dismiss.waitForDisplayed({ timeout });
    await dismiss.click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__appUpdateAudit.calls.filter((command) => command === "clear_update_recovery_state").length === 1,
    ), { timeout });
    await browser.waitUntil(async () => !(await dismiss.isEnabled()), { timeout });
    await browser.execute(() => {
      [...document.querySelectorAll("button")]
        .find((button) => button.textContent?.includes("关闭更新失败提示"))
        ?.click();
    });
    assert.equal(await browser.execute(() =>
      window.__appUpdateAudit.calls.filter((command) => command === "clear_update_recovery_state").length,
    ), 1, "A disabled dismissal control must not start a second non-idempotent clear");

    await browser.execute(() => window.__appUpdateAudit.clearResolvers[0]());
    await browser.waitUntil(async () => (await view.getText()).includes("当前已是最新版本"), { timeout });
    assert.equal((await view.getText()).includes("更新未完成"), false);
    assert.equal((await view.getText()).includes("更新失败：no update failure record"), false);
  });

  it("UX-1751 keeps the current draft when update preflight fails", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=available&recovery=none&prepare=error");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    const draft = await $('[aria-label="应用更新测试草稿"]');
    const originalDraft = "保留这条设置修改";
    await draft.setValue(originalDraft);
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();

    await browser.waitUntil(async () => (await view.getText()).includes("active generation task prevents update"), { timeout });
    assert.equal(await $('.//div[@role="dialog"]').isDisplayed().catch(() => false), false);
    assert.equal(await draft.getValue(), originalDraft, "A failed preflight must not discard drafts for an update that never started");
    assert.deepEqual(await browser.execute(() => window.__appUpdateAudit.calls.slice(-1)), ["prepare_app_update_install"]);
  });

  it("UX-1751 keeps the current draft when the user cancels a data migration", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=available&recovery=none&prepare=migration");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    const draft = await $('[aria-label="应用更新测试草稿"]');
    const originalDraft = "保留这条设置修改";
    await draft.setValue(originalDraft);
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();

    const migrationDialog = await $('.//div[@role="dialog"]');
    await browser.waitUntil(async () => migrationDialog.isDisplayed(), { timeout });
    assert.ok((await migrationDialog.getText()).includes("media layout migration"));
    await (await migrationDialog.$('.//button[normalize-space(.)="取消"]')).click();
    await browser.waitUntil(async () => !(await migrationDialog.isDisplayed().catch(() => false)), { timeout });
    assert.equal(await draft.getValue(), originalDraft);
    assert.deepEqual(await browser.execute(() => window.__appUpdateAudit.calls.slice(-1)), ["prepare_app_update_install"]);
  });

  it("UX-1751 keeps the current draft when update download fails", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=available&recovery=none&download=error");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    const draft = await $('[aria-label="应用更新测试草稿"]');
    const originalDraft = "保留这条设置修改";
    await draft.setValue(originalDraft);
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();

    await browser.waitUntil(async () => (await view.getText()).includes("update package unavailable"), { timeout });
    assert.equal(await $('.//div[@role="dialog"]').isDisplayed().catch(() => false), false);
    assert.equal(await draft.getValue(), originalDraft);
    assert.deepEqual(await browser.execute(() => window.__appUpdateAudit.calls.slice(-2)), [
      "prepare_app_update_install", "download_app_update",
    ]);
  });

  it("UX-1757 reports a download failure after the update panel is unmounted", async () => {
    await browser.url("/e2e/ui-page-audit/app-update.html?check=available&recovery=none&download=deferred");
    const view = await panel();
    await browser.waitUntil(async () => (await view.getText()).includes("发现新版本 2.0.0"), { timeout });
    await (await view.$('.//button[normalize-space(.)="下载并重启更新"]')).click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__appUpdateAudit.calls.includes("download_app_update"),
    ), { timeout });

    await (await $('[aria-label="离开应用更新页"]')).click();
    await browser.waitUntil(async () => !(await view.isExisting()), { timeout });
    await browser.execute(() => window.__appUpdateAudit.downloadResolvers[0]("Injected detached update download failure"));
    await browser.waitUntil(async () => (await $("body").getText()).includes("Injected detached update download failure"), { timeout });
    assert.ok(await $('[role="alert"]').isDisplayed());
    assert.equal(await panel().isExisting(), false);
  });
});
