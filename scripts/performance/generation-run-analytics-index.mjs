import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "generation_run_analytics_index_probe";
const index = "idx_generation_run_analytics_index_probe_created";
const rows = 100_000;
const rounds = 5;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/generation-run-analytics-index");
const containers = execFileSync("docker", ["ps", "--format", "{{.ID}}\t{{.Ports}}"], { encoding: "utf8" }).trim().split("\n").filter((line) => line.split("\t")[1]?.includes(`127.0.0.1:${url.port}->5432/tcp`));
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[0];
async function ready() { const deadline = Date.now() + 30_000; while (Date.now() < deadline) { try { execFileSync("docker", ["exec", container, "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "a4n_test", "-d", "a4n_test"], { stdio: "ignore" }); return; } catch { await delay(100); } } throw new Error("PostgreSQL did not become ready"); }
await ready();
function sql(value) { return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], { input: value, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 }).trim(); }
sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (id bigint PRIMARY KEY, project_id bigint NOT NULL, root_project_id bigint NOT NULL, workflow text NOT NULL, step_key text NOT NULL, title text NOT NULL, model_kind text NOT NULL, provider text, model text, status text NOT NULL, workflow_run_id bigint, call_key text, input_fingerprint_sha256 text, attempt integer NOT NULL, error_message text, created_at timestamp NOT NULL, completed_at timestamp);
INSERT INTO ${table}
SELECT id, id % 1000, 42, 'generation', 'step-' || (id % 50), 'Generation run ' || id,
  CASE WHEN id > 93000 AND id % 3 <> 0 THEN CASE WHEN id % 2 = 0 THEN 'image' ELSE 'video' END ELSE 'text' END,
  'provider', 'model', CASE WHEN id % 11 = 0 THEN 'failed' ELSE 'completed' END, id, 'call-' || id, md5(id::text), 1,
  CASE WHEN id % 11 = 0 THEN 'failure text' ELSE NULL END,
  CASE WHEN id > 93000 THEN TIMESTAMP '2026-10-15' - (100000 - id) * INTERVAL '1 minute' ELSE TIMESTAMP '2025-01-01' + id * INTERVAL '1 minute' END,
  TIMESTAMP '2026-10-15' - id * INTERVAL '1 second'
FROM generate_series(1, ${rows}) AS series(id);
CREATE INDEX idx_generation_run_probe_project_created ON ${table} (project_id, created_at DESC);
ANALYZE ${table};
`);
const statement = `SELECT id, project_id, root_project_id, workflow, step_key, title, model_kind, provider, model, status, workflow_run_id, call_key, input_fingerprint_sha256, attempt, error_message, created_at, completed_at FROM ${table} WHERE created_at >= TIMESTAMP '2026-10-08' AND model_kind IN ('image','video') ORDER BY created_at DESC`;
function setIndex(enabled) { sql(enabled ? `CREATE INDEX IF NOT EXISTS ${index} ON ${table} (created_at DESC); ANALYZE ${table};` : `DROP INDEX IF EXISTS ${index}; ANALYZE ${table};`); }
function sample() { const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0]; const ids = sql(`SELECT id FROM ${table} WHERE created_at >= TIMESTAMP '2026-10-08' AND model_kind IN ('image','video') ORDER BY created_at DESC;`).split("\n").map(Number); const nodes = []; const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); }; visit(plan.Plan); return { execution_ms: plan["Execution Time"], nodes, ids }; }
const samples = { baseline: [], indexed: [] }; const orders = { baseline: [], indexed: [] };
for (let round = 0; round < rounds; round += 1) for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) { setIndex(enabled); const result = sample(); const name = enabled ? "indexed" : "baseline"; orders[name].push(result.ids); samples[name].push({ execution_ms: result.execution_ms, nodes: result.nodes, result_count: result.ids.length }); }
assert.deepEqual(orders.baseline, orders.indexed); setIndex(true);
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]; const baseline = median(samples.baseline.map((sample) => sample.execution_ms)); const indexed = median(samples.indexed.map((sample) => sample.execution_ms));
const report = { rows, matching_rows: orders.baseline[0].length, index_bytes: Number(sql(`SELECT pg_relation_size('${index}');`)), baseline_p50_ms: baseline, indexed_p50_ms: indexed, reduction_percent: Number(((1 - indexed / baseline) * 100).toFixed(2)), result_order_preserved: true, samples }; await mkdir(artifacts, { recursive: true }); await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2)); console.log(JSON.stringify(report, null, 2));
