import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";

const username = process.env.A4N_BROWSER_E2E_USERNAME;
const password = process.env.A4N_BROWSER_E2E_PASSWORD;
const providerUrl = process.env.A4N_BROWSER_E2E_PROVIDER_URL;
const configuredKeys = [];
const activeDialogs = '.a4n-modal-layer:not([data-modal-state="exiting"]) [role="dialog"]';
const failedProgress = /生成失败|任务失败|调用失败|分析失败|同步失败|已取消|已停止/;

function ignoreStaleElement(error) {
  // BiDi identifies a replaced DOM node by its expired shared reference.
  const message = String(error);
  if (!/stale element reference/i.test(message)
      && !/no such node - SharedId "[^"]+" was not found\./i.test(message)) throw error;
  return false;
}

export async function captureBrowserDiagnostics(prefix) {
  const endpoint = `${browser.options.protocol}://${browser.options.hostname}:${browser.options.port}${browser.options.path.replace(/\/$/, "")}/session/${browser.sessionId}/se/log`;
  const redact = (value, key = "") => {
    if (/password|token|secret|api_key/i.test(key)) return value ? "[present]" : value;
    if (Array.isArray(value)) return value.map((item) => redact(item));
    if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([child, item]) => [child, redact(item, child)]));
    return value;
  };
  const logs = {};
  for (const type of ["browser", "performance"]) {
    const response = await fetch(endpoint, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ type }), signal: AbortSignal.timeout(3000) });
    assert.ok(response.ok, `ChromeDriver ${type} logs: ${response.status}`);
    const raw = (await response.json()).value;
    const entries = type === "browser" ? raw : raw.map((item) => JSON.parse(item.message).message)
      .filter((item) => item.method === "Network.requestWillBeSent" || item.method === "Network.responseReceived")
      .map((item) => {
        const request = item.params.request ?? item.params.response;
        let data = request.postData;
        if (data) { try { data = redact(JSON.parse(data)); } catch { data = "[unparsed request body]"; } }
        const url = new URL(request.url);
        return { method: item.method, requestId: item.params.requestId, url: url.origin + url.pathname, attempt: url.searchParams.get("attempt"), status: request.status, data };
      });
    writeFileSync(`${prefix}-${type}.json`, JSON.stringify(redact(entries), null, 2));
    logs[type] = entries;
  }
  const progressTaskByRequestId = new Map();
  const progressResponseStatuses = new Map();
  for (const entry of logs.performance) {
    if (entry.url.endsWith("/api/commands/get_workflow_progress_events")) {
      if (entry.method === "Network.requestWillBeSent") {
        const taskId = entry.data?.input?.task_id;
        if (typeof taskId === "string") progressTaskByRequestId.set(entry.requestId, taskId);
      } else if (entry.method === "Network.responseReceived") {
        const taskId = progressTaskByRequestId.get(entry.requestId);
        if (taskId) {
          const statuses = progressResponseStatuses.get(taskId) ?? [];
          statuses.push({ requestId: entry.requestId, status: entry.status });
          progressResponseStatuses.set(taskId, statuses);
        }
      }
    }
  }
  const ignoredTerminalProgressRequests = new Set();
  const ignoredTerminalProgressTasks = new Set();
  for (const [taskId, statuses] of progressResponseStatuses) {
    if (statuses.at(-1)?.status === 400 && statuses.at(-2)?.status === 200) {
      // A completed synchronous host command may evict its transient task before the final auxiliary poll returns.
      ignoredTerminalProgressRequests.add(statuses.at(-1).requestId);
      ignoredTerminalProgressTasks.add(taskId);
    }
  }
  // A replay cursor outside retention explicitly negotiates a snapshot with HTTP 409.
  // Only a successful snapshot of that same task/attempt proves this recovery completed.
  const recoveries = logs.performance.flatMap((entry, index) => {
    if (entry.status !== 409 || !/\/generation-tasks\/[^/]+\/events$/.test(entry.url)) return [];
    const snapshot = logs.performance.slice(index + 1).find((row) =>
      row.url === entry.url.replace(/\/events$/, "/snapshot") && row.attempt === entry.attempt && row.status === 200);
    return snapshot ? [{ ...entry, snapshotRequestId: snapshot.requestId }] : [];
  });
  writeFileSync(`${prefix}-recoveries.json`, JSON.stringify(recoveries, null, 2));
  const recovered = new Set(recoveries.map((entry) => entry.requestId));
  const errors = logs.performance.filter((entry) =>
    entry.status >= 400
    && !recovered.has(entry.requestId)
    && !ignoredTerminalProgressRequests.has(entry.requestId));
  errors.push(...logs.browser.filter((entry) => {
    if (entry.level !== "SEVERE") return false;
    if (
      entry.source === "console-api"
      && entry.message.includes("get_workflow_progress_events")
      && [...ignoredTerminalProgressTasks].some((taskId) => entry.message.includes(taskId))
    ) return false;
    if (
      entry.source === "network"
      && entry.message.includes("/api/commands/get_workflow_progress_events")
      && ignoredTerminalProgressRequests.size > 0
      && logs.performance
        .filter((row) => row.url.endsWith("/api/commands/get_workflow_progress_events") && row.status >= 400)
        .every((row) => ignoredTerminalProgressRequests.has(row.requestId))
    ) return false;
    if (entry.source !== "network" || !entry.message.includes("status of 409")) return true;
    const address = /^https?:\/\/\S+/.exec(entry.message)?.[0];
    if (!address) return true;
    const url = new URL(address);
    return !recoveries.some((row) => row.url === url.origin + url.pathname && row.attempt === url.searchParams.get("attempt"));
  }));
  return redact(errors);
}

async function visibleElements(selector, scope = null) {
  const elements = scope ? await scope.$$(selector) : await $$(selector);
  const visible = [];
  for (const element of elements) {
    if (await element.isDisplayed().catch(ignoreStaleElement)) visible.push(element);
  }
  return visible;
}

async function firstVisibleContaining(selector, text, scope = null) {
  const elements = await visibleElements(selector, scope);
  for (const element of elements) {
    try {
      if ((await element.getText()).includes(text)) return element;
    } catch (error) {
      ignoreStaleElement(error);
    }
  }
  return null;
}

async function buttonMatching(matcher, scope = null, timeout = 30_000) {
  let found = null;
  const scan = async () => {
    const buttons = await visibleElements("button", scope);
    for (const button of buttons) {
      let text;
      let aria;
      let title;
      try {
        text = await button.getText();
        if (!text) text = await browser.execute((element) => element.textContent ?? "", button);
        aria = (await button.getAttribute("aria-label")) ?? "";
        title = (await button.getAttribute("title")) ?? "";
      } catch (error) {
        ignoreStaleElement(error);
        continue;
      }
      const haystack = `${text} ${aria} ${title}`;
      if (typeof matcher === "string" ? haystack.includes(matcher) : matcher.test(haystack)) {
        found = button;
        return true;
      }
    }
    return false;
  };
  if (!(await scan()) && timeout > 0) {
    await browser.waitUntil(scan, { timeout, interval: 250, timeoutMsg: `找不到可见按钮：${matcher}` });
  }
  if (!found) throw new Error(`找不到可见按钮：${matcher}`);
  return found;
}

async function clickButton(matcher, scope = null) {
  process.stdout.write(`[user-flow] click ${matcher}\n`);
  const button = await buttonMatching(matcher, scope);
  await button.waitForClickable({ timeout: 30_000 });
  await button.click();
  return button;
}

async function currentDialog(timeout = 30_000) {
  let dialogs = await visibleElements(activeDialogs);
  if (!dialogs.length && timeout > 0) {
    await browser.waitUntil(
      async () => {
        dialogs = await visibleElements(activeDialogs);
        return dialogs.length > 0;
      },
      { timeout, interval: 250, timeoutMsg: "当前没有可见对话框" },
    );
  }
  if (!dialogs.length) throw new Error("当前没有可见对话框");
  return dialogs[dialogs.length - 1];
}

async function fieldByLabel(label, tag = "input", scope = null) {
  const root = scope ?? (await visibleElements(activeDialogs)).at(-1) ?? await $("body");
  // An accessible label may intentionally be sr-only.  Filter the associated
  // control by visibility, not the label node itself, otherwise the browser
  // test rejects valid mobile/compact forms.
  const labels = await root.$$("label");
  for (const item of labels) {
    let text;
    try {
      // WebDriver's rendered-text algorithm can return an empty string for a
      // sr-only label.  DOM textContent still preserves the accessible contract.
      text = await browser.execute((element) => element.textContent ?? "", item);
    } catch (error) {
      ignoreStaleElement(error);
      continue;
    }
    if (!text.includes(label)) continue;
    const controls = await item.$$(tag);
    for (const control of controls) {
      if (await control.isDisplayed().catch(ignoreStaleElement)) return control;
    }
    const forId = await item.getAttribute("for");
    if (forId) {
      // React-generated ids can contain ':' and therefore cannot be used
      // safely in a CSS id selector without additional escaping.
      const candidates = await root.$$(`${tag}[id]`);
      for (const control of candidates) {
        if ((await control.getAttribute("id")) === forId && await control.isDisplayed().catch(ignoreStaleElement)) {
          return control;
        }
      }
    }
  }
  const aria = await visibleElements(`${tag}[aria-label*=${JSON.stringify(label)}]`, root);
  if (aria.length) return aria[0];
  throw new Error(`找不到字段：${label}`);
}

async function setField(label, value, tag = "input", scope = null) {
  const field = await fieldByLabel(label, tag, scope);
  await field.waitForDisplayed({ timeout: 30_000 });
  await field.clearValue();
  await field.setValue(String(value));
  return field;
}

async function waitForBodyText(text, timeout = 60_000) {
  await browser.waitUntil(
    async () => (await $("body").getText()).includes(text),
    {
      timeout,
      interval: 500,
      timeoutMsg: `页面未出现文本：${text}；当前页面：${(await $("body").getText()).slice(-800)}`,
    },
  );
}

async function waitForBodyMatch(pattern, timeout = 60_000) {
  await browser.waitUntil(
    async () => pattern.test(await $("body").getText()),
    { timeout, interval: 500, timeoutMsg: `页面未出现文本模式：${pattern}` },
  );
}

async function waitForRoute(pattern, timeout = 60_000) {
  await browser.waitUntil(
    async () => pattern.test(await browser.getUrl()),
    { timeout, interval: 500, timeoutMsg: `未进入路由：${pattern}` },
  );
}

async function waitForNoDialog(timeout = 30_000) {
  await browser.waitUntil(
    async () => {
      const dialogs = await $$(activeDialogs);
      for (const dialog of dialogs) {
        if (await dialog.isDisplayed().catch(ignoreStaleElement)) return false;
      }
      return true;
    },
    { timeout, interval: 500, timeoutMsg: "对话框未关闭" },
  );
}

async function closeDialog() {
  const dialog = (await visibleElements(activeDialogs)).at(-1);
  if (!dialog) return;
  await clickButton("关闭", dialog);
  await waitForNoDialog();
}

async function waitAndCloseProgress(successText, timeout = 480_000) {
  process.stdout.write(`[user-flow] wait for ${successText}\n`);
  let progressText = "";
  await browser.waitUntil(
    async () => {
      // React replaces the progress dialog while a task advances.  Resolve
      // the node and read it in one retryable transaction so a replacement
      // cannot abort the whole acceptance flow with a stale-element error.
      try {
        const dialogs = await visibleElements(activeDialogs);
        // The launching form has the same title as the progress task.  Only
        // a dialog containing the progress bar is authoritative for task
        // completion; otherwise the title would make this return immediately.
        for (const dialog of [...dialogs].reverse()) {
          const bars = await visibleElements('[role="progressbar"]', dialog);
          if (!bars.length) continue;
          progressText = await dialog.getText();
          const value = await bars[0].getAttribute("aria-valuenow");
          if (failedProgress.test(progressText)) return true;
          // The dialog title is also included in `getText()`, so a 100% running
          // task can contain successText before it has entered its terminal
          // state.  A close control is rendered only for terminal states.
          let hasCloseButton = false;
          for (const button of await visibleElements("button", dialog)) {
            const label = (await button.getAttribute("aria-label")) ?? "";
            if (label.includes("关闭") || (await button.getText()).trim() === "关闭") {
              hasCloseButton = true;
              break;
            }
          }
          return value === "100" && progressText.includes(successText) && hasCloseButton;
        }
        return false;
      } catch (error) {
        ignoreStaleElement(error);
        return false;
      }
    },
    { timeout, interval: 1_000, timeoutMsg: `异步任务未完成：${successText}` },
  );
  assert.ok(progressText.includes(successText), `异步任务未成功：${progressText.slice(-1_000)}`);
  assert.doesNotMatch(progressText, failedProgress, `异步任务失败：${progressText}`);
  await browser.waitUntil(
    async () => {
      try {
        const hasProgressDialog = await browser.execute((selector) =>
          Array.from(document.querySelectorAll(selector)).some((dialog) => {
            const bar = dialog.querySelector('[role="progressbar"]');
            if (!bar) return false;
            const rect = bar.getBoundingClientRect();
            const style = getComputedStyle(bar);
            return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none";
          }), activeDialogs,
        );
        if (!hasProgressDialog) return true;
        const dialogs = await visibleElements(activeDialogs);
        let progress = null;
        for (const dialog of [...dialogs].reverse()) {
          if ((await visibleElements('[role="progressbar"]', dialog)).length > 0) {
            progress = dialog;
            break;
          }
        }
        if (progress) {
          const buttons = await visibleElements("button", progress);
          for (const button of buttons) {
            const label = (await button.getAttribute("aria-label")) ?? "";
            if (label.includes("关闭") || (await button.getText()).trim() === "关闭") {
              await button.click();
              return false;
            }
          }
          return false;
        }
        return true;
      } catch (error) {
        ignoreStaleElement(error);
        return false;
      }
    },
    { timeout: 30_000, interval: 250, timeoutMsg: `异步任务结果对话框未关闭：${successText}` },
  );
}

async function tab(label) {
  process.stdout.write(`[user-flow] tab ${label}\n`);
  const scan = async () => {
    try {
      const tabs = await visibleElements('[role="tab"]');
      const candidates = [];
      for (const item of tabs) {
        try {
          const text = await item.getText();
          if (text.includes(label)) candidates.push({ item, exact: text.trim() === label });
        } catch (error) {
          ignoreStaleElement(error);
        }
      }
      const visibleMatch = candidates.find(({ exact }) => exact) ?? candidates[0];
      let ariaMatch = null;
      if (!visibleMatch) {
        for (const item of tabs) {
          try {
            if ((await item.getAttribute("aria-label")) === label) {
              ariaMatch = item;
              break;
            }
          } catch (error) {
            ignoreStaleElement(error);
          }
        }
      }
      const match = visibleMatch?.item ?? ariaMatch;
      if (match) {
        await match.waitForClickable({ timeout: 5_000 });
        await match.click();
        return true;
      }
    } catch (error) {
      ignoreStaleElement(error);
      return false;
    }
    return false;
  };
  await browser.waitUntil(scan, { timeout: 30_000, interval: 250, timeoutMsg: `找不到标签：${label}` });
  await browser.waitUntil(async () => {
    try {
      const tabs = await visibleElements('[role="tab"]');
      for (const item of tabs) {
        const aria = (await item.getAttribute("aria-label")) ?? "";
        const text = await item.getText();
        if ((aria === label || text.includes(label)) && (await item.getAttribute("aria-selected")) === "true") {
          return true;
        }
      }
    } catch (error) {
      ignoreStaleElement(error);
      return false;
    }
    return false;
  }, {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: `标签未选中：${label}`,
  });
}

async function sectionContaining(text) {
  let found = null;
  const scan = async () => {
    const sections = await visibleElements("section");
    for (const section of sections) {
      if ((await section.getText()).includes(text)) {
        found = section;
        return true;
      }
    }
    return false;
  };
  await browser.waitUntil(scan, { timeout: 30_000, interval: 250, timeoutMsg: `找不到包含文本的区域：${text}` });
  return found;
}

async function createProject() {
  await clickButton("创建项目");
  await waitForBodyText("新建项目", 30_000);
  await clickButton("自由创作", await currentDialog());
  await waitForRoute(/\/novel\/\d+/);
  const match = (await browser.getUrl()).match(/\/novel\/(\d+)/);
  assert.ok(match, "自由创作项目缺少 ID");
  return Number(match[1]);
}

async function configureServerEmbedding() {
  await browser.url("/settings#admin");
  await waitForBodyText("设置", 60_000);
  await clickButton("管理员");
  await clickButton("配置");
  await clickButton("嵌入");
  await waitForBodyMatch(/嵌入配置|Embedding 配置/);
  const create = await buttonMatching(/新增\s*(Embedding|嵌入).*配置|新建\s*(Embedding|嵌入).*配置/i);
  await create.click();
  await waitForBodyMatch(/新建.*(?:嵌入|Embedding).*配置|嵌入配置|Embedding 配置/);
  await setField("配置名称", "Chromium E2E 嵌入");
  const backend = await fieldByLabel("后端", 'button[role="combobox"]');
  await backend.click();
  const option = await $('//div[@role="option" and normalize-space(.)="OpenAI 兼容 API"]');
  await option.waitForClickable({ timeout: 30_000 });
  await option.click();
  await setField("模型", "a4n-virtual-embedding");
  await setField("接口地址", providerUrl);
  await setField("访问密钥", "a4n-e2e-provider-key");
  await setField("维度", process.env.A4N_E2E_EMBEDDING_DIMENSION ?? "32");
  await setField("批量大小", "32");
  await clickButton("创建");
  await waitForBodyText("Chromium E2E 嵌入", 60_000);
  // 新建后列表不会自动切换当前配置；先通过可见列表项选中刚创建的配置，
  // 再打开该配置的操作菜单。
  const created = await firstVisibleContaining("aside button", "Chromium E2E 嵌入");
  assert.ok(created, "创建的 Embedding 配置未出现在列表中");
  await created.waitForClickable({ timeout: 30_000 });
  await created.click();
  await browser.waitUntil(async () => (await created.getAttribute("aria-current")) === "true", {
    timeout: 30_000,
    interval: 250,
    timeoutMsg: "创建的 Embedding 配置未被选中",
  });
  const more = await buttonMatching("更多操作");
  await more.click();
  await clickButton("设为默认");
  await waitForBodyMatch(/默认配置已设置|已设为默认|设置成功/, 60_000);
}

async function configureSystemProviderKey(tabLabel, title) {
  await browser.url("/settings");
  await waitForBodyText("设置", 60_000);
  // 设置页 hash 只在初次挂载时读取；连续 browser.url('/settings#...')
  // 不保证切换 React 内部 Tab，因此必须走用户可见的侧栏按钮。
  await clickButton(tabLabel);
  await waitForBodyText(title, 60_000);
  const key = await fieldByLabel("访问密钥");
  await key.waitForDisplayed({ timeout: 30_000 });
  await key.setValue("a4n-e2e-provider-key");
  await clickButton(/^保存\s/);
  const kind = { "LLM 服务": "llm", "生图模型": "image", "视频生成": "video" }[tabLabel];
  assert.ok(kind, "unknown provider kind");
  configuredKeys.push(kind);
  let present;
  await browser.waitUntil(async () => {
    present = await browser.execute(() => {
      const settings = JSON.parse(localStorage.getItem("a4n:local-runtime-settings:v1") ?? "{}");
      return Object.fromEntries(["llm", "image", "video"].map((name) => [name, Boolean(settings[`system_${name}_api_key_encrypted`])]));
    });
    return configuredKeys.every((name) => present[name]);
  }, { timeout: 10_000, interval: 100, timeoutMsg: `保存 ${tabLabel} 后个人密钥没有完整保留` }).finally(() => {
    writeFileSync(`${process.env.A4N_BROWSER_E2E_ARTIFACT_DIR}/provider-key-presence.json`, JSON.stringify({ after: tabLabel, configured: configuredKeys, present }));
  });
}


export async function login() {
  assert.ok(username, "缺少 A4N_BROWSER_E2E_USERNAME");
  assert.ok(password, "缺少 A4N_BROWSER_E2E_PASSWORD");

  await browser.url("/");
  const loginUser = await $("input[autocomplete='username']");
  await loginUser.waitForDisplayed({ timeout: 60_000 });
  await loginUser.setValue(username);
  await (await $("input[type='password']")).setValue(password);
  await (await $("form button[type='submit']")).click();
  await waitForBodyMatch(/创建项目|Create project/, 120_000);

  const statusBarToggle = await buttonMatching(/展开快捷工具|Open quick tools/);
  await statusBarToggle.click();
  const languageTrigger = await buttonMatching(/界面语言|Interface language/);
  await languageTrigger.click();
  const chineseOption = await firstVisibleContaining('button[role="menuitem"]', "中文")
    ?? await firstVisibleContaining('button[role="menuitem"]', "Chinese");
  if (!chineseOption) throw new Error("语言菜单缺少简体中文选项");
  await chineseOption.click();
  await waitForBodyText("创建项目", 60_000);
}

export async function prepareSession() {
  assert.ok(providerUrl?.endsWith("/v1"), "缺少 A4N_BROWSER_E2E_PROVIDER_URL（必须是 /v1）");
  await login();

  // Web 端将系统模板的个人密钥保存在浏览器运行时设置中；后续正文、图片和派生流程
  // 的请求都会由这些真实页面配置注入 runtime snapshot。
  await configureSystemProviderKey("LLM 服务", "LLM 配置");
  await configureSystemProviderKey("生图模型", "图片生成配置");
  await configureSystemProviderKey("视频生成", "视频生成配置");
  await configureServerEmbedding();
  // 设置页完成配置后回到项目列表，后续“创建项目”操作必须由项目列表真实触发。
  await browser.url("/");
  await waitForBodyText("创建项目", 60_000);
}

export { visibleElements, firstVisibleContaining, buttonMatching, clickButton, currentDialog, fieldByLabel, setField, waitForBodyText, waitForBodyMatch, waitForRoute, waitForNoDialog, closeDialog, waitAndCloseProgress, tab, sectionContaining, createProject };
