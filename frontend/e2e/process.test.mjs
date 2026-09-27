import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { test } from "node:test";
import { startVite, stopProcess } from "./process.mjs";

test("Vite readiness cannot borrow an existing server on the requested port", async () => {
  const existing = createServer((_request, response) => response.end("other server"));
  existing.listen(0, "127.0.0.1");
  await once(existing, "listening");
  const port = existing.address().port;
  try {
    await assert.rejects(startVite(port), /Vite 提前退出/);
    assert.equal(await (await fetch(`http://127.0.0.1:${port}`)).text(), "other server");
  } finally {
    existing.closeAllConnections();
    await new Promise((resolve) => existing.close(resolve));
  }
});

test("process cleanup waits for exit and terminates an unresponsive owned child", { skip: process.platform === "win32" }, async () => {
  const child = spawn(process.execPath, ["-e", 'process.on("SIGTERM", () => {}); setInterval(() => {}, 1000); console.log("ready");'], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  try {
    await once(child.stdout, "data");
    await stopProcess(child);
    assert.equal(child.signalCode, "SIGKILL");
    await stopProcess(child);
  } finally {
    await stopProcess(child);
  }
});
