// Read-only reproduction probes; successful assertions confirm the reported gaps.
// Node 24 can execute the production TypeScript module without dependencies.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const sourceRoot = resolve(process.argv[2] ?? ".");
const { reduceAgentEvent } = await import(pathToFileURL(resolve(
  sourceRoot,
  "frontend/src/utils/agentEventReducer.ts",
)));
const snapshot = {
  thread: { id: "thread-1", next_event_sequence: 1 },
  runs: [{ id: "run-1", status: "running", waiting_reason: null }],
  approvals: [],
  executions: [],
  messages: [],
  events: [],
};
const event = (kind, payload) => ({
  thread_id: "thread-1",
  run_id: "run-1",
  sequence: 1,
  kind,
  payload,
});

const approval = reduceAgentEvent(
  snapshot,
  event("approval_requested", { approval_id: "approval-1" }),
  0,
);
assert.equal(approval.needsReload, false);
assert.equal(approval.snapshot.approvals.length, 0);
assert.equal(approval.snapshot.runs[0].status, "running");

const waiting = reduceAgentEvent(
  snapshot,
  event("run_waiting", { waiting_reason: "budget" }),
  0,
);
assert.equal(waiting.needsReload, false);
assert.equal(waiting.snapshot.runs[0].status, "waiting");
assert.equal(waiting.snapshot.runs[0].waiting_reason, null);

console.log(JSON.stringify({
  meaning: "defects reproduced; not a passing product acceptance suite",
  source_root: sourceRoot,
  approval_event_requests_reload: approval.needsReload,
  visible_pending_approvals_after_event: approval.snapshot.approvals.length,
  run_status_after_approval_event: approval.snapshot.runs[0].status,
  waiting_reason_after_budget_event: waiting.snapshot.runs[0].waiting_reason,
}, null, 2));
