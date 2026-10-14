import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
assert.ok(process.env.A4N_TEST_DATABASE_URL, "run this benchmark through the isolated test database runner");
const databaseUrl = new URL(process.env.A4N_TEST_DATABASE_URL);
const reportDir = path.resolve(
  process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20261015/generated-image-index"),
);
const rowCount = Number(process.env.A4N_PERF_IMAGE_ROWS ?? 100_000);
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 5);
const table = "generated_image_index_probe";
const index = "idx_generated_image_index_probe_project";
const columns = `id, project_id, image_type, file_path, prompt, model, width, height,
  file_size, ai_score, is_favorite, chapter_id, panel_id, metadata_json, created_at`;
const query = `SELECT ${columns} FROM ${table} WHERE project_id = 42 ORDER BY created_at DESC`;

assert.equal(databaseUrl.hostname, "127.0.0.1", "run this benchmark through the isolated test database runner");
assert.ok(Number.isSafeInteger(rowCount) && rowCount >= 100_000 && rowCount % 100 === 0);
assert.ok(Number.isSafeInteger(rounds) && rounds >= 3);

const dockerRows = execFileSync("docker", ["ps", "--format", "{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim()
  .split("\n")
  .filter((line) => line.split("\t")[1]?.includes(`127.0.0.1:${databaseUrl.port}->5432/tcp`));
assert.equal(dockerRows.length, 1, "the test database container must be uniquely identified by its mapped port");
const container = dockerRows[0].split("\t")[0];

async function waitForDatabase() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      execFileSync("docker", [
        "exec", container, "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "a4n_test", "-d", "a4n_test",
      ], { stdio: "ignore" });
      return;
    } catch {
      await delay(100);
    }
  }
  throw new Error("isolated PostgreSQL TCP listener was not ready within 30 seconds");
}

function psql(sql) {
  return execFileSync(
    "docker",
    ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'],
    { input: sql, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  ).trim();
}

function percentile50(values) {
  const ordered = [...values].sort((left, right) => left - right);
  return ordered[Math.floor(ordered.length / 2)];
}

function nodeTypes(node, found = []) {
  found.push(node["Node Type"]);
  for (const child of node.Plans ?? []) nodeTypes(child, found);
  return found;
}

await waitForDatabase();
psql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id bigint PRIMARY KEY,
  project_id bigint NOT NULL,
  image_type text NOT NULL,
  file_path text NOT NULL,
  prompt text NOT NULL,
  model text NOT NULL,
  width integer NOT NULL,
  height integer NOT NULL,
  file_size bigint,
  ai_score real,
  is_favorite boolean NOT NULL,
  chapter_id bigint,
  panel_id text,
  metadata_json jsonb,
  created_at timestamp NOT NULL
);
INSERT INTO ${table} (${columns})
SELECT id,
  ((id - 1) % 100) + 1,
  'scene',
  '/media/images/' || id || '.webp',
  'Generated image prompt ' || id || repeat(' detail', 12),
  'fixture-model', 1024, 1024, 180000, 0.75,
  id % 20 = 0,
  ((id - 1) % 10) + 1,
  'panel-' || id,
  jsonb_build_object('payload', repeat('x', 256), 'image_id', id),
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second'
FROM generate_series(1, ${rowCount}) AS series(id);
ANALYZE ${table};
`);

function setIndex(enabled) {
  psql(enabled
    ? `CREATE INDEX IF NOT EXISTS ${index} ON ${table} (project_id); ANALYZE ${table};`
    : `DROP INDEX IF EXISTS ${index}; ANALYZE ${table};`);
}

function sample() {
  const plan = JSON.parse(psql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${query};`))[0];
  const ids = psql(`SELECT id FROM ${table} WHERE project_id = 42 ORDER BY created_at DESC;`)
    .split("\n")
    .map(Number);
  return {
    execution_ms: plan["Execution Time"],
    planning_ms: plan["Planning Time"],
    nodes: nodeTypes(plan.Plan),
    shared_hit_blocks: plan.Plan["Shared Hit Blocks"] ?? 0,
    shared_read_blocks: plan.Plan["Shared Read Blocks"] ?? 0,
    ids,
  };
}

const samples = { baseline: [], indexed: [] };
const resultOrders = { baseline: [], indexed: [] };
for (const enabled of [false, true]) {
  setIndex(enabled);
  sample();
}

for (let round = 0; round < rounds; round += 1) {
  const order = round % 2 === 0 ? [false, true] : [true, false];
  for (const enabled of order) {
    setIndex(enabled);
    const variant = enabled ? "indexed" : "baseline";
    const result = sample();
    resultOrders[variant].push(result.ids);
    const { ids, ...metrics } = result;
    samples[variant].push({
      ...metrics,
      result_count: ids.length,
      id_order_sha256: createHash("sha256").update(JSON.stringify(ids)).digest("hex"),
    });
  }
}

assert.equal(samples.baseline.length, rounds);
assert.equal(samples.indexed.length, rounds);
const expectedIds = resultOrders.baseline[0];
assert.equal(expectedIds.length, rowCount / 100);
for (const ids of [...resultOrders.baseline, ...resultOrders.indexed]) {
  assert.deepEqual(ids, expectedIds, "adding the index must preserve exact ID order");
}

setIndex(true);
const indexBytes = Number(psql(`SELECT pg_relation_size('${index}');`));
const baselineP50 = percentile50(samples.baseline.map((item) => item.execution_ms));
const indexedP50 = percentile50(samples.indexed.map((item) => item.execution_ms));
const report = {
  success: true,
  row_count: rowCount,
  rows_per_project: rowCount / 100,
  rounds,
  selected_project_id: 42,
  selected_count: expectedIds.length,
  index_definition: `${table}(project_id)`,
  index_bytes: indexBytes,
  baseline_p50_ms: baselineP50,
  indexed_p50_ms: indexedP50,
  reduction_percent: Number(((1 - indexedP50 / baselineP50) * 100).toFixed(2)),
  result_order_preserved: true,
  samples,
};
await mkdir(reportDir, { recursive: true });
await writeFile(path.join(reportDir, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify({
  baseline_p50_ms: baselineP50,
  indexed_p50_ms: indexedP50,
  reduction_percent: report.reduction_percent,
  index_bytes: indexBytes,
  result_count: expectedIds.length,
  result_order_preserved: true,
}, null, 2));
