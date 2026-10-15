import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "task_queue_history_index_probe";
const index = "idx_task_queue_history_probe_project_created";
const rows = 100_000;
const rounds = 5;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/task-history-index");
const statement = `SELECT * FROM ${table} WHERE project_id = 42 ORDER BY created_at DESC LIMIT 100`;
const containers = execFileSync("docker", ["ps", "--format", "{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => line.split("\t")[1]?.includes(`127.0.0.1:${url.port}->5432/tcp`));
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[0];
async function ready() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try { execFileSync("docker", ["exec", container, "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "a4n_test", "-d", "a4n_test"], { stdio: "ignore" }); return; }
    catch { await delay(100); }
  }
  throw new Error("PostgreSQL did not become ready");
}
await ready();
function sql(value) {
  return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], { input: value, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }).trim();
}
sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id text PRIMARY KEY, task_type text NOT NULL, priority text NOT NULL, status text NOT NULL,
  project_id bigint, root_project_id bigint, source_snapshot_id bigint, task_scope text,
  workflow_run_id bigint, payload jsonb, progress jsonb, error_message text, dag_id text,
  created_at timestamp NOT NULL, started_at timestamp, completed_at timestamp
);
INSERT INTO ${table}
SELECT 'task-' || id, CASE WHEN id % 2 = 0 THEN 'llm_call' ELSE 'pipeline' END,
  CASE WHEN id % 3 = 0 THEN 'high' ELSE 'medium' END,
  CASE WHEN id % 5 = 0 THEN 'completed' ELSE 'running' END,
  CASE WHEN id <= 20000 THEN 42 ELSE (id % 99) + 1 END,
  42, NULL, 'project', id,
  jsonb_build_object('input', repeat('x', 128)),
  jsonb_build_object('progress', id % 100), NULL, NULL,
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second', NULL, NULL
FROM generate_series(1, ${rows}) AS series(id);
ANALYZE ${table};
`);
function setIndex(enabled) {
  sql(enabled
    ? `CREATE INDEX IF NOT EXISTS ${index} ON ${table} (project_id, created_at DESC); ANALYZE ${table};`
    : `DROP INDEX IF EXISTS ${index}; ANALYZE ${table};`);
}
function sample() {
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0];
  const ids = sql(`SELECT id FROM (${statement}) AS latest;`).split("\n");
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan.Plan);
  return { execution_ms: plan["Execution Time"], nodes, ids };
}
const samples = { baseline: [], indexed: [] };
const orders = { baseline: [], indexed: [] };
for (let round = 0; round < rounds; round += 1) {
  for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) {
    setIndex(enabled);
    const result = sample();
    const name = enabled ? "indexed" : "baseline";
    orders[name].push(result.ids);
    samples[name].push({ execution_ms: result.execution_ms, nodes: result.nodes, result_count: result.ids.length });
  }
}
assert.equal(orders.baseline[0].length, 100);
for (const ids of [...orders.baseline, ...orders.indexed]) assert.deepEqual(ids, orders.baseline[0]);
setIndex(true);
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
const baseline = median(samples.baseline.map((sample) => sample.execution_ms));
const indexed = median(samples.indexed.map((sample) => sample.execution_ms));
const report = { rows, result_count: 100, index_bytes: Number(sql(`SELECT pg_relation_size('${index}');`)), baseline_p50_ms: baseline, indexed_p50_ms: indexed, reduction_percent: Number(((1 - indexed / baseline) * 100).toFixed(2)), result_order_preserved: true, samples };
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
