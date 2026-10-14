import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_run_business_index_probe";
const index = "idx_workflow_run_business_updated_probe";
const rows = 100_000;
const rounds = 5;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/business-key-index");
const containers = execFileSync("docker", ["ps", "--format", "{{.ID}}\t{{.Ports}}"], { encoding: "utf8" }).trim().split("\n").filter((line) => line.split("\t")[1]?.includes(`127.0.0.1:${url.port}->5432/tcp`));
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[0];
async function ready() { const deadline = Date.now() + 30_000; while (Date.now() < deadline) { try { execFileSync("docker", ["exec", container, "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "a4n_test", "-d", "a4n_test"], { stdio: "ignore" }); return; } catch { await delay(100); } } throw new Error("PostgreSQL did not become ready"); }
await ready();
function sql(value) { return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], { input: value, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 }).trim(); }
sql(`DROP TABLE IF EXISTS ${table}; CREATE TABLE ${table} (id bigint PRIMARY KEY, business_key text NOT NULL, workflow_kind text NOT NULL, project_id bigint, status text NOT NULL, progress_json jsonb, active_task_id text, can_resume boolean NOT NULL, error_message text, created_at timestamp NOT NULL, updated_at timestamp NOT NULL); INSERT INTO ${table} SELECT id, 'key-' || ((id - 1) % 100), 'video', 42, CASE WHEN id % 5 = 0 THEN 'completed' ELSE 'running' END, jsonb_build_object('step', id % 30, 'payload', repeat('x', 256)), 'task-' || id, id % 3 = 0, NULL, TIMESTAMP '2026-01-01' + id * INTERVAL '1 second', TIMESTAMP '2026-01-01' + id * INTERVAL '1 second' FROM generate_series(1, ${rows}) AS series(id); CREATE INDEX ${index}_existing ON ${table} (business_key); ANALYZE ${table};`);
const statement = `SELECT * FROM ${table} WHERE business_key = 'key-42' ORDER BY updated_at DESC, id DESC LIMIT 1`;
function setIndex(enabled) { sql(enabled ? `CREATE INDEX IF NOT EXISTS ${index} ON ${table} (business_key, updated_at DESC, id DESC); ANALYZE ${table};` : `DROP INDEX IF EXISTS ${index}; ANALYZE ${table};`); }
function sample() { const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0]; const id = Number(sql(`SELECT id FROM (${statement}) AS latest;`)); const nodes = []; const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); }; visit(plan.Plan); return { execution_ms: plan["Execution Time"], nodes, id }; }
const samples = { baseline: [], indexed: [] }; const ids = { baseline: [], indexed: [] };
for (let round = 0; round < rounds; round += 1) for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) { setIndex(enabled); const result = sample(); const name = enabled ? "indexed" : "baseline"; ids[name].push(result.id); samples[name].push({ execution_ms: result.execution_ms, nodes: result.nodes }); }
assert.deepEqual(ids.baseline, ids.indexed);
setIndex(true);
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]; const baseline = median(samples.baseline.map((sample) => sample.execution_ms)); const indexed = median(samples.indexed.map((sample) => sample.execution_ms));
const report = { rows, history_per_key: 1000, index_bytes: Number(sql(`SELECT pg_relation_size('${index}');`)), baseline_p50_ms: baseline, indexed_p50_ms: indexed, reduction_percent: Number(((1 - indexed / baseline) * 100).toFixed(2)), result_identity_preserved: true, samples }; await mkdir(artifacts, { recursive: true }); await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2)); console.log(JSON.stringify(report, null, 2));
