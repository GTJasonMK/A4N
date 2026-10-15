import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "llm_call_trace_index_probe";
const index = "idx_llm_call_trace_index_probe_created";
const rows = 100_000;
const rounds = 5;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/llm-analytics-index");
const containers = execFileSync("docker", ["ps", "--format", "{{.ID}}\t{{.Ports}}"], { encoding: "utf8" }).trim().split("\n").filter((line) => line.split("\t")[1]?.includes(`127.0.0.1:${url.port}->5432/tcp`));
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[0];
async function ready() { const deadline = Date.now() + 30_000; while (Date.now() < deadline) { try { execFileSync("docker", ["exec", container, "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "a4n_test", "-d", "a4n_test"], { stdio: "ignore" }); return; } catch { await delay(100); } } throw new Error("PostgreSQL did not become ready"); }
await ready();
function sql(value) { return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], { input: value, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 }).trim(); }
sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id bigint PRIMARY KEY, generation_run_id bigint, workflow_run_id bigint, call_key text, input_fingerprint_sha256 text,
  project_id bigint, root_project_id bigint, workflow text NOT NULL, step_key text NOT NULL, provider text, model text,
  context_plan_json json, context_pack_refs_json json, prompt_tokens integer, completion_tokens integer, total_tokens integer,
  prompt_cache_hit_tokens integer, prompt_cache_miss_tokens integer, prompt_cache_write_tokens integer, prompt_cache_hit_ratio double precision,
  duration_ms bigint, streamed boolean NOT NULL, status text NOT NULL, error_kind text, error_message text, finish_reason text,
  created_at timestamp NOT NULL
);
INSERT INTO ${table}
SELECT id, id, id, 'call-' || id, md5(id::text),
  CASE WHEN id % 10 = 0 THEN 42 ELSE (id % 100) + 1 END, 42,
  'chapter_analysis', 'analyze', 'provider', 'model',
  json_build_object('stable_prefix_tokens', 800, 'dynamic_delta_tokens', 400, 'estimated_input_tokens', 1200, 'dropped_optional_pack_refs', json_build_array(1,2,3), 'unused_payload', repeat('x', 1024)),
  json_build_array(repeat('y', 128)), 800, 200, 1000, 300, 500, 0, 0.3, 1280, true,
  CASE WHEN id % 17 = 0 THEN 'failed' ELSE 'completed' END, NULL, NULL, 'stop',
  CASE WHEN id > 93000 THEN TIMESTAMP '2026-10-15' - (100000 - id) * INTERVAL '1 minute' ELSE TIMESTAMP '2025-01-01' + id * INTERVAL '1 minute' END
FROM generate_series(1, ${rows}) AS series(id);
ANALYZE ${table};
`);
const statement = `SELECT id, generation_run_id, workflow_run_id, call_key, input_fingerprint_sha256, project_id, root_project_id, workflow, step_key, provider, model,
CASE WHEN context_plan_json IS NULL THEN NULL ELSE json_build_object('stable_prefix_tokens', context_plan_json -> 'stable_prefix_tokens', 'dynamic_delta_tokens', context_plan_json -> 'dynamic_delta_tokens', 'estimated_input_tokens', context_plan_json -> 'estimated_input_tokens', 'dropped_optional_pack_count', CASE WHEN json_typeof(context_plan_json -> 'dropped_optional_pack_refs') = 'array' THEN json_array_length(context_plan_json -> 'dropped_optional_pack_refs') ELSE 0 END) END AS context_plan_json,
prompt_tokens, completion_tokens, total_tokens, prompt_cache_hit_tokens, prompt_cache_miss_tokens, prompt_cache_write_tokens, prompt_cache_hit_ratio, duration_ms, streamed, status, error_kind, error_message, finish_reason, created_at
FROM ${table} WHERE created_at >= TIMESTAMP '2026-10-08' ORDER BY created_at DESC`;
function setIndex(enabled) { sql(enabled ? `CREATE INDEX IF NOT EXISTS ${index} ON ${table} (created_at DESC); ANALYZE ${table};` : `DROP INDEX IF EXISTS ${index}; ANALYZE ${table};`); }
function sample() { const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0]; const ids = sql(`SELECT id FROM ${table} WHERE created_at >= TIMESTAMP '2026-10-08' ORDER BY created_at DESC;`).split("\n").map(Number); const nodes = []; const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); }; visit(plan.Plan); return { execution_ms: plan["Execution Time"], planning_ms: plan["Planning Time"], rows_returned: plan.Plan["Actual Rows"], shared_hit_blocks: plan.Plan["Shared Hit Blocks"] ?? 0, shared_read_blocks: plan.Plan["Shared Read Blocks"] ?? 0, nodes, ids }; }
const samples = { baseline: [], indexed: [] }; const orders = { baseline: [], indexed: [] };
for (let round = 0; round < rounds; round += 1) for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) { setIndex(enabled); const result = sample(); const name = enabled ? "indexed" : "baseline"; orders[name].push(result.ids); const { ids, ...metrics } = result; samples[name].push(metrics); }
assert.deepEqual(orders.baseline, orders.indexed);
setIndex(true);
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]; const baseline = median(samples.baseline.map((sample) => sample.execution_ms)); const indexed = median(samples.indexed.map((sample) => sample.execution_ms));
const report = { rows, matching_rows: orders.baseline[0].length, index_bytes: Number(sql(`SELECT pg_relation_size('${index}');`)), baseline_p50_ms: baseline, indexed_p50_ms: indexed, reduction_percent: Number(((1 - indexed / baseline) * 100).toFixed(2)), result_order_preserved: true, samples }; await mkdir(artifacts, { recursive: true }); await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2)); console.log(JSON.stringify(report, null, 2));
