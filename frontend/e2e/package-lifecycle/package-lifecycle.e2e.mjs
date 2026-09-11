import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";

const username = process.env.A4N_BROWSER_E2E_USERNAME;
const password = process.env.A4N_BROWSER_E2E_PASSWORD;

async function visibleButton(label) {
  const buttons = await browser.$$("button");
  for (const button of buttons) {
    if (!(await button.isDisplayed().catch(() => false))) continue;
    const text = (await button.getText().catch(() => "")).trim();
    if (typeof label === "string" ? text === label : label.test(text)) return button;
  }
  throw new Error(`找不到可见按钮：${label}`);
}

async function packageCard(packageId) {
  const card = await browser.$(`li[data-package-id="${packageId}"]`);
  await card.waitForDisplayed({ timeout: 30_000 });
  return card;
}

async function packageSnapshot(packageId) {
  return await browser.execute((id) => {
    const card = [...document.querySelectorAll("li[data-package-id]")]
      .find((item) => item.getAttribute("data-package-id") === id);
    if (!card) return null;
    const rawStatusVersion = card.getAttribute("data-status-version");
    return {
      installation: card.getAttribute("data-installation"),
      hostState: card.getAttribute("data-host-state"),
      statusVersion: rawStatusVersion == null ? null : Number(rawStatusVersion),
      operationId: card.getAttribute("data-operation-id"),
    };
  }, packageId);
}

async function packageState(packageId) {
  return (await packageSnapshot(packageId))?.hostState ?? "";
}

async function waitForPackageState(packageId, state) {
  await browser.waitUntil(
    async () => (await packageState(packageId)) === state,
    {
      timeout: 90_000,
      interval: 250,
      timeoutMsg: `功能包 ${packageId} 未进入 ${state}：${JSON.stringify(await packageSnapshot(packageId))}`,
    },
  );
}

async function setPackageState(packageId, state) {
  await browser.waitUntil(async () => {
    const current = await packageState(packageId);
    return current !== "enabling" && current !== "disabling";
  }, {
    timeout: 90_000,
    interval: 250,
    timeoutMsg: `功能包 ${packageId} 仍处于生命周期过渡状态：${JSON.stringify(await packageSnapshot(packageId))}`,
  });

  if (await packageState(packageId) === state) return;

  const card = await packageCard(packageId);
  const buttons = await card.$$("button");
  for (const button of buttons) {
    const text = (await button.getText().catch(() => "")).trim();
    if (
      (state === "disabled" && /^(停用|Disable)$/.test(text))
      || (state === "enabled" && /^(启用|Enable)$/.test(text))
    ) {
      await button.waitForClickable({ timeout: 30_000 });
      await button.click();
      await waitForPackageState(packageId, state);
      return;
    }
  }
  assert.equal(await packageState(packageId), state);
}

async function openPackageSection() {
  await browser.waitUntil(async () => {
    const body = await browser.$("body").getText();
    return body.includes("管理员") || body.includes("Administrator");
  }, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "设置导航未加载",
  });
  await (await visibleButton(/^(管理员|Administrator)$/)).click();
  await browser.waitUntil(async () => {
    const body = await browser.$("body").getText();
    return body.includes("Packages") || body.includes("功能包");
  }, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "管理员设置未加载",
  });
  await (await visibleButton(/^(功能包|Packages)$/)).click();
  await browser.waitUntil(async () => (await browser.$$("li[data-package-id]")).length >= 12, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "功能包列表未加载完整目录",
  });
}

async function setHiddenPreference(packageId, hidden) {
  const readHidden = async () => {
    const input = await (await packageCard(packageId)).$("input[data-package-preference]");
    return await input.isSelected();
  };
  if (await readHidden() === hidden) return;
  const input = await (await packageCard(packageId)).$("input[data-package-preference]");
  await input.click();
  await browser.waitUntil(async () => (await readHidden()) === hidden, {
    timeout: 30_000,
    interval: 100,
    timeoutMsg: `功能包 ${packageId} 的个人隐藏偏好未保存`,
  });
}

async function invokeFrontend(module, exportName, args = []) {
  const result = await browser.executeAsync((moduleName, name, values, done) => {
    import(moduleName)
      .then((api) => {
        const fn = api[name];
        if (typeof fn !== "function") throw new Error(`前端模块没有导出 ${name}`);
        return fn(...values);
      })
      .then((value) => done({ value }), (error) => done({ error: String(error) }));
  }, module, exportName, args);
  assert.equal(result.error, undefined, result.error);
  return result.value;
}

async function packageOverview(packageId) {
  const overview = await invokeFrontend("/src/api/package.ts", "getPackageOverview");
  const item = overview.find((candidate) => candidate.package_id === packageId);
  assert.ok(item, `包概览缺少 ${packageId}`);
  return item;
}

async function loginViaApi(loginUsername, loginPassword, expectedRole) {
  const result = await invokeFrontend("/src/api/auth.ts", "loginUser", [
    loginUsername,
    loginPassword,
  ]);
  assert.equal(result.user.username, loginUsername);
  if (expectedRole) assert.equal(result.user.role, expectedRole);
  await browser.refresh();
}

async function createSecondUser() {
  const suffix = `${Date.now()}-${randomUUID().slice(0, 8)}`;
  const account = {
    username: `package-e2e-${suffix}`,
    email: `package-e2e-${suffix}@example.com`,
    password: `Package-E2E-${randomUUID()}`,
  };
  const user = await invokeFrontend("/src/api/admin/users.ts", "createAdminUser", [{
    ...account,
    role: "user",
  }]);
  assert.equal(user.username, account.username);
  assert.equal(user.role, "user");
  assert.equal(user.is_active, true);
  return account;
}

async function openPersonalPackageSection() {
  await browser.url("/settings#about");
  await browser.waitUntil(async () => {
    const section = await browser.$("[data-package-section]");
    return await section.isDisplayed().catch(() => false);
  }, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "普通用户个人功能包区域未加载",
  });
  await browser.waitUntil(async () => (await browser.$$("li[data-package-id]")).length >= 12, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "普通用户功能包目录未加载完整",
  });
}

async function attemptLifecycleAsCurrentUser(input) {
  return await browser.executeAsync((request, done) => {
    import("/src/utils/runtimeMode.ts")
      .then(({ apiBaseUrl }) => fetch(`${apiBaseUrl()}/commands/start_package_lifecycle`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${localStorage.getItem("a4n:current_access_token") ?? ""}`,
        },
        body: JSON.stringify({ input: request }),
      }))
      .then(async (response) => ({
        status: response.status,
        body: await response.json().catch(() => null),
      }))
      .then((value) => done(value), (error) => done({ error: String(error) }));
  }, input);
}

async function listPackageCommandAvailability(commands) {
  const result = await browser.executeAsync((module, input, done) => {
    import(module)
      .then((api) => api.listPackageCommandAvailability(input))
      .then((value) => done({ value }), (error) => done({ error: String(error) }));
  }, "/src/api/package.ts", { commands });
  assert.equal(result.error, undefined, result.error);
  assert.ok(Array.isArray(result.value), "包准入查询没有返回数组");
  return result.value;
}

function commandAvailability(items, command) {
  const item = items.find((candidate) => candidate.command === command);
  assert.ok(item, `准入查询缺少命令：${command}`);
  return item;
}

describe("功能包客户端生命周期", () => {
  it("管理员停用并恢复 manga 时保留命令准入与维护边界", async () => {
    assert.ok(username, "缺少 A4N_BROWSER_E2E_USERNAME");
    assert.ok(password, "缺少 A4N_BROWSER_E2E_PASSWORD");

    await browser.url("/");
    await (await browser.$("input[autocomplete='username']")).setValue(username);
    await (await browser.$("input[type='password']")).setValue(password);
    await (await browser.$("form button[type='submit']")).click();
    await browser.waitUntil(async () => {
      const body = await browser.$("body").getText();
      return body.includes("创建项目") || body.includes("Create project");
    }, {
      timeout: 30_000,
      interval: 250,
      timeoutMsg: "管理员登录未完成",
    });

    await browser.url("/settings#admin");
    await browser.waitUntil(async () => (await browser.getUrl()).includes("/settings"), {
      timeout: 30_000,
      interval: 250,
    });
    await openPackageSection();

    assert.equal((await browser.$$("li[data-package-id]")).length, 12, "包目录不是 3 个必装包 + 9 个可选包");
    for (const packageId of ["platform", "novel-management", "export"]) {
      const requiredButton = await (await packageCard(packageId)).$("button[data-package-lifecycle]");
      assert.equal(await requiredButton.isEnabled(), false, `必装功能包 ${packageId} 不应可停用`);
      assert.match(await requiredButton.getText(), /必装|Required/);
    }

    await setPackageState("manga", "enabled");
    const enabledSnapshot = await packageSnapshot("manga");
    assert.equal(enabledSnapshot?.installation, "installed");
    assert.equal(enabledSnapshot?.hostState, "enabled");

    await setPackageState("manga", "disabled");
    const disabledSnapshot = await packageSnapshot("manga");
    assert.equal(disabledSnapshot?.hostState, "disabled");
    assert.ok(disabledSnapshot?.operationId, "停用完成后必须保留可查询的操作身份");

    const preference = await (await packageCard("manga")).$("input[data-package-preference]");
    const initialHidden = await preference.isSelected();
    await setHiddenPreference("manga", !initialHidden);
    assert.equal(await packageState("manga"), "disabled", "个人隐藏不能改变 Host 停用状态");
    await setHiddenPreference("manga", initialHidden);

    const disabledAvailability = await listPackageCommandAvailability([
      "start_manga_pipeline",
      "get_manga_pipeline_status",
      "list_manga_page_prompts",
      "cancel_manga_pipeline",
      "duplicate_project",
      "export_project_family_data",
      "delete_project",
    ]);
    const generation = commandAvailability(disabledAvailability, "start_manga_pipeline");
    assert.deepEqual(
      {
        package_id: generation.package_id,
        category: generation.category,
        available: generation.available,
        reason_code: generation.reason_code,
      },
      {
        package_id: "manga",
        category: "generation",
        available: false,
        reason_code: "disabled",
      },
      "停用后不能接收新的漫画生成工作",
    );
    for (const command of ["get_manga_pipeline_status", "list_manga_page_prompts", "cancel_manga_pipeline"]) {
      const item = commandAvailability(disabledAvailability, command);
      assert.equal(item.package_id, "manga", command);
      assert.equal(item.available, true, `${command} 应保留读取/维护准入`);
      assert.equal(item.reason_code, "allowed", command);
    }
    for (const command of ["duplicate_project", "export_project_family_data", "delete_project"]) {
      const item = commandAvailability(disabledAvailability, command);
      assert.equal(item.available, true, `${command} 不应被 manga 停用连带封锁`);
    }

    const disabledOperationId = disabledSnapshot.operationId;
    await browser.refresh();
    await openPackageSection();
    const afterRefresh = await packageSnapshot("manga");
    assert.equal(afterRefresh?.hostState, "disabled");
    assert.equal(afterRefresh?.operationId, disabledOperationId, "刷新后必须找回同一次停用操作");

    await setPackageState("manga", "enabled");
    const recoveredSnapshot = await packageSnapshot("manga");
    assert.equal(recoveredSnapshot?.hostState, "enabled");
    assert.ok(recoveredSnapshot?.operationId);
    assert.notEqual(recoveredSnapshot?.operationId, disabledOperationId, "恢复必须产生独立的操作身份");

    const recoveredGeneration = commandAvailability(
      await listPackageCommandAvailability(["start_manga_pipeline"]),
      "start_manga_pipeline",
    );
    assert.notEqual(recoveredGeneration.reason_code, "disabled", "恢复后不能残留停用准入原因");
  });

  it("普通用户不能启停，个人隐藏只隔离自己的偏好", async () => {
    assert.ok(username, "缺少 A4N_BROWSER_E2E_USERNAME");
    assert.ok(password, "缺少 A4N_BROWSER_E2E_PASSWORD");

    await loginViaApi(username, password, "admin");
    await browser.url("/settings#admin");
    await openPackageSection();
    await setPackageState("manga", "enabled");

    const adminBefore = await packageOverview("manga");
    const adminPreferenceBefore = await (
      await packageCard("manga")
    ).$("input[data-package-preference]")
      .then((input) => input.isSelected());
    const secondUser = await createSecondUser();

    await loginViaApi(secondUser.username, secondUser.password, "user");
    await browser.url("/settings#admin");
    await browser.waitUntil(async () => {
      const body = await browser.$("body").getText();
      return body.includes("需要管理员权限") || body.includes("Administrator permission required");
    }, {
      timeout: 30_000,
      interval: 250,
      timeoutMsg: "普通用户深链管理员页未显示权限边界",
    });
    assert.equal(
      (await browser.$$("button[data-package-lifecycle]")).length,
      0,
      "普通用户深链管理员页不能出现生命周期按钮",
    );

    const nonAdminBefore = await packageOverview("manga");
    for (const action of ["disable", "enable"]) {
      const denial = await attemptLifecycleAsCurrentUser({
        package_id: "manga",
        action,
        operation_id: randomUUID(),
        expected_status_version: nonAdminBefore.status_version,
      });
      assert.equal(denial.error, undefined, denial.error);
      assert.equal(denial.status, 409, `普通用户 ${action} 请求必须被拒绝`);
      assert.equal(denial.body?.code, "package_unavailable");
      assert.equal(denial.body?.retryable, false);
      assert.equal(denial.body?.details?.command, "start_package_lifecycle");
      assert.equal(denial.body?.details?.reason_code, "host_admin_required");
    }

    await openPersonalPackageSection();
    const personalCard = await packageCard("manga");
    assert.equal(
      (await personalCard.$$("button[data-package-lifecycle]")).length,
      0,
      "普通用户个人页只能管理显示偏好，不能启停功能包",
    );
    const beforePreferenceAvailability = commandAvailability(
      await listPackageCommandAvailability(["start_manga_pipeline"]),
      "start_manga_pipeline",
    );

    await setHiddenPreference("manga", true);
    const personalPreferences = await invokeFrontend(
      "/src/api/package.ts",
      "getPackagePreferences",
    );
    assert.ok(personalPreferences.hidden_package_ids.includes("manga"));

    const afterPreference = await packageOverview("manga");
    assert.deepEqual(
      {
        host_state: afterPreference.host_state,
        status_version: afterPreference.status_version,
      },
      {
        host_state: adminBefore.host_state,
        status_version: adminBefore.status_version,
      },
      "个人隐藏不能改变共享 Host 状态或版本",
    );
    const afterPreferenceAvailability = commandAvailability(
      await listPackageCommandAvailability(["start_manga_pipeline"]),
      "start_manga_pipeline",
    );
    assert.deepEqual(
      {
        package_id: afterPreferenceAvailability.package_id,
        category: afterPreferenceAvailability.category,
        available: afterPreferenceAvailability.available,
        reason_code: afterPreferenceAvailability.reason_code,
        status_version: afterPreferenceAvailability.status_version,
      },
      {
        package_id: beforePreferenceAvailability.package_id,
        category: beforePreferenceAvailability.category,
        available: beforePreferenceAvailability.available,
        reason_code: beforePreferenceAvailability.reason_code,
        status_version: beforePreferenceAvailability.status_version,
      },
      "个人隐藏不能改变深链命令准入查询",
    );

    await loginViaApi(username, password, "admin");
    await browser.url("/settings#admin");
    await openPackageSection();
    const adminAfter = await packageOverview("manga");
    assert.deepEqual(
      {
        host_state: adminAfter.host_state,
        status_version: adminAfter.status_version,
      },
      {
        host_state: adminBefore.host_state,
        status_version: adminBefore.status_version,
      },
      "第二用户隐藏后管理员仍看到相同 Host 状态",
    );
    const adminPreferenceAfter = await (
      await packageCard("manga")
    ).$("input[data-package-preference]")
      .then((input) => input.isSelected());
    assert.equal(adminPreferenceAfter, adminPreferenceBefore, "个人偏好不能串到管理员账户");
  });
});
