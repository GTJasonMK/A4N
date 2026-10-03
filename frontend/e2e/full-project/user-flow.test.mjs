import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { captureBrowserDiagnostics, visibleElements, waitAndCloseProgress } from "./user-flow.mjs";

test("element scans tolerate replaced nodes but propagate transport failures", async () => {
  try {
    globalThis.$$ = async () => [{ isDisplayed: async () => { throw new Error("stale element reference"); } }];
    assert.deepEqual(await visibleElements("button"), []);
    globalThis.$$ = async () => [{ isDisplayed: async () => { throw new Error("session disconnected"); } }];
    await assert.rejects(visibleElements("button"), /session disconnected/);
  } finally {
    delete globalThis.$$;
  }
});

test("progress requires a terminal success and exposes failure or cancellation without retrying", async () => {
  let phase = 0;
  let closed = false;
  let progressText = "生成首版蓝图";
  const close = {
    isDisplayed: async () => true,
    getAttribute: async () => "关闭弹窗",
    getText: async () => "",
    click: async () => {
      assert.equal(phase, 2, "a title and 100% progress cannot close a running task");
      closed = true;
    },
  };
  const bar = { isDisplayed: async () => true, getAttribute: async () => "100" };
  const dialog = {
    isDisplayed: async () => !closed,
    getText: async () => progressText,
    $$: async (selector) => selector === "button" ? (phase === 2 ? [close] : []) : (phase > 0 ? [bar] : []),
  };
  globalThis.$$ = async () => [dialog];
  globalThis.browser = {
    execute: async () => !closed,
    waitUntil: async (condition) => {
      for (let attempt = 0; attempt < 3; attempt += 1) {
        if (await condition()) return;
        phase += 1;
      }
      assert.fail("progress never reached its expected terminal state");
    },
  };
  try {
    await waitAndCloseProgress("生成首版蓝图");
    assert.equal(closed, true);
    for (const status of ["生成失败", "已取消", "已停止"]) {
      phase = 2;
      closed = false;
      progressText = `生成首版蓝图 ${status}`;
      await assert.rejects(waitAndCloseProgress("生成首版蓝图"), /异步任务失败/);
      assert.equal(closed, false, "failed progress must remain visible for diagnostics");
    }
  } finally {
    delete globalThis.$$;
    delete globalThis.browser;
  }
});

test("browser acceptance exposes hidden failures, redacts credentials, and rejects missing diagnostics", async () => {
  mkdirSync(new URL("../../../data/", import.meta.url), { recursive: true });
  const directory = mkdtempSync(new URL("../../../data/browser-diagnostics-", import.meta.url));
  const originalFetch = globalThis.fetch;
  globalThis.browser = { sessionId: "test", options: { protocol: "http", hostname: "localhost", port: 1, path: "/" } };
  const logs = {
    browser: [{ level: "WARNING", message: "layout hint" }, { level: "SEVERE", message: "Uncaught Error" }],
    performance: [
      { method: "Network.requestWillBeSent", params: { requestId: "1", request: { url: "http://localhost/api/test?token=hidden", postData: JSON.stringify({ runtime: { api_key: "private", password: "private" }, input: "keep" }) } } },
      { method: "Network.responseReceived", params: { requestId: "1", response: { url: "http://localhost/api/test", status: 500 } } },
    ].map((message) => ({ message: JSON.stringify({ message }) })),
  };
  try {
    globalThis.fetch = async (_url, options) => ({ ok: true, json: async () => ({ value: logs[JSON.parse(options.body).type] }) });
    const errors = await captureBrowserDiagnostics(join(directory, "stage"));
    assert.deepEqual(errors.map((entry) => entry.message ?? entry.status), [500, "Uncaught Error"]);
    const evidence = readFileSync(join(directory, "stage-performance.json"), "utf8");
    assert.doesNotMatch(evidence, /private|hidden|\?token/);
    assert.equal(JSON.parse(evidence)[0].data.input, "keep");
    logs.browser = [{ level: "SEVERE", source: "network", message: "http://localhost/api/generation-tasks/task-1/events?attempt=1 - Failed to load resource: the server responded with a status of 409 (Conflict)" }];
    logs.performance = [
      { url: "http://localhost/api/generation-tasks/task-1/events?attempt=1", status: 409 },
      { url: "http://localhost/api/generation-tasks/task-1/snapshot?attempt=1", status: 200 },
    ].map((response, index) => ({ message: JSON.stringify({ message: { method: "Network.responseReceived", params: { requestId: String(index), response } } }) }));
    assert.deepEqual(await captureBrowserDiagnostics(join(directory, "recovered")), []);
    const snapshot = logs.performance[1].message;
    logs.performance[1].message = snapshot.replace("snapshot?attempt=1", "snapshot?attempt=2");
    assert.ok((await captureBrowserDiagnostics(join(directory, "wrong-attempt"))).length > 0, "a snapshot from another attempt cannot prove recovery");
    logs.performance[1].message = snapshot;
    logs.performance.pop();
    assert.ok((await captureBrowserDiagnostics(join(directory, "unrecovered"))).length > 0, "409 without a successful snapshot must fail");
    globalThis.fetch = async () => ({ ok: false, status: 503 });
    await assert.rejects(captureBrowserDiagnostics(join(directory, "missing")), /503/);
  } finally {
    globalThis.fetch = originalFetch;
    delete globalThis.browser;
    rmSync(directory, { recursive: true });
  }
});

test("browser acceptance allows only the final expired poll after live progress was read", async () => {
  const originalFetch = globalThis.fetch;
  const taskId = "chapter-analysis-78-e2e";
  const progressUrl = "http://localhost/api/commands/get_workflow_progress_events";
  const request = (requestId) => ({
    method: "Network.requestWillBeSent",
    params: {
      requestId,
      request: {
        url: progressUrl,
        postData: JSON.stringify({ input: { task_id: taskId } }),
      },
    },
  });
  const response = (requestId, status) => ({
    method: "Network.responseReceived",
    params: { requestId, response: { url: progressUrl, status } },
  });
  const performanceLogs = [request("ok"), response("ok", 200), request("expired"), response("expired", 400)]
    .map((message) => ({ message: JSON.stringify({ message }) }));
  const browserLogs = [
    {
      level: "SEVERE",
      source: "network",
      message: `${progressUrl} - Failed to load resource: the server responded with a status of 400 (Bad Request)`,
    },
    {
      level: "SEVERE",
      source: "console-api",
      message: `[apiInvoke] get_workflow_progress_events task_id=${taskId}`,
    },
  ];
  const directory = mkdtempSync(new URL("../../../data/browser-progress-poll-", import.meta.url));
  globalThis.browser = { sessionId: "test", options: { protocol: "http", hostname: "localhost", port: 1, path: "/" } };
  globalThis.fetch = async (_url, options) => ({
    ok: true,
    json: async () => ({
      value: JSON.parse(options.body).type === "browser" ? browserLogs : performanceLogs,
    }),
  });
  try {
    assert.deepEqual(await captureBrowserDiagnostics(join(directory, "terminal")), []);
    performanceLogs.splice(0, 2);
    assert.ok((await captureBrowserDiagnostics(join(directory, "missing"))).length > 0);
  } finally {
    globalThis.fetch = originalFetch;
    delete globalThis.browser;
    rmSync(directory, { recursive: true });
  }
});
