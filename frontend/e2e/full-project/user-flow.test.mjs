import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { captureBrowserDiagnostics, visibleElements } from "./user-flow.mjs";

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
